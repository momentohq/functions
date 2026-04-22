//! Indexes news articles into Turbopuffer. For each batch of documents we
//! query OpenAI to generate embeddings, then upsert them into the configured
//! Turbopuffer namespace.
//!
//! Required env vars: `OPENAI_API_KEY`, `TURBOPUFFER_REGION`,
//! `TURBOPUFFER_NAMESPACE`, `TURBOPUFFER_API_KEY`.

use itertools::Itertools;
use momento_functions_bytes::encoding::{Extract, Json};
use momento_functions_guest_web::{WebEnvironment, WebError, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_http::{Request as HttpRequest, invoke as http_invoke};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tiktoken_rs::{CoreBPE, cl100k_base_singleton};

const OPENAI_URL: &str = "https://api.openai.com/v1/embeddings";
const EMBEDDING_MODEL: &str = "text-embedding-3-small";
const MAX_TOKENS: usize = 8192; // OpenAI's limit for text-embedding-3-small

#[derive(Deserialize, Debug)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}
#[derive(Deserialize, Serialize, Debug)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize, Serialize, Debug)]
struct DocumentMetadata {
    title: String,
    link: String,
    authors: Vec<String>,
    language: String,
    description: String,
    feed: String,
}

#[derive(Deserialize, Debug)]
struct DocumentInput {
    id: String,
    #[serde(alias = "metadata")]
    document_metadata: DocumentMetadata,
    page_content: String,
}

impl DocumentInput {
    fn into_turbopuffer_document(self, vector: Vec<f32>) -> TurbopufferDocument {
        TurbopufferDocument {
            id: self.id,
            page_content: self.page_content,
            vector,
            metadata_title: self.document_metadata.title,
            metadata_link: self.document_metadata.link,
            metadata_authors: self.document_metadata.authors,
            metadata_language: self.document_metadata.language,
            metadata_description: self.document_metadata.description,
            metadata_feed: self.document_metadata.feed,
        }
    }
}

#[derive(Serialize, Debug)]
struct TurbopufferDocument {
    id: String,
    page_content: String,
    vector: Vec<f32>,
    #[serde(rename = "metadata$title")]
    metadata_title: String,
    #[serde(rename = "metadata$link")]
    metadata_link: String,
    #[serde(rename = "metadata$authors")]
    metadata_authors: Vec<String>,
    #[serde(rename = "metadata$language")]
    metadata_language: String,
    #[serde(rename = "metadata$description")]
    metadata_description: String,
    #[serde(rename = "metadata$feed")]
    metadata_feed: String,
}

invoke!(index_documents);
fn index_documents(Json(documents): Json<Vec<DocumentInput>>) -> WebResult<WebResponse> {
    setup_logging()?;

    let bpe = cl100k_base_singleton();

    if documents.is_empty() {
        log::warn!("No documents provided for indexing.");
        return Ok(WebResponse::new()
            .with_status(400)
            .with_body("No documents provided")?);
    }

    let turbopuffer_api_key = format!(
        "Bearer {}",
        std::env::var("TURBOPUFFER_API_KEY").unwrap_or_default()
    );
    let turbopuffer_region = std::env::var("TURBOPUFFER_REGION").unwrap_or_default();
    let turbopuffer_namespace = std::env::var("TURBOPUFFER_NAMESPACE").unwrap_or_default();
    let turbopuffer_endpoint = format!(
        "https://{turbopuffer_region}.turbopuffer.com/v2/namespaces/{turbopuffer_namespace}"
    );
    let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    // 100 is a reasonable batch size for OpenAI's embeddings endpoint.
    for chunk in &documents.into_iter().chunks(100) {
        let chunk: Vec<DocumentInput> = chunk.collect();
        let page_contents = chunk.iter().map(|d| d.page_content.clone()).collect();
        let embedding_data = get_embeddings(page_contents, &openai_api_key, bpe)?;

        let mut to_upsert = Vec::with_capacity(chunk.len());
        for (document, embedding) in chunk.into_iter().zip(embedding_data) {
            to_upsert.push(document.into_turbopuffer_document(embedding.embedding));
        }

        index_in_turbopuffer(to_upsert, &turbopuffer_endpoint, &turbopuffer_api_key)?;
    }

    Ok(WebResponse::new()
        .with_status(200)
        .with_body(json!({ "message": "Documents indexed successfully" }).to_string())?)
}

fn index_in_turbopuffer(
    rows: Vec<TurbopufferDocument>,
    endpoint: &str,
    api_key: &str,
) -> WebResult<()> {
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
                    "upsert_rows": rows,
                    "distance_metric": "cosine_distance",
                })
                .to_string(),
            ),
    )?;
    log::debug!("turbopuffer response status: {}", response.status);
    if response.status != 200 {
        let body = response.body.into_bytes();
        let message = format!(
            "Failed to index documents: {}",
            String::from_utf8(body).unwrap_or_default()
        );
        return Err(WebError::message(message));
    }
    Ok(())
}

fn get_embeddings(
    documents: Vec<String>,
    openai_api_key: &str,
    tokenizer: &'static CoreBPE,
) -> WebResult<Vec<EmbeddingData>> {
    log::debug!("getting embeddings for input");

    let mut prepared = Vec::with_capacity(documents.len());
    for document in &documents {
        if document.is_empty() {
            // OpenAI rejects empty inputs.
            prepared.push("no_content".to_string());
            continue;
        }
        let mut tokens = tokenizer.encode_with_special_tokens(document);
        if tokens.len() > MAX_TOKENS {
            log::debug!(
                "token length was {}, truncating to {MAX_TOKENS}",
                tokens.len(),
            );
            tokens.truncate(MAX_TOKENS);
            prepared.push(tokenizer.decode(tokens).map_err(|e| {
                WebError::message(format!(
                    "Failed to convert truncated tokens back to string for OpenAI input: {e:?}"
                ))
            })?);
        } else {
            prepared.push(document.clone());
        }
    }

    let response = http_invoke(
        HttpRequest::new(OPENAI_URL, "POST")
            .with_header("authorization", format!("Bearer {openai_api_key}"))
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "model": EMBEDDING_MODEL,
                    "encoding_format": "float",
                    "input": prepared,
                })
                .to_string(),
            ),
    )?;
    if response.status != 200 {
        let bytes = response.body.into_bytes();
        log::error!(
            "OpenAI returned non-200: {}",
            String::from_utf8_lossy(&bytes)
        );
        return Err(WebError::message("OpenAI failed to return embeddings"));
    }
    let Json(EmbeddingResponse { mut data }) = Json::<EmbeddingResponse>::extract(response.body)?;
    data.sort_by_key(|d| d.index);
    Ok(data)
}

fn setup_logging() -> WebResult<()> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;
    Ok(())
}
