//! Indexes a batch of documents into a Momento-managed Valkey cluster as
//! HSET hashes with a `vector` field. The index is created on demand. Pass
//! the cluster name via `CLUSTER_NAME` when deploying.

use std::collections::HashMap;

use momento_functions_bytes::encoding::Json;
use momento_functions_guest_web::{WebEnvironment, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogConfiguration, LogDestination, configure_logs};
use momento_functions_valkey::{Command, Value, get_managed_cluster_client};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
struct Request {
    documents: Vec<Document>,
}
#[derive(Deserialize, Debug)]
struct Document {
    id: String,
    embedding: Vec<f32>,
    fields: HashMap<String, String>,
}

invoke!(index_document);
fn index_document(Json(body): Json<Request>) -> WebResult<WebResponse> {
    setup_logging()?;

    let Request { documents } = body;
    log::debug!("indexing {} documents", documents.len());

    let dimensions = match documents.first() {
        Some(doc) => doc.embedding.len(),
        None => {
            log::warn!("No documents provided for indexing.");
            return Ok(WebResponse::new()
                .with_status(400)
                .with_body("No documents provided")?);
        }
    };

    let cluster_name = std::env::var("CLUSTER_NAME").unwrap_or_default();
    log::debug!("Connecting to managed Valkey cluster {cluster_name}");
    let client = get_managed_cluster_client(&cluster_name);
    log::info!("created client");

    ensure_index_exists(dimensions, &client)?;

    let length = documents.len();
    for document in documents {
        let mut cmd = Command::builder("HSET");
        cmd.argument(document.id);
        for (key, value) in document.fields {
            cmd.argument(key).argument(value);
        }
        let embedding: Vec<u8> = document
            .embedding
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        cmd.argument("vector").argument(embedding);
        client.command(cmd)?;
    }

    Ok(WebResponse::new().with_status(200).with_body(Json(json!({
        "message": "Documents indexed successfully",
        "indexed_count": length,
    })))?)
}

fn ensure_index_exists(
    dimensions: usize,
    client: &momento_functions_valkey::ClusterClient,
) -> WebResult<()> {
    let mut info = Command::builder("FT.INFO");
    info.argument("document_index");

    match client.command(info) {
        Ok(Value::Bulk(_)) | Ok(Value::Ok) | Ok(Value::SimpleString(_)) => {
            log::debug!("index already exists");
        }
        Ok(_) | Err(_) => {
            log::info!("index does not exist or returned unexpected shape, creating it");
            create_index(dimensions, client)?;
        }
    }
    Ok(())
}

fn create_index(
    dimensions: usize,
    client: &momento_functions_valkey::ClusterClient,
) -> WebResult<()> {
    let mut create = Command::builder("FT.CREATE");
    create
        .argument("document_index")
        .argument("SCHEMA")
        .argument("vector")
        .argument("VECTOR")
        .argument("HNSW")
        .argument("6")
        .argument("TYPE")
        .argument("FLOAT32")
        .argument("DIM")
        .argument(dimensions.to_string())
        .argument("DISTANCE_METRIC")
        .argument("COSINE");
    client.command(create)?;
    log::info!("Index created successfully");
    Ok(())
}

fn setup_logging() -> WebResult<()> {
    let env = WebEnvironment::load();
    configure_logs([
        LogConfiguration::new(LogDestination::topic(env.function_name()))
            .with_log_level(log::LevelFilter::Debug),
    ])?;
    Ok(())
}
