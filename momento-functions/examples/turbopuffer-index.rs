//! Using the embeddings from the fine-foods-embeddings example, this
//! example indexes the documents into a Turbopuffer namespace.
//!
//! You need to provide `TURBOPUFFER_REGION`, `TURBOPUFFER_NAMESPACE`, and `TURBOPUFFER_API_KEY`
//! environment variables.
//! * `TURBOPUFFER_REGION`      -> Region your namespace resides. E.g. gcp-us-central1
//! * `TURBOPUFFER_NAMESPACE`   -> Namespace within your turbopuffer account
//! * `TURBOPUFFER_API_KEY`     -> The API key should just be the key itself.

use itertools::Itertools;
use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{encoding::Json, logging::LogDestination, web_extensions::headers};

use serde::{Deserialize, Serialize};
use serde_json::json;

/// Just like the `fine-foods-embeddings` example, but with
/// the embedding included.
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

momento_functions::post!(index_document);
fn index_document(Json(documents): Json<Vec<Document>>) -> WebResult<WebResponse> {
    let headers = headers();
    setup_logging(&headers)?;

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

    // Runtime environment variables - pass with -E flag when deploying
    let turbopuffer_api_key = format!(
        "Bearer {}",
        std::env::var("TURBOPUFFER_API_KEY").unwrap_or_default()
    );
    let turbopuffer_region = std::env::var("TURBOPUFFER_REGION").unwrap_or_default();
    let turbopuffer_namespace = std::env::var("TURBOPUFFER_NAMESPACE").unwrap_or_default();
    let turbopuffer_endpoint = format!(
        "https://{turbopuffer_region}.turbopuffer.com/v2/namespaces/{turbopuffer_namespace}"
    );

    let chunks = documents.into_iter().chunks(2000);
    for chunk in chunks.into_iter() {
        let chunk: Vec<Document> = chunk.collect();
        let result = momento_functions_host::http::post(
            &turbopuffer_endpoint,
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
                "upsert_rows": chunk,
                "distance_metric": "cosine_distance",
            }),
        );
        log::debug!("Turbopuffer response: {result:?}");
        match result {
            Ok(response) => {
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

fn setup_logging(_headers: &[(String, String)]) -> WebResult<()> {
    momento_functions_log::configure_logs([LogDestination::topic("turbopuffer-index").into()])?;
    Ok(())
}
