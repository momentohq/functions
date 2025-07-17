//! After we've indexed data using `turbopuffer-index-articles.rs`, this function will
//! query OpenAI to generate embeddings for our search text, then query our
//! Turbopuffer namespace using a nearest-neighbor search. To speed things up,
//! we'll use Momento in-host caching for the embedding queries.
//!
//! You need to provide `OPENAI_KEY`, `TURBOPUFFER_ENDPOINT`, and `TURBOPUFFER_API_KEY`
//! environment variables when creating this function:
//! * `OPENAI_KEY`              -> The API key for accessing OpenAI, shoud just be the key itself.
//! * `TURBOPUFFER_ENDPOINT`    -> The endpoint contains the namespace. Ensure it ends with `/query`
//! * `TURBOPUFFER_API_KEY`     -> The API key should just be the key itself.
//!
//! You can also provide the `TTL_SECONDS` environment variable to override the default
//! ttl used to store embeddings in your Momento cache.
//!
//! To demo this, you can create your function and then pipe your search query as JSON
//! into your function. See Turbopuffer's Query docs page for example filters:
//! https://turbopuffer.com/docs/query#param-filters
//!
//! ```bash
//! # Export your environment variables
//! export MOMENTO_CELL_HOSTNAME=<momento cell>
//! export MOMENTO_CACHE_NAME=my-functions-cache
//! export MOMENTO_API_KEY=<your api key>
//!
//! export OPENAI_KEY=<openai api key>
//! export TURBOPUFFER_ENDPOINT=<Should be v2 namespace suffixed by `/query`>
//! export TURBOPUFFER_API_KEY=<turbopuffer api key>
//!
//! # Create your Momento cache
//! momento cache create $MOMENTO_CACHE_NAME
//!
//! # Create your Momento function
//! momento preview function put-function \
//!   --cache-name "$MOMENTO_CACHE_NAME" \
//!   --name turbopuffer-search-articles \
//!   --wasm-file /path/to/this/compiled/turbopuffer_search_articles.wasm \
//!   -E OPENAI_KEY="$OPENAI_KEY" \
//!   -E TURBOPUFFER_ENDPOINT="$TURBOPUFFER_ENDPOINT" \
//!   -E TURBOPUFFER_API_KEY="$TURBOPUFFER_API_KEY"
//!
//! # Perform a basic text search
//! curl --silent \
//! https://api.cache.$MOMENTO_CELL_HOSTNAME/functions/$MOMENTO_CACHE_NAME/turbopuffer-search-articles \
//!  -H "authorization: $MOMENTO_API_KEY" \
//!  -H "Content-Type: application/json" \
//!  -d '{"topk": 5, "query": "Portland Trail Blazers", "include_attributes": ["metadata$title", "metadata$link"]}'  | jq
//!
//! # Filter out entries that include a glob, and filter out a specific ID
//! read -r -d '' payload << EOM
//! {
//!  "topk": 25,
//!  "query": "Portland Trail Blazers",
//!  "include_attributes": [
//!    "metadata\$title",
//!    "metadata\$link"
//!  ],
//!  "filters": [
//!    "And",
//!    [
//!      ["metadata\$title", "NotGlob", "*Mock Draft*"],
//!      ["Not", ["id", "In", ["11782925380870169755"]]]
//!    ]
//!  ]
//! }
//! EOM
//! curl --silent \
//!  https://api.cache.$MOMENTO_CELL_HOSTNAME/functions/$MOMENTO_CACHE_NAME/turbopuffer-search-articles \
//!  -H "authorization: $MOMENTO_API_KEY" \
//!  -H "Content-Type: application/json" \
//!  -d "$payload" | jq
//!
//! # Now do the same thing, but use `jq` to reduce articles with a cosine distance <= 0.6
//! read -r -d '' payload << EOM
//! {
//!  "topk": 25,
//!  "query": "Portland Trail Blazers",
//!  "include_attributes": [
//!    "metadata\$title",
//!    "metadata\$link"
//!  ],
//!  "filters": [
//!    "And",
//!    [
//!      ["metadata\$title", "NotGlob", "*Mock Draft*"],
//!      ["Not", ["id", "In", ["11782925380870169755"]]]
//!    ]
//!  ]
//! }
//! EOM
//! curl --silent \
//!  https://api.cache.$MOMENTO_CELL_HOSTNAME/functions/$MOMENTO_CACHE_NAME/turbopuffer-search-articles \
//!  -H "authorization: $MOMENTO_API_KEY" \
//!  -H "Content-Type: application/json" \
//!  -d "$payload" | \
//! jq '[.[] | select(.dist <= 0.6)]'
//! ```

use std::time::Duration;

use log::LevelFilter;

use momento_functions::{WebError, WebResponse, WebResult};
use momento_functions_host::{cache, encoding::Json, web_extensions::headers};
use momento_functions_log::LogMode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize, Debug)]
struct Request {
    query: String,
    topk: Option<usize>,
    include_attributes: Option<Vec<String>>,
    filters: Option<Filter>,
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
    // See example in `turbopuffer-index-articles.rs`.
    // If we specify attributes in `include_attributes`, we
    // want to deserialize those in our response body
    #[serde(skip_serializing_if = "Option::is_none")]
    vector: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_content: Option<String>,
    #[serde(rename = "metadata$title")]
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_title: Option<String>,
    #[serde(rename = "metadata$link")]
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_link: Option<String>,
    #[serde(rename = "metadata$authors")]
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_authors: Option<Vec<String>>,
    #[serde(rename = "metadata$language")]
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_language: Option<String>,
    #[serde(rename = "metadata$description")]
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "metadata$feed")]
    metadata_feed: Option<String>,
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
        filters,
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

    log::debug!(
        "querying turbopuffer with (topk={topk}), (include_attributes={include_attributes:?})",
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
            "filters": filters,
        }),
    );
    match result {
        Ok(mut response) => {
            log::debug!(
                "Turbopuffer response: {} - {:?}",
                response.status,
                response.headers
            );
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
            let raw = String::from_utf8(response.body.clone()).unwrap_or_default();
            // You can set this to debug if you'd like to see what Turbopuffer is sending back
            log::trace!("Turbopuffer response body: {raw:?}");
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

// ------------------------------------------------------
// | Utility functions for convenience
// ------------------------------------------------------

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
                    let ttl: u64 = std::env::var("TTL_SECONDS")
                        .unwrap_or(DEFAULT_TTL_SECONDS.to_string())
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
    let mut response = result?;
    log::debug!(
        "OpenAI response: {} - {:?}",
        response.status,
        response.headers
    );
    let Json(EmbeddingResponse { mut data }) = response.extract()?;
    data.sort_by_key(|d| d.index);
    log::trace!("OpenAI extracted data: {data:?}");
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
                topic: "turbopuffer-search-articles".to_string(),
            },
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// | Utility structs when de/serializing filtering args for Turbopuffer
// ---------------------------------------------------------------------------

// Recursive struct without any validation, Turbopuffer will let you know
// if the filter is invalid.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    // Logical group: ["And", [filters...]] or ["Or", [filters...]]
    Logical(LogicalFilter),

    // Negation: ["Not", filter]
    Not(NotFilter),

    // Atomic comparison: ["field", "op", value]
    Comparison(ComparisonFilter),
}

/// Represents a logical group like ["And", [...]] or ["Or", [...]]
#[derive(Debug, Serialize, Deserialize)]
pub struct LogicalFilter(
    pub String,      // "And" or "Or"
    pub Vec<Filter>, // nested filters
);

/// Represents a unary negation: ["Not", inner_filter]
#[derive(Debug, Serialize, Deserialize)]
pub struct NotFilter(
    pub String,      // must be "Not"
    pub Box<Filter>, // inner filter
);

/// Represents a basic comparison: ["field", "op", value]
#[derive(Debug, Serialize, Deserialize)]
pub struct ComparisonFilter(
    pub String,       // field
    pub ComparisonOp, // operator
    pub Value,        // right-hand side (string, number, or array)
);

/// Supported comparison operators
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ComparisonOp {
    Eq,
    Neq,
    In,
    NotIn,
    Lt,
    Lte,
    Gt,
    Gte,
    Glob,
    NotGlob,
}
