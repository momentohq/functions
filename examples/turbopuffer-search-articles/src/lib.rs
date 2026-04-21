//! After indexing articles with `turbopuffer-index-articles`, query them by
//! text. The query embedding is fetched from OpenAI on miss and cached in
//! Momento Cache to keep response latency low.
//!
//! Required env vars: `OPENAI_API_KEY`, `TURBOPUFFER_REGION`,
//! `TURBOPUFFER_NAMESPACE`, `TURBOPUFFER_API_KEY`. Optional: `TTL_SECONDS`.

use std::time::Duration;

use momento_functions_bytes::encoding::{Extract, Json};
use momento_functions_cache as cache;
use momento_functions_guest_web::{WebEnvironment, WebError, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_http::{Request as HttpRequest, invoke as http_invoke};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    vector: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_content: Option<String>,
    #[serde(rename = "metadata$title", skip_serializing_if = "Option::is_none")]
    metadata_title: Option<String>,
    #[serde(rename = "metadata$link", skip_serializing_if = "Option::is_none")]
    metadata_link: Option<String>,
    #[serde(rename = "metadata$authors", skip_serializing_if = "Option::is_none")]
    metadata_authors: Option<Vec<String>>,
    #[serde(rename = "metadata$language", skip_serializing_if = "Option::is_none")]
    metadata_language: Option<String>,
    #[serde(
        rename = "metadata$description",
        skip_serializing_if = "Option::is_none"
    )]
    metadata_description: Option<String>,
    #[serde(rename = "metadata$feed", skip_serializing_if = "Option::is_none")]
    metadata_feed: Option<String>,
}

const DEFAULT_TTL_SECONDS: u64 = 30;

invoke!(search);
fn search(Json(request): Json<Request>) -> WebResult<WebResponse> {
    setup_logging()?;

    let Request {
        query,
        topk,
        include_attributes,
        filters,
    } = request;
    let embeddings = get_cached_query_embedding(query)?;
    let topk = topk.unwrap_or(5);
    let include_attributes = include_attributes.unwrap_or_default();

    let turbopuffer_api_key = format!(
        "Bearer {}",
        std::env::var("TURBOPUFFER_API_KEY").unwrap_or_default()
    );
    let turbopuffer_region = std::env::var("TURBOPUFFER_REGION").unwrap_or_default();
    let turbopuffer_namespace = std::env::var("TURBOPUFFER_NAMESPACE").unwrap_or_default();
    let turbopuffer_endpoint = format!(
        "https://{turbopuffer_region}.turbopuffer.com/v2/namespaces/{turbopuffer_namespace}/query"
    );

    log::debug!("querying turbopuffer with topk={topk}, include_attributes={include_attributes:?}");
    let response = http_invoke(
        HttpRequest::new(turbopuffer_endpoint, "POST")
            .with_headers([
                ("Authorization", turbopuffer_api_key.as_str()),
                ("Content-Type", "application/json"),
                ("Accept", "*/*"),
                ("User-Agent", "momento-turbopuffer-example"),
            ])
            .with_body(
                json!({
                    "rank_by": ["vector", "ANN", embeddings],
                    "top_k": topk,
                    "include_attributes": include_attributes,
                    "filters": filters,
                })
                .to_string(),
            ),
    )?;

    if response.status != 200 {
        let body = response.body.into_bytes();
        let message = format!(
            "Failed to search documents: {}",
            String::from_utf8(body).unwrap_or_default()
        );
        return Ok(WebResponse::new()
            .with_status(response.status)
            .with_body(json!({ "message": message }).to_string())?);
    }

    let Json(QueryResponse { rows }) = Json::<QueryResponse>::extract(response.body)?;
    let response_body = serde_json::to_vec(&rows)?;
    Ok(WebResponse::new()
        .with_status(200)
        .header("Content-Type", "application/json")
        .with_body(response_body)?)
}

fn get_cached_query_embedding(query: String) -> WebResult<Vec<f32>> {
    log::debug!("Checking if embeddings are already cached for \"{query}\"");
    if let Some(hit) = cache::get::<Vec<u8>>(query.clone())? {
        log::debug!("cache hit");
        return hit
            .chunks_exact(4)
            .map(|chunk| {
                let arr = <[u8; 4]>::try_from(chunk)
                    .map_err(|_| WebError::message("Chunk length should be 4"))?;
                Ok(f32::from_le_bytes(arr))
            })
            .collect();
    }

    log::debug!("cache miss, querying embeddings from OpenAI");
    let embedding = get_embeddings(query.clone())?
        .into_iter()
        .next()
        .ok_or_else(|| {
            log::error!("Failed to get embedding for query: {query}");
            WebError::message("Failed to get embedding for query")
        })?;

    let bytes: Vec<u8> = embedding
        .embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();
    let ttl: u64 = std::env::var("TTL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_TTL_SECONDS);
    cache::set(query, bytes, Duration::from_secs(ttl))?;
    Ok(embedding.embedding)
}

fn get_embeddings(query: String) -> WebResult<Vec<EmbeddingData>> {
    log::debug!("getting embeddings for content: {query:?}");
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
    Ok(data)
}

fn setup_logging() -> WebResult<()> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Recursive filter representation. Turbopuffer validates the structure;
// these types just carry it across the wire.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    Logical(LogicalFilter),
    Not(NotFilter),
    Comparison(ComparisonFilter),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogicalFilter(pub String, pub Vec<Filter>);

#[derive(Debug, Serialize, Deserialize)]
pub struct NotFilter(pub String, pub Box<Filter>);

#[derive(Debug, Serialize, Deserialize)]
pub struct ComparisonFilter(pub String, pub ComparisonOp, pub Value);

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
