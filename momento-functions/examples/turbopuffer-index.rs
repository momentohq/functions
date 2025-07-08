//! Using the embeddings from the fine-foods-embeddings example, this
//! example indexes the documents into a Turbopuffer namespace.
//!
//! You need to provide `TURBOPUFFER_ENDPOINT` and `TURBOPUFFER_API_KEY`
//! environment variables.
//! * The endpoint contains the namespace.
//! * The API key should just be the key itself without any prefix.

use itertools::Itertools;
use log::LevelFilter;
use momento_functions::{WebResponse, WebResponseBuilder};
use momento_functions_host::{encoding::Json, web_extensions::headers};
use momento_functions_log::LogMode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;

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
fn index_document(
    Json(documents): Json<Vec<Document>>,
) -> Result<impl WebResponse, Box<dyn Error>> {
    let headers = headers();
    setup_logging(&headers)?;

    let dimensions = match documents.first() {
        Some(doc) => doc.vector.len(),
        None => {
            log::warn!("No documents provided for indexing.");
            return Ok(WebResponseBuilder::new()
                .status_code(400)
                .payload("No documents provided")?);
        }
    };
    log::debug!(
        "indexing {} documents with {dimensions} dimensions",
        documents.len()
    );

    const TURBOPUFFER_API_KEY: &str = concat!("Bearer ", env!("TURBOPUFFER_API_KEY"));
    const TURBOPUFFER_ENDPOINT: &str = env!("TURBOPUFFER_ENDPOINT");

    let chunks = documents.into_iter().chunks(2000);
    for chunk in chunks.into_iter() {
        let chunk: Vec<Document> = chunk.collect();
        let result = momento_functions_host::http::post(
            TURBOPUFFER_ENDPOINT,
            [
                ("Authorization".to_string(), TURBOPUFFER_API_KEY.to_string()),
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
                    return Ok(WebResponseBuilder::new()
                        .status_code(response.status)
                        .payload(json!({
                            "message": message,
                        }))?);
                }
            }
            Err(e) => {
                log::error!("Failed to index documents: {e:?}");
                return Ok(WebResponseBuilder::new().status_code(500).payload(json!({
                    "message": e.to_string(),
                }))?);
            }
        }
    }

    Ok(WebResponseBuilder::new().status_code(200).payload(json!({
        "message": "Documents indexed successfully",
    }))?)
}

// ------------------------------------------------------
// | Utility functions for convenience
// ------------------------------------------------------

fn setup_logging(headers: &[(String, String)]) -> Result<(), Box<dyn Error>> {
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
                topic: "turbopuffer-index".to_string(),
            },
        )?;
    }
    Ok(())
}
