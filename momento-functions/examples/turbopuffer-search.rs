//! Using the embeddings from the fine-foods-embeddings example, this
//! example performs a nearest-neighbor query through the documents in the Turbopuffer namespace.
//!
//! You need to provide `OPENAI_API_KEY`, `TURBOPUFFER_REGION`, `TURBOPUFFER_NAMESPACE` and `TURBOPUFFER_API_KEY`
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

use momento_functions::{WebError, WebResponse, WebResult};
use momento_functions_host::{
    cache, encoding::Json, logging::LogDestination, web_extensions::FunctionEnvironment,
};

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

#[derive(Deserialize, Debug)]
struct QueryResponse {
    rows: Vec<QueryRow>,
}

#[derive(Deserialize, Serialize, Debug)]
struct QueryRow {
    #[serde(alias = "$dist")]
    dist: f32,
    id: String,
}

// Default to 30 second caching time for queries in Momento
const DEFAULT_TTL_SECONDS: u64 = 30;

momento_functions::post!(search);
fn search(Json(request): Json<Request>) -> WebResult<WebResponse> {
    setup_logging()?;

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
    let turbopuffer_region = std::env::var("TURBOPUFFER_REGION").unwrap_or_default();
    let turbopuffer_namespace = std::env::var("TURBOPUFFER_NAMESPACE").unwrap_or_default();
    let turbopuffer_endpoint = format!(
        "https://{turbopuffer_region}.turbopuffer.com/v2/namespaces/{turbopuffer_namespace}/query"
    );

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
        Ok(mut response) => {
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
            // Just get the data we care about, no need to report back Turbopuffer timings/billing info
            let Json(QueryResponse { rows }) = response.extract()?;
            let response_body = serde_json::to_vec(&rows)?;
            Ok(WebResponse::new()
                .with_status(200)
                .with_headers(vec![(
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                )])
                .with_body(response_body)?)
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
                    let arr = <[u8; 4]>::try_from(chunk)
                        .map_err(|_| WebError::message("Chunk length should be 4"))?;
                    Ok(f32::from_le_bytes(arr))
                })
                .collect::<Result<Vec<f32>, WebError>>()?
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

fn get_embeddings(mut query: String) -> WebResult<Vec<EmbeddingData>> {
    log::debug!("getting embeddings for document with content: {query:?}");
    // To try and fit within OpenAI's token limits
    query.truncate(10_000);

    // Required to be set as an environment variable when creating the function
    let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let result = momento_functions_host::http::post(
        "https://api.openai.com/v1/embeddings",
        [
            (
                "authorization".to_string(),
                format!("Bearer {openai_api_key}"),
            ),
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

fn setup_logging() -> WebResult<()> {
    let function_env = FunctionEnvironment::get_function_environment();
    momento_functions_log::configure_logs([
        LogDestination::topic(function_env.function_name()).into()
    ])?;
    Ok(())
}
