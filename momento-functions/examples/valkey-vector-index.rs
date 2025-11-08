use momento_functions::{WebError, WebResponse, WebResult};
use momento_functions_host::{
    encoding::Json,
    logging::LogDestination,
    redis::{Command, RedisClient, RedisValue},
    web_extensions::headers,
};

use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

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

momento_functions::post!(index_document);
fn index_document(Json(body): Json<Request>) -> WebResult<WebResponse> {
    let headers = headers();
    setup_logging(&headers)?;

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

    const CONNECTION_STRING: &str = env!("REDIS_CONNECTION_STRING");
    log::debug!("Connecting to Redis at {CONNECTION_STRING}");
    let redis = RedisClient::new(CONNECTION_STRING);
    if let Err(e) = ensure_index_exists(dimensions, &redis) {
        log::error!("Failed to ensure index exists: {e:?}");
        return e;
    }

    let length = documents.len();
    let commands = convert_into_hset_commands(documents);
    match redis.pipe(commands) {
        Ok(_) => Ok(WebResponse::new().with_status(200).with_body(Json(json!({
            "message": "Documents indexed successfully",
            "indexed_count": length,
        })))?),
        Err(e) => Ok(WebResponse::new().with_status(500).with_body(Json(json!({
            "message": e.to_string(),
        })))?),
    }
}

fn convert_into_hset_commands(documents: Vec<Document>) -> Vec<Command> {
    let mut commands: Vec<Command> = Vec::with_capacity(documents.len());
    for document in documents {
        let mut command = Command::builder().any("HSET").arg(document.id);
        for (key, value) in document.fields {
            command = command.arg(key).arg(value);
        }
        let embedding: Vec<_> = document
            .embedding
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        command = command.arg("vector").arg(embedding);
        commands.push(command.build());
    }
    commands
}

fn ensure_index_exists(
    dimensions: usize,
    redis: &RedisClient,
) -> Result<(), Result<WebResponse, WebError>> {
    match redis.pipe(vec![
        Command::builder()
            .any("FT.INFO")
            .arg("document_index")
            .build(),
    ]) {
        Ok(mut definition) => match definition.next() {
            Some(definition) => {
                if let RedisValue::Bulk(definition_stream) = definition {
                    let values: Vec<RedisValue> = definition_stream.collect();
                    log::debug!("Redis index info: {values:?}");
                } else {
                    log::warn!("Unexpected response type from Redis FT.INFO: {definition:?}");
                    return Err(WebResponse::new()
                        .with_status(500)
                        .with_body("Unexpected response from Redis FT.INFO")
                        .map_err(WebError::from));
                }
            }
            None => {
                log::info!("redis did not return an answer for index info");
                return Err(WebResponse::new()
                    .with_status(400)
                    .with_body("redis is not answering index info")
                    .map_err(WebError::from));
            }
        },
        Err(e) => {
            log::info!("index does not exist, creating a new one: {e:?}");
            match redis.pipe(vec![
                Command::builder()
                    .any("FT.CREATE")
                    .arg("document_index")
                    .arg("SCHEMA")
                    .arg("vector")
                    .arg("VECTOR HNSW")
                    .arg("6")
                    .arg("TYPE FLOAT32")
                    .arg("DIM")
                    .arg(dimensions.to_string())
                    .arg("DISTANCE_METRIC COSINE")
                    .build(),
            ]) {
                Ok(_) => log::info!("Index created successfully"),
                Err(e) => {
                    log::error!("Failed to create index: {e:?}");
                    return Err(WebResponse::new()
                        .with_status(500)
                        .with_body("Failed to create index")
                        .map_err(WebError::from));
                }
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------
// | Utility functions for convenience
// ------------------------------------------------------

fn setup_logging(_headers: &[(String, String)]) -> WebResult<()> {
    momento_functions_log::configure_logs([LogDestination::topic("valkey-vector-index").into()])?;
    Ok(())
}
