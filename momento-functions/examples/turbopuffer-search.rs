//! Using the embeddings from the fine-foods-embeddings example, this
//! example performs a nearest-neighbor query through the documents in the Turbopuffer namespace.
//!
//! You need to provide `OPENAI_KEY`, `TURBOPUFFER_ENDPOINT` and `TURBOPUFFER_API_KEY`
//! environment variables upon creating the function. If you'd like to store queries for longer,
//! pass along the `TTL` environment variable upon creating the function.
//!
//! Once uploaded, you can call with:
//!
//! ```bash
//! curl \                                                                             
//! https://api.cache.$MOMENTO_CELL_HOSTNAME/functions/$MOMENTO_CACHE_NAME/turbopuffer-search \
//! -H "authorization: $MOMENTO_API_KEY" \
//! -H "Content-Type: application/json" \
//! -d '{"topk": 5, "query": "sweet food"}'
//! ```

use std::time::Duration;

use log::LevelFilter;

use momento_functions::{WebError, WebResponse, WebResult};
use momento_functions_host::{cache, encoding::Json, web_extensions::headers};
use momento_functions_log::LogMode;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Debug)]
struct Request {
    query: String,
    topk: Option<usize>,
    include_attributes: Option<Vec<String>>,
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

// Default to 30 second caching time for queries in Momento
const DEFAULT_TTL_SECONDS: u64 = 30;

momento_functions::post!(search);
fn search(Json(request): Json<Request>) -> WebResult<WebResponse> {
    let headers = headers();
    setup_logging(&headers)?;

    let Request {
        query,
        topk,
        include_attributes,
    } = request;
    // Transform our human-readable text into embeddings via OpenAI
    let embeddings = get_cached_query_embedding(query)?;
    // Default to top 5 results if not provided
    let topk = topk.unwrap_or(5);
    // Default to empty list if not provided
    let include_attributes = include_attributes.unwrap_or_default();

    // These are passed in as environment variables when creating the function
    let turbopuffer_api_key = format!(
        "Bearer {}",
        std::env::var("TURBOPUFFER_API_KEY").unwrap_or_default()
    );
    let turbopuffer_endpoint = std::env::var("TURBOPUFFER_ENDPOINT").unwrap_or_default();

    let result = momento_functions_host::http::post(
        turbopuffer_endpoint,
        [
            ("Authorization".to_string(), turbopuffer_api_key.to_string()),
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "*/*".to_string()),
            (
                "User-Agent".to_string(),
                "momento-turbobuffer-example".to_string(),
            ),
        ],
        json!({
            "rank_by": ["vector", "ANN", embeddings],
            "top_k": topk,
            "include_attributes": include_attributes,
        }),
    );
    log::debug!("Turbopuffer response: {result:?}");
    match result {
        Ok(response) => {
            if response.status != 200 {
                let message = format!(
                    "Failed to search documents: {}",
                    String::from_utf8(response.body).unwrap_or_default(),
                );
                return Ok(WebResponse::new()
                    .with_status(response.status)
                    .with_body(json!({
                        "message": message,
                    }))?);
            }
            Ok(WebResponse::new()
                .with_status(200)
                .with_body(Json(response))?)
        }
        Err(e) => {
            log::error!("Failed to search documents: {e:?}");
            Ok(WebResponse::new().with_status(500).with_body(json!({
                "message": e.to_string(),
            }))?)
        }
    }
}

fn get_cached_query_embedding(query: String) -> WebResult<Vec<f32>> {
    log::debug!("Checking if embeddings are already cached for \"{query}\"");
    Ok(match cache::get::<Vec<u8>>(query.clone())? {
        Some(hit) => {
            log::debug!("cache hit");
            // Convert raw bytes back into our Vec<f32> type
            hit.chunks_exact(4)
                .map(|chunk| {
                    let arr = <[u8; 4]>::try_from(chunk).expect("Chunk length should be 4");
                    f32::from_le_bytes(arr)
                })
                .collect()
        }
        None => {
            log::debug!("cache miss, querying embeddings from open ai");
            match get_embeddings(query.clone())?.into_iter().next() {
                Some(embedding) => {
                    // Convert to raw bytes before storing in cache
                    let new_query_embedding = embedding
                        .embedding
                        .clone()
                        .into_iter()
                        .flat_map(f32::to_le_bytes)
                        .collect::<Vec<u8>>();
                    let ttl: u64 = std::env::var("TTL")
                        .unwrap_or("".to_string())
                        .parse::<u64>()
                        .unwrap_or(DEFAULT_TTL_SECONDS);
                    cache::set(query, new_query_embedding.clone(), Duration::from_secs(ttl))?;
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

// ------------------------------------------------------
// | Utility functions for convenience
// ------------------------------------------------------

fn get_embeddings(query: String) -> WebResult<Vec<EmbeddingData>> {
    log::debug!("getting embeddings for document with content: {query:?}");
    let query = if query.contains("\n") {
        // openai guide currently says to replace newlines with spaces. This, then, must be how you get the cargo to come.
        // https://platform.openai.com/docs/guides/embeddings
        query.replace("\n", " ")
    } else {
        query
    };

    // Required to be set as an environment variable when creating the function
    let openapi_key = std::env::var("OPENAI_KEY").unwrap_or_default();
    let result = momento_functions_host::http::post(
        "https://api.openai.com/v1/embeddings",
        [
            ("authorization".to_string(), format!("Bearer {openapi_key}")),
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
    log::debug!("OpenAI response: {result:?}");
    let mut response = result?;
    let Json(EmbeddingResponse { mut data }) = response.extract()?;
    data.sort_by_key(|d| d.index);
    log::debug!("OpenAI extracted data: {data:?}");
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
                topic: "turbopuffer-search".to_string(),
            },
        )?;
    }
    Ok(())
}
