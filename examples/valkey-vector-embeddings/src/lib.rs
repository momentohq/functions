//! Request embeddings from OpenAI for a batch of documents. Serves as the
//! embedding producer used by the other valkey-vector-* examples.

use momento_functions_bytes::encoding::{Extract, Json};
use momento_functions_guest_web::{WebEnvironment, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_http::{Request as HttpRequest, invoke as http_invoke};
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

invoke!(get_document_embeddings);
fn get_document_embeddings(Json(body): Json<Request>) -> WebResult<WebResponse> {
    setup_logging()?;

    let data = get_embeddings(body.documents)?;
    Ok(WebResponse::new().with_status(200).with_body(Json(data))?)
}

fn get_embeddings(mut documents: Vec<String>) -> WebResult<Vec<EmbeddingData>> {
    log::debug!("getting embeddings for document with content: {documents:?}");
    for document in &mut documents {
        if document.contains('\n') {
            // OpenAI's embeddings guide recommends replacing newlines with spaces.
            // https://platform.openai.com/docs/guides/embeddings
            *document = document.replace('\n', " ");
        }
    }

    let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let response = http_invoke(
        HttpRequest::new("https://api.openai.com/v1/embeddings", "POST")
            .with_header("authorization", format!("Bearer {openai_api_key}"))
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "model": "text-embedding-3-small",
                    "encoding_format": "float",
                    "input": documents,
                })
                .to_string(),
            ),
    )?;
    log::debug!("OpenAI response status: {}", response.status);

    let Json(EmbeddingResponse { mut data }) = Json::<EmbeddingResponse>::extract(response.body)?;
    data.sort_by_key(|d| d.index);
    Ok(data)
}

fn setup_logging() -> WebResult<()> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;
    Ok(())
}
