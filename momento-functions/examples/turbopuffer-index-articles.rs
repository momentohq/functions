//! Using input data from CBS Sports articles, this function will
//! query OpenAI to generate embeddings for each document, then index it
//! within Turbopuffer so we can search through it. To make things even faster,
//! we'll use Momento in-host caching for the embedding queries.
//!
//! You need to provide `OPENAI_KEY`, `TURBOPUFFER_ENDPOINT`, and `TURBOPUFFER_API_KEY`
//! environment variables when creating this function:
//! * `OPENAI_KEY`              -> The API key for accessing OpenAI, shoud just be the key itself.
//! * `TURBOPUFFER_ENDPOINT`    -> The endpoint contains the namespace.
//! * `TURBOPUFFER_API_KEY`     -> The API key should just be the key itself.
//!
//! You can also provide the `TTL_SECONDS` environment variable to override the default
//! ttl used to store embeddings in your Momento cache.
//!
//! To demo this, you can pipe a subset of the data and feed it into a cURL command
//! to invoke your function:
//! ```bash
//! # Export your environment variables
//! export MOMENTO_CELL_HOSTNAME=<momento cell>
//! export MOMENTO_CACHE_NAME=my-functions-cache
//! export MOMENTO_API_KEY=<your api key>
//!
//! export OPENAI_KEY=<openai api key>
//! export TURBOPUFFER_ENDPOINT=<Should be v2 namespace>
//! export TURBOPUFFER_API_KEY=<turbopuffer api key>
//!
//! # Create your Momento cache
//! momento cache create $MOMENTO_CACHE_NAME
//!
//! # Create your Momento function
//! momento preview function put-function \
//!   --cache-name "$MOMENTO_CACHE_NAME" \
//!   --name turbopuffer-index-articles \
//!   --wasm-file /path/to/this/compiled/turbopuffer_index_articles.wasm \
//!   -E OPENAI_KEY="$OPENAI_KEY" \
//!   -E TURBOPUFFER_ENDPOINT="$TURBOPUFFER_ENDPOINT" \
//!   -E TURBOPUFFER_API_KEY="$TURBOPUFFER_API_KEY"
//!
//! # Send subset of articles for indexing via our uploaded function
//! jq 'articles.nba' /path/to/your/data.json | curl \
//!   curl https://api.cache.$MOMENTO_CELL_HOSTNAME/functions/$MOMENTO_CACHE_NAME/turbopuffer-index-articles \
//!   -XPOST \
//!   -H "authorization: $MOMENTO_API_KEY" \
//!   -H "Content-Type: application/json" \
//!   -d @-   
//! ```

use std::time::Duration;

use itertools::Itertools;
use log::LevelFilter;
use momento_functions::{WebError, WebResponse, WebResult};
use momento_functions_host::{cache, encoding::Json, web_extensions::headers};
use momento_functions_log::LogMode;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Debug)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}
#[derive(Deserialize, Serialize, Debug)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize, Serialize, Debug)]
struct QueryRow {
    #[serde(alias = "$dist")]
    dist: f32,
    id: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct DocumentMetadata {
    title: String,
    link: String,
    authors: Vec<String>,
    language: String,
    description: String,
    feed: String,
}

#[derive(Deserialize, Debug)]
struct DocumentInput {
    id: String,
    #[serde(alias = "metadata")]
    document_metadata: DocumentMetadata,
    page_content: String,
}

impl DocumentInput {
    pub fn into_turbopuffer_document(self, vector: Vec<f32>) -> TurbopufferDocument {
        TurbopufferDocument {
            id: self.id,
            page_content: self.page_content,
            vector,
            // Flatten out the dimensions
            metadata_title: self.document_metadata.title,
            metadata_link: self.document_metadata.link,
            metadata_authors: self.document_metadata.authors,
            metadata_language: self.document_metadata.language,
            metadata_description: self.document_metadata.description,
            metadata_feed: self.document_metadata.feed,
        }
    }
}

#[derive(Serialize, Debug)]
struct TurbopufferDocument {
    id: String,
    page_content: String,
    vector: Vec<f32>,
    // Flatten out the dimensions for easy query
    #[serde(rename = "metadata$title")]
    metadata_title: String,
    #[serde(rename = "metadata$link")]
    metadata_link: String,
    #[serde(rename = "metadata$authors")]
    metadata_authors: Vec<String>,
    #[serde(rename = "metadata$language")]
    metadata_language: String,
    #[serde(rename = "metadata$description")]
    metadata_description: String,
    #[serde(rename = "metadata$feed")]
    metadata_feed: String,
}

const DEFAULT_TTL_SECONDS: u64 = 30;

momento_functions::post!(index_document);
fn index_document(Json(documents): Json<Vec<DocumentInput>>) -> WebResult<WebResponse> {
    let headers = headers();
    setup_logging(&headers)?;

    if documents.is_empty() {
        log::warn!("No documents provided for indexing.");
        return Ok(WebResponse::new()
            .with_status(400)
            .with_body("No documents provided")?);
    }

    // These are passed in as environment variables when creating the function
    let turbopuffer_api_key = format!(
        "Bearer {}",
        std::env::var("TURBOPUFFER_API_KEY").unwrap_or_default()
    );
    let turbopuffer_endpoint = std::env::var("TURBOPUFFER_ENDPOINT").unwrap_or_default();
    let openapi_key = std::env::var("OPENAI_KEY").unwrap_or_default();

    // Not required, but can be overridden
    let ttl_seconds = std::env::var("TTL_SECONDS")
        .unwrap_or_default()
        .parse::<u64>()
        .unwrap_or(DEFAULT_TTL_SECONDS);

    let chunks = documents.into_iter().chunks(2000);
    for chunk in chunks.into_iter() {
        let chunk: Vec<DocumentInput> = chunk.collect();
        // Queries OpenAI to generate an embedding for this document so we can ship it off to Turbopuffer
        let mut turbopuffer_inputs = Vec::new();
        for document in chunk {
            log::debug!("getting embedding for id {}", document.id);
            let embedding = get_cached_query_embedding(
                document.page_content.clone(),
                ttl_seconds,
                openapi_key.clone(),
            )?;
            turbopuffer_inputs.push(document.into_turbopuffer_document(embedding));
        }
        log::debug!("sending to turbopuffer: {}", json!(turbopuffer_inputs));
        // Send off our transformed data to Turbopuffer, complete with embeddings
        let result = momento_functions_host::http::post(
            turbopuffer_endpoint.clone(),
            [
                ("Authorization".to_string(), turbopuffer_api_key.clone()),
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Accept".to_string(), "*/*".to_string()),
                (
                    "User-Agent".to_string(),
                    "momento-turbobuffer-example".to_string(),
                ),
            ],
            json!({
                "upsert_rows": turbopuffer_inputs,
                "distance_metric": "cosine_distance",
            }),
        );
        match result {
            Ok(response) => {
                log::debug!("turbopuffer response: {:?}", response.headers);
                if response.status != 200 {
                    let message = format!(
                        "Failed to index documents: {}",
                        String::from_utf8(response.body).unwrap_or_default(),
                    );
                    return Ok(WebResponse::new().with_status(response.status).with_body(
                        json!({
                            "message": message,
                        }),
                    )?);
                }
            }
            Err(e) => {
                log::error!("Failed to index documents: {e:?}");
                return Ok(WebResponse::new().with_status(500).with_body(json!({
                    "message": e.to_string(),
                }))?);
            }
        }
    }
    Ok(WebResponse::new().with_status(200).with_body(json!({
        "message": "Documents indexed successfully",
    }))?)
}

// ------------------------------------------------------
// | Utility functions for convenience
// ------------------------------------------------------

fn get_cached_query_embedding(
    query: String,
    ttl_seconds: u64,
    openai_key: String,
) -> WebResult<Vec<f32>> {
    log::debug!("Checking if embeddings are already cached for input");
    Ok(match cache::get::<Vec<u8>>(query.clone())? {
        Some(hit) => {
            log::debug!("cache hit");
            // Convert raw bytes back into our Vec<f32> type
            hit.chunks_exact(4)
                .map(|chunk| {
                    let arr = <[u8; 4]>::try_from(chunk)
                        .map_err(|_| WebError::message("Chunk length should be 4"))?;
                    Ok(f32::from_le_bytes(arr))
                })
                .collect::<Result<Vec<f32>, WebError>>()?
        }
        None => {
            log::debug!("cache miss, querying embeddings from open ai");
            match get_embeddings(query.clone(), openai_key)?
                .into_iter()
                .next()
            {
                Some(embedding) => {
                    // Convert to raw bytes before storing in cache
                    let new_query_embedding = embedding
                        .embedding
                        .clone()
                        .into_iter()
                        .flat_map(f32::to_le_bytes)
                        .collect::<Vec<u8>>();
                    log::debug!("setting embeddings in cache");
                    cache::set(
                        query,
                        new_query_embedding.clone(),
                        Duration::from_secs(ttl_seconds),
                    )?;
                    embedding.embedding
                }
                None => {
                    log::error!("Failed to get embedding for query: {query}");
                    return Err(WebError::message("Failed to get embedding for query"));
                }
            }
        }
    })
}

fn get_embeddings(query: String, openai_key: String) -> WebResult<Vec<EmbeddingData>> {
    log::debug!("getting embeddings for input");
    let query = if query.contains("\n") {
        // openai guide currently says to replace newlines with spaces. This, then, must be how you get the cargo to come.
        // https://platform.openai.com/docs/guides/embeddings
        query.replace("\n", " ")
    } else {
        query
    };

    // openai will fail to generate an embedding if no content is provided
    let query = if query.is_empty() {
        "no content".to_string()
    } else {
        query
    };

    let result = momento_functions_host::http::post(
        "https://api.openai.com/v1/embeddings",
        [
            ("authorization".to_string(), format!("Bearer {openai_key}")),
            ("content-type".to_string(), "application/json".to_string()),
        ],
        // 1536 float32 for text-embedding-3-small
        serde_json::json!({
            "model": "text-embedding-3-small",
            "encoding_format": "float",
            "input": [query],
        })
        .to_string(),
    );
    let mut response = result?;
    log::debug!("OpenAI response: {:?}", response.headers);
    let Json(EmbeddingResponse { mut data }) = response.extract()?;
    data.sort_by_key(|d| d.index);
    Ok(data)
}

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
            LogMode::Topic {
                topic: "turbopuffer-index-articles".to_string(),
            },
        )?;
    }
    Ok(())
}
