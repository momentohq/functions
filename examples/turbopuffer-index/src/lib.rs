//! Indexes the documents from the `fine-foods-embeddings` example into a
//! Turbopuffer namespace.
//!
//! Required env vars:
//! * `TURBOPUFFER_REGION` — e.g. `gcp-us-central1`
//! * `TURBOPUFFER_NAMESPACE`
//! * `TURBOPUFFER_API_KEY`

use itertools::Itertools;
use momento_functions_bytes::encoding::Json;
use momento_functions_guest_web::{WebEnvironment, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_http::{Request as HttpRequest, invoke as http_invoke};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Same as the `fine-foods-embeddings` output but with the embedding included.
#[derive(Deserialize, Serialize, Debug)]
struct Document {
    #[serde(alias = "embedding")]
    vector: Vec<f32>,
    id: String,
    product_id: String,
    user_id: String,
    profile_name: String,
    helpfulness_numerator: i32,
    helpfulness_denominator: i32,
    score: i32,
    time: u32,
    summary: String,
    text: String,
}

invoke!(index_document);
fn index_document(Json(documents): Json<Vec<Document>>) -> WebResult<WebResponse> {
    setup_logging()?;

    let dimensions = match documents.first() {
        Some(doc) => doc.vector.len(),
        None => {
            log::warn!("No documents provided for indexing.");
            return Ok(WebResponse::new()
                .with_status(400)
                .with_body("No documents provided")?);
        }
    };
    log::debug!(
        "indexing {} documents with {dimensions} dimensions",
        documents.len()
    );

    let turbopuffer_api_key = format!(
        "Bearer {}",
        std::env::var("TURBOPUFFER_API_KEY").unwrap_or_default()
    );
    let turbopuffer_region = std::env::var("TURBOPUFFER_REGION").unwrap_or_default();
    let turbopuffer_namespace = std::env::var("TURBOPUFFER_NAMESPACE").unwrap_or_default();
    let turbopuffer_endpoint = format!(
        "https://{turbopuffer_region}.turbopuffer.com/v2/namespaces/{turbopuffer_namespace}"
    );

    for chunk in &documents.into_iter().chunks(2000) {
        let chunk: Vec<Document> = chunk.collect();
        let response = http_invoke(
            HttpRequest::new(&turbopuffer_endpoint, "POST")
                .with_headers([
                    ("Authorization", turbopuffer_api_key.as_str()),
                    ("Content-Type", "application/json"),
                    ("Accept", "*/*"),
                    ("User-Agent", "momento-turbopuffer-example"),
                ])
                .with_body(
                    json!({
                        "upsert_rows": chunk,
                        "distance_metric": "cosine_distance",
                    })
                    .to_string(),
                ),
        )?;
        if response.status != 200 {
            let body = response.body.into_bytes();
            let message = format!(
                "Failed to index documents: {}",
                String::from_utf8(body).unwrap_or_default()
            );
            return Ok(WebResponse::new()
                .with_status(response.status)
                .with_body(json!({ "message": message }).to_string())?);
        }
    }

    Ok(WebResponse::new()
        .with_status(200)
        .with_body(json!({ "message": "Documents indexed successfully" }).to_string())?)
}

fn setup_logging() -> WebResult<()> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;
    Ok(())
}
