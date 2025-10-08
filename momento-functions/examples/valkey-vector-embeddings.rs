use log::LevelFilter;
use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{
    encoding::Json,
    logging::{ConfigureLoggingInput, LogDestination},
    web_extensions::headers,
};

use serde::{Deserialize, Serialize};

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
fn get_document_embeddings(Json(body): Json<Request>) -> WebResult<WebResponse> {
    let headers = headers();
    setup_logging(&headers)?;

    let Request { documents } = body;
    let data = get_embeddings(documents)?;

    Ok(WebResponse::new().with_status(200).with_body(Json(data))?)
}

fn get_embeddings(mut documents: Vec<String>) -> WebResult<Vec<EmbeddingData>> {
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
    const OPENAI_API_KEY: &str = env!("OPENAI_API_KEY");
    let result = momento_functions_host::http::post(
        "https://api.openai.com/v1/embeddings",
        [
            (
                "authorization".to_string(),
                format!("Bearer {OPENAI_API_KEY}"),
            ),
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
            vec![ConfigureLoggingInput::new(LogDestination::Topic {
                topic: "valkey-vector-embeddings".to_string(),
            })],
        )?;
    }
    Ok(())
}
