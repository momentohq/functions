use log::LevelFilter;
use momento_functions::{WebError, WebResponse, WebResult};
use momento_functions_host::{
    encoding::Json,
    logging::{ConfigureLoggingInput, LogDestination},
    redis::{Command, RedisClient, RedisValue},
    web_extensions::headers,
};

use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{collections::HashMap, mem::take};

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

momento_functions::post!(index_document);
fn index_document(Json(body): Json<Request>) -> WebResult<WebResponse> {
    let headers = headers();
    setup_logging(&headers)?;

    let Request { query, topk } = body;
    let topk = topk.unwrap_or(5);
    log::debug!("getting top {topk} documents for search: {query:?}");
    let mut hash = sha2::Sha256::new();
    hash.update(query.as_bytes());
    let query_hash: [u8; 32] = hash.finalize().into();

    const CONNECTION_STRING: &str = env!("REDIS_CONNECTION_STRING");
    let redis = RedisClient::new(CONNECTION_STRING);
    let query_embedding = get_cached_query_embedding(query, query_hash, &redis)?;

    let response = redis.pipe(vec![
        Command::builder()
            .any("FT.SEARCH")
            .arg("document_index")
            .arg(format!("*=>[KNN {topk} @vector $query_vector]"))
            .arg("PARAMS")
            .arg("2")
            .arg("query_vector")
            .arg(query_embedding)
            .build(),
    ])?;

    let responses: RedisValue = response
        .into_iter()
        .next()
        .ok_or_else(|| WebError::message("No response from Redis"))?;
    let mut responses = match responses {
        RedisValue::Bulk(items) => items.into_iter(),
        RedisValue::Status(e) => {
            log::error!("Redis error: {e}");
            return Err(WebError::message(format!("Redis error: {e}")));
        }
        other => {
            log::error!("Unexpected Redis response: {other:?}");
            return Err(WebError::message("Unexpected Redis response"));
        }
    };
    let topk_actual = responses
        .next()
        .ok_or_else(|| WebError::message("No results returned from Redis"))?;
    let topk_actual = match topk_actual {
        RedisValue::Int(count) => count as usize,
        other => {
            log::error!("Expected an integer for the number of results, got: {other:?}");
            return Err(WebError::message(
                "Expected an integer for the number of results",
            ));
        }
    };

    let mut documents: Vec<Document> = Vec::with_capacity(topk_actual);
    loop {
        log::debug!("reading next document");
        let mut search_result_parser;
        let value = match responses.next() {
            Some(value) => value,
            None => break,
        };

        match &value {
            RedisValue::Data(_) => {
                search_result_parser = FtSearchParserExpect::DocumentId;
                search_result_parser.try_parse(value)?;
            }
            RedisValue::Status(e) => {
                log::error!("Redis error: {e}");
                return Err(WebError::message(format!("Redis error: {e}")));
            }
            other => {
                log::error!("Unexpected Redis response: {other:?}");
                return Err(WebError::message("Unexpected Redis response"));
            }
        }

        let value = match responses.next() {
            Some(value) => value,
            None => {
                log::error!("Unexpected end of response after document ID");
                return Err(WebError::message(
                    "Unexpected end of response after document ID",
                ));
            }
        };
        match value {
            RedisValue::Bulk(items) => {
                let mut skip = 0;
                for item in items {
                    if skip != 0 {
                        skip -= 1;
                        continue;
                    }
                    skip = search_result_parser.try_parse(item)?;
                }
            }
            RedisValue::Status(e) => {
                log::error!("Redis error: {e}");
                return Err(WebError::message(format!("Redis error: {e}")));
            }
            other => {
                log::error!("Unexpected Redis response: {other:?}");
                return Err(WebError::message("Unexpected Redis response"));
            }
        }
        match search_result_parser {
            FtSearchParserExpect::FieldName {
                id,
                vector_score,
                fields,
            } => {
                documents.push(Document {
                    id,
                    __vector_score: vector_score,
                    fields,
                });
            }
            other => {
                log::error!("Unexpected terminal parser state: {other:?}");
                return Err(WebError::message("Unexpected parser state".to_string()));
            }
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
enum FtSearchParserExpect {
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
impl FtSearchParserExpect {
    fn try_parse(&mut self, value: RedisValue) -> WebResult<usize> {
        match self {
            FtSearchParserExpect::DocumentId => {
                if let RedisValue::Data(data) = value {
                    *self = FtSearchParserExpect::VectorScore {
                        id: String::from_utf8(data).map_err(|e| {
                            WebError::message(format!("Failed to parse document ID: {e}"))
                        })?,
                    };
                    log::debug!("parsed document ID");
                    Ok(0)
                } else {
                    log::error!("Expected Data type for document ID, got: {value:?}");
                    Err(WebError::message("Expected Data type for document ID"))
                }
            }
            FtSearchParserExpect::VectorScore { id } => {
                if let RedisValue::Data(data) = value {
                    let vector_score = String::from_utf8(data).map_err(|e| {
                        WebError::message(format!("failed to parse vector score as utf8: {e:?}"))
                    })?;
                    if vector_score == "__vector_score" {
                        log::debug!("found vector score field");
                        return Ok(0);
                    }
                    let vector_score = vector_score.parse().map_err(|e| {
                        WebError::message(format!("Failed to parse vector score: {e:?}"))
                    })?;
                    *self = FtSearchParserExpect::FieldName {
                        id: take(id),
                        vector_score,
                        fields: HashMap::new(),
                    };
                    log::debug!("parsed vector score");
                    Ok(0)
                } else {
                    log::error!("Expected Data type for vector score, got: {value:?}");
                    Err(WebError::message("Expected Data type for vector score"))
                }
            }
            FtSearchParserExpect::FieldName {
                id,
                vector_score,
                fields,
            } => {
                if let RedisValue::Data(data) = value {
                    let field_name = String::from_utf8(data).map_err(|e| {
                        WebError::message(format!("Failed to parse field name: {e}"))
                    })?;
                    if field_name == "vector" {
                        log::debug!("found vector field");
                        return Ok(1);
                    }
                    log::debug!("parsed field name {field_name}");
                    *self = FtSearchParserExpect::FieldValue {
                        id: take(id),
                        vector_score: *vector_score,
                        fields: take(fields),
                        name: field_name,
                    };
                    Ok(0)
                } else {
                    log::error!("Expected Data type for field name, got: {value:?}");
                    Err(WebError::message("Expected Data type for field name"))
                }
            }
            FtSearchParserExpect::FieldValue {
                id,
                vector_score,
                fields,
                name,
            } => {
                if let RedisValue::Data(data) = value {
                    let field_value = String::from_utf8(data).map_err(|e| {
                        WebError::message(format!("Failed to parse field value: {e}"))
                    })?;
                    fields.insert(name.clone(), field_value);
                    *self = FtSearchParserExpect::FieldName {
                        id: take(id),
                        vector_score: *vector_score,
                        fields: take(fields),
                    };
                    log::debug!("parsed field value");
                    Ok(0)
                } else {
                    log::error!("Expected Data type for field value, got: {value:?}");
                    Err(WebError::message("Expected Data type for field value"))
                }
            }
        }
    }
}

fn get_cached_query_embedding(
    query: String,
    query_hash: [u8; 32],
    redis: &RedisClient,
) -> WebResult<Vec<u8>> {
    Ok(match redis.get::<Vec<u8>>(&query_hash)? {
        Some(hit) => hit,
        None => match get_embeddings(vec![query.clone()])?.into_iter().next() {
            Some(embedding) => {
                let new_query_embedding = embedding
                    .embedding
                    .into_iter()
                    .flat_map(f32::to_le_bytes)
                    .collect::<Vec<u8>>();
                redis.set(query_hash, new_query_embedding.clone())?;
                new_query_embedding
            }
            None => {
                log::error!("Failed to get embedding for query: {query}");
                return Err(WebError::message("Failed to get embedding for query"));
            }
        },
    })
}

// ------------------------------------------------------
// | Utility functions for convenience
// ------------------------------------------------------

fn setup_logging(headers: &[(String, String)]) -> WebResult<()> {
    let log_level = headers.iter().find_map(|(name, value)| {
        if name == "x-momento-log" {
            Some(value)
        } else {
            None
        }
    });
    if let Some(log_level) = log_level {
        let log_level = log_level
            .parse::<LevelFilter>()
            .unwrap_or(LevelFilter::Info);
        momento_functions_log::configure_logging(
            log_level,
            vec![ConfigureLoggingInput::new(LogDestination::Topic {
                topic: "valkey-vector-search".to_string(),
            })],
        )?;
    }
    Ok(())
}

#[derive(Deserialize, Serialize, Debug)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}
fn get_embeddings(mut documents: Vec<String>) -> WebResult<Vec<EmbeddingData>> {
    log::debug!("getting embeddings for document with content: {documents:?}");
    for document in &mut documents {
        if document.contains("\n") {
            // openai guide currently says to replace newlines with spaces. This, then, must be how you get the cargo to come.
            // https://platform.openai.com/docs/guides/embeddings
            *document = document.replace("\n", " ");
        }
    }
    // compile-time environment variable.
    // Runtime environment variable secrets will be supported in the future.
    const OPENAI_API_KEY: &str = env!("OPENAI_API_KEY");
    let result = momento_functions_host::http::post(
        "https://api.openai.com/v1/embeddings",
        [
            (
                "authorization".to_string(),
                format!("Bearer {OPENAI_API_KEY}"),
            ),
            ("content-type".to_string(), "application/json".to_string()),
        ],
        // 1536 float32 for text-embedding-3-small
        serde_json::json!({
            "model": "text-embedding-3-small",
            "encoding_format": "float",
            "input": documents,
        })
        .to_string(),
    );
    log::debug!("OpenAI response: {result:?}");
    let mut response = result?;

    #[derive(Deserialize, Debug)]
    struct GetEmbeddingResponse {
        data: Vec<EmbeddingData>,
    }
    let Json(GetEmbeddingResponse { mut data }) = response.extract()?;
    data.sort_by_key(|d| d.index);
    Ok(data)
}
