//! Performs a KNN vector search against a managed Valkey cluster by
//! converting a text query to an embedding via OpenAI (caching the
//! embedding bytes in Valkey under the query's SHA-256 hash). Pass the
//! cluster name via `CLUSTER_NAME` when deploying.

use std::{collections::HashMap, mem::take};

use momento_functions_bytes::encoding::{Extract, Json};
use momento_functions_guest_web::{WebEnvironment, WebError, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_http::{Request as HttpRequest, invoke as http_invoke};
use momento_functions_valkey::{ClusterClient, Command, Value, get_managed_cluster_client};
use serde::{Deserialize, Serialize};
use sha2::Digest;

#[derive(Deserialize, Debug)]
struct Request {
    query: String,
    topk: Option<usize>,
}

#[derive(Serialize, Debug)]
struct Document {
    id: String,
    __vector_score: f32,
    #[serde(flatten)]
    fields: HashMap<String, String>,
}

invoke!(search);
fn search(Json(body): Json<Request>) -> WebResult<WebResponse> {
    setup_logging()?;

    let Request { query, topk } = body;
    let topk = topk.unwrap_or(5);
    log::debug!("getting top {topk} documents for search: {query:?}");

    let mut hasher = sha2::Sha256::new();
    hasher.update(query.as_bytes());
    let query_hash: [u8; 32] = hasher.finalize().into();

    let cluster_name = std::env::var("CLUSTER_NAME").unwrap_or_default();
    let client = get_managed_cluster_client(&cluster_name);
    let query_embedding = get_cached_query_embedding(query, query_hash, &client)?;

    let mut cmd = Command::builder("FT.SEARCH");
    cmd.argument("document_index")
        .argument(format!("*=>[KNN {topk} @vector $query_vector]"))
        .argument("PARAMS")
        .argument("2")
        .argument("query_vector")
        .argument(query_embedding);
    let response = client.command(cmd)?;

    let mut responses = match response {
        Value::Bulk(items) => items,
        Value::SimpleError(e) => {
            log::error!("Valkey error: {e}");
            return Err(WebError::message(format!("Valkey error: {e}")));
        }
        _ => return Err(WebError::message("Unexpected Valkey response")),
    };

    let topk_actual = responses
        .next()
        .ok_or_else(|| WebError::message("No results returned from Valkey"))?;
    let topk_actual = match topk_actual {
        Value::Int(count) => count as usize,
        _ => {
            return Err(WebError::message(
                "Expected an integer for the number of results",
            ));
        }
    };

    let mut documents: Vec<Document> = Vec::with_capacity(topk_actual);
    loop {
        log::debug!("reading next document");
        let value = match responses.next() {
            Some(value) => value,
            None => break,
        };

        let mut parser;
        match value {
            Value::Data(_) => {
                parser = ParserExpect::DocumentId;
                parser.try_parse(value)?;
            }
            Value::SimpleError(e) => {
                log::error!("Valkey error: {e}");
                return Err(WebError::message(format!("Valkey error: {e}")));
            }
            _ => return Err(WebError::message("Unexpected Valkey response")),
        }

        let value = responses
            .next()
            .ok_or_else(|| WebError::message("Unexpected end of response after document ID"))?;
        match value {
            Value::Bulk(items) => {
                let mut skip = 0;
                for item in items {
                    if skip != 0 {
                        skip -= 1;
                        continue;
                    }
                    skip = parser.try_parse(item)?;
                }
            }
            Value::SimpleError(e) => {
                log::error!("Valkey error: {e}");
                return Err(WebError::message(format!("Valkey error: {e}")));
            }
            _ => return Err(WebError::message("Unexpected Valkey response")),
        }
        match parser {
            ParserExpect::FieldName {
                id,
                vector_score,
                fields,
            } => documents.push(Document {
                id,
                __vector_score: vector_score,
                fields,
            }),
            _ => return Err(WebError::message("Unexpected parser terminal state")),
        }
    }

    Ok(WebResponse::new()
        .with_status(200)
        .with_headers(vec![(
            "content-type".to_string(),
            "application/json".to_string(),
        )])
        .with_body(Json(documents))?)
}

#[derive(Debug)]
enum ParserExpect {
    DocumentId,
    VectorScore {
        id: String,
    },
    FieldName {
        id: String,
        vector_score: f32,
        fields: HashMap<String, String>,
    },
    FieldValue {
        id: String,
        vector_score: f32,
        fields: HashMap<String, String>,
        name: String,
    },
}

impl ParserExpect {
    fn try_parse(&mut self, value: Value) -> WebResult<usize> {
        match self {
            ParserExpect::DocumentId => {
                let bytes = expect_data(value, "document ID")?;
                *self = ParserExpect::VectorScore {
                    id: String::from_utf8(bytes).map_err(|e| {
                        WebError::message(format!("Failed to parse document ID: {e}"))
                    })?,
                };
                Ok(0)
            }
            ParserExpect::VectorScore { id } => {
                let bytes = expect_data(value, "vector score")?;
                let raw = String::from_utf8(bytes).map_err(|e| {
                    WebError::message(format!("failed to parse vector score as utf8: {e:?}"))
                })?;
                if raw == "__vector_score" {
                    return Ok(0);
                }
                let vector_score = raw.parse().map_err(|e| {
                    WebError::message(format!("Failed to parse vector score: {e:?}"))
                })?;
                *self = ParserExpect::FieldName {
                    id: take(id),
                    vector_score,
                    fields: HashMap::new(),
                };
                Ok(0)
            }
            ParserExpect::FieldName {
                id,
                vector_score,
                fields,
            } => {
                let bytes = expect_data(value, "field name")?;
                let name = String::from_utf8(bytes)
                    .map_err(|e| WebError::message(format!("Failed to parse field name: {e}")))?;
                if name == "vector" {
                    return Ok(1);
                }
                *self = ParserExpect::FieldValue {
                    id: take(id),
                    vector_score: *vector_score,
                    fields: take(fields),
                    name,
                };
                Ok(0)
            }
            ParserExpect::FieldValue {
                id,
                vector_score,
                fields,
                name,
            } => {
                let bytes = expect_data(value, "field value")?;
                let field_value = String::from_utf8(bytes)
                    .map_err(|e| WebError::message(format!("Failed to parse field value: {e}")))?;
                fields.insert(name.clone(), field_value);
                *self = ParserExpect::FieldName {
                    id: take(id),
                    vector_score: *vector_score,
                    fields: take(fields),
                };
                Ok(0)
            }
        }
    }
}

fn expect_data(value: Value, what: &str) -> WebResult<Vec<u8>> {
    match value {
        Value::Data(data) => Ok(data.into_bytes()),
        _ => Err(WebError::message(format!("Expected Data type for {what}"))),
    }
}

fn get_cached_query_embedding(
    query: String,
    query_hash: [u8; 32],
    client: &ClusterClient,
) -> WebResult<Vec<u8>> {
    match client.command(Command::get(query_hash.to_vec()))? {
        Value::Data(data) => {
            log::debug!("cache hit for query embedding");
            Ok(data.into_bytes())
        }
        Value::Nil => {
            log::debug!("cache miss, querying embeddings from OpenAI");
            let embedding = get_embedding(&query)?;
            let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
            client.command(Command::set(query_hash.to_vec(), bytes.clone()))?;
            Ok(bytes)
        }
        _ => {
            log::warn!("unexpected valkey GET response, treating as miss");
            let embedding = get_embedding(&query)?;
            Ok(embedding.iter().flat_map(|f| f.to_le_bytes()).collect())
        }
    }
}

#[derive(Deserialize, Debug)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}
#[derive(Deserialize, Serialize, Debug)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

fn get_embedding(query: &str) -> WebResult<Vec<f32>> {
    let query = query.replace('\n', " ");
    let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let response = http_invoke(
        HttpRequest::new("https://api.openai.com/v1/embeddings", "POST")
            .with_header("authorization", format!("Bearer {openai_api_key}"))
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "model": "text-embedding-3-small",
                    "encoding_format": "float",
                    "input": [query],
                })
                .to_string(),
            ),
    )?;
    let Json(EmbeddingResponse { mut data }) = Json::<EmbeddingResponse>::extract(response.body)?;
    data.sort_by_key(|d| d.index);
    data.into_iter()
        .next()
        .map(|d| d.embedding)
        .ok_or_else(|| WebError::message("Failed to get embedding for query"))
}

fn setup_logging() -> WebResult<()> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;
    Ok(())
}
