//! Recommends similar articles given a list of "preferred" article IDs.
//! Looks up each article's embedding (cached in Momento Cache, refreshed
//! from Turbopuffer on miss), averages the embeddings, then runs an ANN
//! search excluding the seed articles. Filters out results whose cosine
//! distance exceeds `MAXIMUM_COSINE_DISTANCE` to maintain quality.
//!
//! Required env vars: `TURBOPUFFER_REGION`, `TURBOPUFFER_NAMESPACE`,
//! `TURBOPUFFER_API_KEY`. Optional: `TTL_SECONDS`.

use std::{collections::HashMap, time::Duration};

use momento_functions_bytes::encoding::{Extract, Json};
use momento_functions_cache as cache;
use momento_functions_guest_web::{WebEnvironment, WebError, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_http::{Request as HttpRequest, invoke as http_invoke};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize, Debug)]
struct Request {
    article_ids: Vec<String>,
    topk: Option<usize>,
}

#[derive(Deserialize, Debug)]
struct QueryResponse {
    rows: Vec<QueryRow>,
}

#[derive(Deserialize, Serialize, Debug)]
struct QueryRow {
    #[serde(alias = "$dist", skip_serializing_if = "Option::is_none")]
    dist: Option<f32>,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector: Option<Vec<f32>>,
    #[serde(rename = "metadata$title", skip_serializing_if = "Option::is_none")]
    metadata_title: Option<String>,
    #[serde(rename = "metadata$link", skip_serializing_if = "Option::is_none")]
    metadata_link: Option<String>,
}

const DEFAULT_TTL_SECONDS: u64 = 300;
const MAXIMUM_COSINE_DISTANCE: f32 = 0.6;

invoke!(get_recommended_articles);
fn get_recommended_articles(Json(request): Json<Request>) -> WebResult<WebResponse> {
    setup_logging()?;

    let turbopuffer_api_key = format!(
        "Bearer {}",
        std::env::var("TURBOPUFFER_API_KEY").unwrap_or_default()
    );
    let turbopuffer_region = std::env::var("TURBOPUFFER_REGION").unwrap_or_default();
    let turbopuffer_namespace = std::env::var("TURBOPUFFER_NAMESPACE").unwrap_or_default();
    let turbopuffer_endpoint = format!(
        "https://{turbopuffer_region}.turbopuffer.com/v2/namespaces/{turbopuffer_namespace}/query"
    );
    let ttl_seconds = std::env::var("TTL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_TTL_SECONDS);
    let ttl = Duration::from_secs(ttl_seconds);

    let Request { article_ids, topk } = request;
    let topk = topk.unwrap_or(10);

    let article_embeddings = get_article_embeddings(
        article_ids.clone(),
        &turbopuffer_endpoint,
        &turbopuffer_api_key,
        &ttl,
    )?;

    // Drop the IDs that didn't have an embedding before computing the mean.
    let embeddings: Vec<Vec<f32>> = article_embeddings
        .iter()
        .filter_map(|(article_id, maybe_embedding)| {
            if maybe_embedding.is_none() {
                log::debug!("no embeddings found for {article_id}");
            }
            maybe_embedding.clone()
        })
        .collect();

    let mean = mean_vector(&embeddings).unwrap_or_else(|| vec![0.0_f32; 1536]);

    let recommended = get_similar_articles_from_turbopuffer(
        mean,
        article_ids,
        topk,
        &turbopuffer_endpoint,
        &turbopuffer_api_key,
    )?;

    let response_body = serde_json::to_vec(&recommended)?;
    Ok(WebResponse::new()
        .with_status(200)
        .header("Content-Type", "application/json")
        .with_body(response_body)?)
}

fn get_article_embeddings(
    article_ids: Vec<String>,
    endpoint: &str,
    api_key: &str,
    ttl: &Duration,
) -> WebResult<Vec<(String, Option<Vec<f32>>)>> {
    log::debug!("Getting article embeddings from cache (if available)");
    let mut embeddings_map: HashMap<String, Option<Vec<f32>>> = HashMap::new();
    for article_id in &article_ids {
        embeddings_map.insert(
            article_id.clone(),
            get_article_embeddings_from_cache(article_id.clone())?,
        );
    }

    let cache_misses: Vec<String> = embeddings_map
        .iter()
        .filter(|(_, e)| e.is_none())
        .map(|(id, _)| id.clone())
        .collect();

    if !cache_misses.is_empty() {
        let fetched =
            get_article_embeddings_from_turbopuffer(cache_misses, endpoint, api_key, ttl)?;
        for (id, maybe_embedding) in fetched {
            embeddings_map.insert(id, maybe_embedding);
        }
    }

    // Reconstruct the original ordering.
    let mut embeddings = Vec::with_capacity(article_ids.len());
    for article_id in &article_ids {
        embeddings.push((
            article_id.clone(),
            embeddings_map.get(article_id).cloned().unwrap_or_default(),
        ));
    }
    Ok(embeddings)
}

fn get_article_embeddings_from_cache(article_id: String) -> WebResult<Option<Vec<f32>>> {
    match cache::get::<Vec<u8>>(article_id.clone())? {
        Some(hit) => {
            log::debug!("cache hit for key '{article_id}'");
            let embedding = hit
                .chunks_exact(4)
                .map(|chunk| {
                    let arr = <[u8; 4]>::try_from(chunk)
                        .map_err(|_| WebError::message("Chunk length should be 4"))?;
                    Ok(f32::from_le_bytes(arr))
                })
                .collect::<WebResult<Vec<f32>>>()?;
            Ok(Some(embedding))
        }
        None => {
            log::debug!("cache miss for key '{article_id}'");
            Ok(None)
        }
    }
}

fn get_article_embeddings_from_turbopuffer(
    article_ids: Vec<String>,
    endpoint: &str,
    api_key: &str,
    ttl: &Duration,
) -> WebResult<Vec<(String, Option<Vec<f32>>)>> {
    let response = http_invoke(
        HttpRequest::new(endpoint, "POST")
            .with_headers([
                ("Authorization", api_key),
                ("Content-Type", "application/json"),
                ("Accept", "*/*"),
                ("User-Agent", "momento-turbopuffer-example"),
            ])
            .with_body(
                json!({
                    "top_k": article_ids.len(),
                    "include_attributes": vec!["id", "vector"],
                    "filters": Filter::Comparison(ComparisonFilter(
                        "id".to_string(),
                        ComparisonOp::In,
                        Value::Array(
                            article_ids
                                .iter()
                                .map(|id| Value::String(id.clone()))
                                .collect(),
                        ),
                    )),
                })
                .to_string(),
            ),
    )?;
    if response.status != 200 {
        let bytes = response.body.into_bytes();
        return Err(WebError::message(format!(
            "Failed to get indexed embeddings: {}",
            String::from_utf8(bytes).unwrap_or_default()
        )));
    }
    let Json(QueryResponse { rows }) = Json::<QueryResponse>::extract(response.body)?;

    let mut embeddings = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(vector) = &row.vector {
            let bytes: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
            log::debug!("setting in cache for {} with ttl {:?}", row.id, ttl);
            cache::set(row.id.clone(), bytes, *ttl)?;
        }
        embeddings.push((row.id, row.vector.clone()));
    }
    Ok(embeddings)
}

fn get_similar_articles_from_turbopuffer(
    mean_vector: Vec<f32>,
    seen: Vec<String>,
    topk: usize,
    endpoint: &str,
    api_key: &str,
) -> WebResult<Vec<QueryRow>> {
    let response = http_invoke(
        HttpRequest::new(endpoint, "POST")
            .with_headers([
                ("Authorization", api_key),
                ("Content-Type", "application/json"),
                ("Accept", "*/*"),
                ("User-Agent", "momento-turbopuffer-example"),
            ])
            .with_body(
                json!({
                    "rank_by": ["vector", "ANN", mean_vector],
                    "top_k": topk,
                    "include_attributes": vec!["metadata$title", "metadata$link"],
                    "filters": Filter::Comparison(ComparisonFilter(
                        "id".to_string(),
                        ComparisonOp::NotIn,
                        Value::Array(seen.iter().map(|id| Value::String(id.clone())).collect()),
                    )),
                })
                .to_string(),
            ),
    )?;
    if response.status != 200 {
        let bytes = response.body.into_bytes();
        return Err(WebError::message(format!(
            "Failed to search documents: {}",
            String::from_utf8(bytes).unwrap_or_default()
        )));
    }
    let Json(QueryResponse { rows }) = Json::<QueryResponse>::extract(response.body)?;
    Ok(rows
        .into_iter()
        .filter(|row| row.dist.unwrap_or_default() <= MAXIMUM_COSINE_DISTANCE)
        .collect())
}

fn mean_vector(vectors: &[Vec<f32>]) -> Option<Vec<f32>> {
    let total = vectors.len();
    if total == 0 {
        return None;
    }
    let dimensions = vectors[0].len();
    let mut result = vec![0.0_f32; dimensions];

    for v in vectors {
        if v.len() != dimensions {
            log::error!(
                "Failed to calculate mean vector! Expected dimension {dimensions} but got {}",
                v.len()
            );
            return None;
        }
        for (i, x) in v.iter().enumerate() {
            result[i] += x;
        }
    }

    let inv_avg = 1.0 / total as f32;
    for x in &mut result {
        *x *= inv_avg;
    }
    Some(result)
}

fn setup_logging() -> WebResult<()> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Recursive filter representation, see turbopuffer-search-articles.
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
