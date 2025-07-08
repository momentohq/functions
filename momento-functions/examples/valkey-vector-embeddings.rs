use log::LevelFilter;
use momento_functions::{WebResponse, WebResponseBuilder};
use momento_functions_host::{encoding::Json, web_extensions::headers};
use momento_functions_log::LogMode;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Deserialize, Debug)]
struct Request {
    documents: Vec<String>,
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

momento_functions::post!(get_document_embeddings);
fn get_document_embeddings(Json(body): Json<Request>) -> Result<impl WebResponse, Box<dyn Error>> {
    let headers = headers();
    setup_logging(&headers)?;

    let Request { documents } = body;
    let data = get_embeddings(documents)?;

    Ok(WebResponseBuilder::new()
        .status_code(200)
        .payload(Json(data))?)
}

fn get_embeddings(mut documents: Vec<String>) -> Result<Vec<EmbeddingData>, Box<dyn Error>> {
    log::debug!("getting embeddings for document with content: {documents:?}");
    for document in &mut documents {
        if document.contains("\n") {
            // openai guide currently says to replace newlines with spaces. This, then, must be how you get the cargo to come.
            // https://platform.openai.com/docs/guides/embeddings
            *document = document.replace("\n", " ");
        }
    }
    // compile-time environment variable.
    // Runtime environment variable secrets will be supported in the future.
    const OPENAPI_KEY: &str = env!("OPENAI_KEY");
    let result = momento_functions_host::http::post(
        "https://api.openai.com/v1/embeddings",
        [
            ("authorization".to_string(), format!("Bearer {OPENAPI_KEY}")),
            ("content-type".to_string(), "application/json".to_string()),
        ],
        // 1536 float32 for text-embedding-3-small
        serde_json::json!({
            "model": "text-embedding-3-small",
            "encoding_format": "float",
            "input": documents,
        })
        .to_string(),
    );
    log::debug!("OpenAI response: {result:?}");
    let mut response = result?;
    let Json(EmbeddingResponse { mut data }) = response.extract()?;
    data.sort_by_key(|d| d.index);
    Ok(data)
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
                topic: "valkey-vector-embeddings".to_string(),
            },
        )?;
    }
    Ok(())
}
