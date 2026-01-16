use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{encoding::Json, logging::LogDestination, web_extensions::headers};

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
    // Runtime environment variable - pass with -E flag when deploying
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

fn setup_logging(_headers: &[(String, String)]) -> WebResult<()> {
    momento_functions_log::configure_logs([
        LogDestination::topic("valkey-vector-embeddings").into()
    ])?;
    Ok(())
}
