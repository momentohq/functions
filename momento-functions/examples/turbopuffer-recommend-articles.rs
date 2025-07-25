//! This function presumes we've indexed data using `turbopuffer-index-articles.rs`.
//! From there, we can provide an input of article IDs to simulate a user's "preferred"
//! content to recommend similar articles.
//!
//! You need to provide `TURBOPUFFER_REGION`, `TURBOPUFFER_NAMESPACE` and `TURBOPUFFER_API_KEY`
//! environment variables when creating this function:
//! * `TURBOPUFFER_REGION`      -> Region your namespace resides. E.g. gcp-us-central1
//! * `TURBOPUFFER_NAMESPACE`   -> Namespace within your turbopuffer account
//! * `TURBOPUFFER_API_KEY`     -> The API key should just be the key itself.
//!
//! You can also provide the `TTL_SECONDS` environment variable to override the default
//! ttl used to store embeddings in your Momento cache.
//!
//! To demo this, you can create your function and then use your provided data
//! to query by ID.
//!
//! ```bash
//! # Export your environment variables
//! export MOMENTO_CELL_HOSTNAME=<momento cell>
//! export MOMENTO_CACHE_NAME=my-functions-cache
//! export MOMENTO_API_KEY=<your api key>
//!
//! export TURBOPUFFER_REGION=<turbopuffer region>
//! export TURBOPUFFER_NAMESPACE=<turbopuffer namespace>
//! export TURBOPUFFER_API_KEY=<turbopuffer api key>
//!
//! # Create your Momento cache
//! momento cache create $MOMENTO_CACHE_NAME
//!
//! # Create your Momento function
//! momento preview function put-function \
//!   --cache-name "$MOMENTO_CACHE_NAME" \
//!   --name turbopuffer-recommended-articles \
//!   --wasm-file /path/to/this/compiled/turbopuffer_search_articles.wasm \
//!   -E OPENAI_API_KEY="$OPENAI_API_KEY" \
//!   -E TURBOPUFFER_NAMESPACE="$TURBOPUFFER_NAMESPACE" \
//!   -E TURBOPUFFER_API_KEY="$TURBOPUFFER_API_KEY"
//!
//! # Find some recommended articles!
//! curl --silent \
//! https://api.cache.$MOMENTO_CELL_HOSTNAME/functions/$MOMENTO_CACHE_NAME/turbopuffer-recommended-articles \
//!  -H "authorization: $MOMENTO_API_KEY" \
//!  -H "Content-Type: application/json" \
//!  -d '{"article_ids": ["4484641135897252353", "6703952717813182352", "4965577503700120031"]'  | jq
//! ```

use std::{collections::HashMap, time::Duration};

use log::LevelFilter;

use momento_functions::{WebError, WebResponse, WebResult};
use momento_functions_host::{cache, encoding::Json, web_extensions::headers};
use momento_functions_log::LogMode;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(alias = "$dist")]
    dist: Option<f32>,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector: Option<Vec<f32>>,
    #[serde(rename = "metadata$title")]
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_title: Option<String>,
    #[serde(rename = "metadata$link")]
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_link: Option<String>,
}

// Default to 5 minute caching time for queries in Momento
const DEFAULT_TTL_SECONDS: u64 = 300;
// Filter out articles that aren't similar enough
const MAXIMUM_COSINE_DISTANCE: f32 = 0.6;

momento_functions::post!(get_recommended_articles);
fn get_recommended_articles(Json(request): Json<Request>) -> WebResult<WebResponse> {
    let headers = headers();
    setup_logging(&headers)?;

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
    let ttl_seconds = std::env::var("TTL_SECONDS")
        .unwrap_or(DEFAULT_TTL_SECONDS.to_string())
        .parse::<u64>()
        .unwrap_or(DEFAULT_TTL_SECONDS);
    let ttl = Duration::from_secs(ttl_seconds);

    let Request { article_ids, topk } = request;

    // Default to top 10 articles if not provided
    let topk = topk.unwrap_or(10);

    // Get the embeddings from our provided articles, which may or may not exist.
    // Our returned article embeddings are given in the order of our input. Nothing is really being
    // done with this now, but if you wanted to, you could assign different weights to the items
    // in the front of the list (e.g. most recently seen articles) before calcuating the mean vector.
    //
    // For example, this could instead be something like:
    // ```
    // let mut article_embeddings = get_article_embeddings(
    //     article_ids.clone(),
    //     &turbopuffer_endpoint,
    //     &turbopuffer_api_key,
    //     &ttl,
    // )?;
    // // Assign a higher weight to the first four articles (last 4 recently seen)
    // for (_, embedding) in article_embeddings.iter_mut().take(4) {
    //     if let Some(embedding) = embedding {
    //         for i in 0..embedding.len() {
    //             embedding *= 0.275;
    //         }
    //     }
    // }
    let article_embeddings = get_article_embeddings(
        article_ids.clone(),
        &turbopuffer_endpoint,
        &turbopuffer_api_key,
        &ttl,
    )?;

    // Some articles may not have had embeddings at all (e.g. the provided ID doesn't exist, was deleted, etc.),
    // so only build out our vector of vector embeddings with actual values before calculating the mean vector.
    let embeddings: Vec<Vec<f32>> = article_embeddings
        .iter()
        .flat_map(|(article_id, maybe_embedding)| {
            if maybe_embedding.is_none() {
                log::debug!("no embeddings found for {article_id}");
            }
            maybe_embedding.clone()
        })
        .collect();

    // Calculate the mean vector from our Vector of Vectors
    let mean_vector = match mean_vector(&embeddings) {
        Some(result) => result,
        None => {
            // We know which model we are using, so we can return an empty Vec of size 1536 initialized to 0.
            vec![0.0f32; 1536]
        }
    };

    // We have our mean vector, now query Turbopuffer to find similar articles close to our mean vector,
    // while also filtering out the articles we've already seen
    let recommended_articles = get_similar_articles_from_turbopuffer(
        mean_vector,
        article_ids,
        topk,
        &turbopuffer_endpoint,
        &turbopuffer_api_key,
    )?;

    let response_body = serde_json::to_vec(&recommended_articles)?;
    Ok(WebResponse::new()
        .with_status(200)
        .with_headers(vec![(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])
        .with_body(response_body)?)
}

// ------------------------------------------------------
// | Utility functions for convenience
// ------------------------------------------------------

/// Gets existing article vector embeddings from our momento cache first,
/// then queries (and caches) any misses from Turbopuffer itself, if an embedding
/// exists within Turbopuffer.
fn get_article_embeddings(
    article_ids: Vec<String>,
    turbopuffer_endpoint: &String,
    turbopuffer_api_key: &String,
    ttl: &Duration,
) -> WebResult<Vec<(String, Option<Vec<f32>>)>> {
    log::debug!("Getting article embeddings from momento (if available)");
    Ok({
        let mut embeddings_map = HashMap::new();
        // Although we're not exercising the concept here, you may want to preserve the ordering of
        // of article IDs to assign a weight towards most recently viewed articles (for example).
        // This showcases how we can still succinctly get embeddings (if they exist) while
        // mainting the order of these embeddings.
        for article_id in &article_ids {
            match get_article_embeddings_from_cache(article_id.clone())? {
                Some(embedding) => embeddings_map.insert(article_id.clone(), Some(embedding)),
                // Cache miss is fine, we'll insert it into our map and attempt a query from Turbopuffer
                None => embeddings_map.insert(article_id.clone(), None),
            };
        }

        let cache_misses: Vec<String> = embeddings_map
            .iter()
            .filter(|(_, embeddings)| embeddings.is_none())
            .map(|(article, _)| article.clone())
            .collect();

        // For all the misses, get their associated embeddings from our Turbopuffer namespace since
        // they are already indexed.
        if !cache_misses.is_empty() {
            let fetched_embeddings = get_article_embeddings_from_turbopuffer(
                cache_misses,
                turbopuffer_endpoint,
                turbopuffer_api_key,
                ttl,
            )?;
            // The ordering may not be preserved in our response, so we'll update the values in our map
            for (found_article, maybe_embeddings) in fetched_embeddings {
                embeddings_map.insert(found_article, maybe_embeddings);
            }
        }
        let mut embeddings = Vec::new();
        // As mentioned, the ordering may not be preserved from all of our responses, so we'll
        // iterate back through the original article ID list to rebuild the list of vectors.
        for article_id in &article_ids {
            embeddings.push((
                article_id.clone(),
                embeddings_map.get(article_id).cloned().unwrap_or_default(),
            ))
        }
        embeddings
    })
}

/// Gets the cached embeddings from our Momento cache, if they exist
fn get_article_embeddings_from_cache(article_id: String) -> WebResult<Option<Vec<f32>>> {
    match cache::get::<Vec<u8>>(article_id.clone())? {
        Some(hit) => {
            log::debug!("cache hit for key '{article_id}'");
            // Convert raw bytes back into our Vec<f32> type
            let embedding = hit
                .chunks_exact(4)
                .map(|chunk| {
                    let arr = <[u8; 4]>::try_from(chunk)
                        .map_err(|_| WebError::message("Chunk length should be 4"))?;
                    Ok(f32::from_le_bytes(arr))
                })
                .collect::<Result<Vec<f32>, WebError>>()?;
            Ok(Some(embedding))
        }
        None => {
            log::debug!("cache miss for key '{article_id}'");
            Ok(None)
        }
    }
}

/// Gets the actual embeddings from Turbopuffer, caching the vector embeddings in
/// our Momento cache.
fn get_article_embeddings_from_turbopuffer(
    article_ids: Vec<String>,
    turbopuffer_endpoint: &String,
    turbopuffer_api_key: &String,
    ttl: &Duration,
) -> WebResult<Vec<(String, Option<Vec<f32>>)>> {
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
            "top_k": article_ids.len(),
            "include_attributes": vec!["id", "vector"],
            "filters": Filter::Comparison(
                ComparisonFilter("id".to_string(),
                ComparisonOp::In,
                Value::Array(article_ids.iter().map(|id| Value::String(id.clone())).collect()))
            ),
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
                    "Failed to get indexed embeddings: {}",
                    String::from_utf8(response.body).unwrap_or_default(),
                );
                return Err(WebError::message(message));
            }
            let raw = String::from_utf8(response.body.clone()).unwrap_or_default();
            // You can set this to debug if you'd like to see what Turbopuffer is sending back
            log::trace!("Turbopuffer response body: {raw:?}");
            // Just get the data we care about, no need to report back Turbopuffer timings/billing info
            let Json(QueryResponse { rows }) = response.extract()?;
            let mut embeddings = Vec::new();
            // Now store in our cache for the future
            for row in rows {
                // Don't cache empty embeddings in Momento
                if let Some(vector) = &row.vector {
                    // Convert to raw bytes before storing in cache
                    let new_query_embedding = vector
                        .clone()
                        .into_iter()
                        .flat_map(f32::to_le_bytes)
                        .collect::<Vec<u8>>();
                    log::debug!("setting in cache for {} with ttl {:?}", row.id, ttl);
                    cache::set(row.id.clone(), new_query_embedding.clone(), *ttl)?;
                }
                embeddings.push((row.id, row.vector.clone()));
            }
            Ok(embeddings)
        }
        Err(e) => {
            log::error!("Failed to get indexed embeddings: {e:?}");
            Err(WebError::message(e.to_string()))
        }
    }
}

/// Given a mean vector, query Turbopuffer for similar articles while also filtering out
/// the articles we've already viewed. Will use `MAXIMUM_COSINE_DISTANCE` to maintain
/// quality within our results.
fn get_similar_articles_from_turbopuffer(
    mean_vector: Vec<f32>,
    already_seen_article_ids: Vec<String>,
    topk: usize,
    turbopuffer_endpoint: &String,
    turbopuffer_api_key: &String,
) -> WebResult<Vec<QueryRow>> {
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
            "rank_by": ["vector", "ANN", mean_vector],
            "top_k": topk,
            "include_attributes": vec!["id", "metadata$title", "metadata$link"],
            "filters": Filter::Comparison(
                ComparisonFilter("id".to_string(),
                ComparisonOp::NotIn,
                Value::Array(already_seen_article_ids.iter().map(|id| Value::String(id.clone())).collect()))
            ),
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
                return Err(WebError::message(message));
            }
            let raw = String::from_utf8(response.body.clone()).unwrap_or_default();
            // You can set this to debug if you'd like to see what Turbopuffer is sending back
            log::trace!("Turbopuffer response body: {raw:?}");
            // Just get the data we care about, no need to report back Turbopuffer timings/billing info
            let Json(QueryResponse { rows }) = response.extract()?;
            // To maintain quality, we'll filter out results that don't meet our distance threshold
            let filtered_rows = rows
                .into_iter()
                .filter(|row| row.dist.unwrap_or_default() <= MAXIMUM_COSINE_DISTANCE)
                .collect();
            Ok(filtered_rows)
        }
        Err(e) => {
            log::error!("Failed to search documents: {e:?}");
            Err(WebError::message(e.to_string()))
        }
    }
}

/// For this demo (and to keep the compiled WASM small), we're going to use our own
/// implementation for calculating the mean vector. We can speed up a lot of the calculations
/// using a little bit of loop optimization
fn mean_vector(vectors: &[Vec<f32>]) -> Option<Vec<f32>> {
    let total_vectors = vectors.len();
    if total_vectors == 0 {
        return None;
    }

    let dimension_size = vectors[0].len();
    // Populate a vector with the exact size of our dimensions
    let mut result = vec![0.0f32; dimension_size];

    for v in vectors {
        // Better than panicking, we'll just return None for now
        if v.len() != dimension_size {
            log::error!(
                "Failed to calculate mean vector! Expected a vector with dimension size {dimension_size} but was {}",
                v.len()
            );
            return None;
        }

        // Sum each value at position `i` and store in our result vector
        for i in 0..dimension_size {
            result[i] += v[i];
        }
    }

    // Multiplication is faster for CPU cycles, so calculate the average once
    // and then multiply each value in the loop instead of dividing to get the average.
    let inv_avg = 1.0 / (total_vectors as f32);
    for x in &mut result {
        *x *= inv_avg;
    }

    Some(result)
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
                topic: "turbopuffer-recommend-articles".to_string(),
            },
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// | Utility structs when de/serializing filtering args for Turbopuffer
// ---------------------------------------------------------------------------

// Recursive struct without any validation, Turbopuffer will let you know
// if the filter is invalid. Although these are largely uncessary, we've provded this
// here for more filter building/allowing for filter overrides via request parameters
// if you so choose.
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
