use momento_functions::{WebError, WebResponse, WebResult};
use momento_functions_host::{
    encoding::Json,
    logging::{LogConfiguration, LogDestination},
    redis::{Command, RedisClusterClient, RedisValue},
    web_extensions::FunctionEnvironment,
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

    let connection_string = std::env::var("CLUSTER_NAME").unwrap_or_default();
    log::debug!("Connecting to Redis at {connection_string}");
    let redis = RedisClusterClient::new_momento_managed(connection_string);
    log::info!("created client");
    if let Err(e) = ensure_index_exists(dimensions, &redis) {
        log::error!("Failed to ensure index exists: {e:?}");
        return e;
    }

    let length = documents.len();
    let commands = convert_into_hset_commands(documents);
    for command in commands {
        match redis.command(command) {
            Ok(_) => {}
            Err(e) => {
                return Ok(WebResponse::new().with_status(500).with_body(Json(json!({
                    "message": e.to_string(),
                })))?);
            }
        }
    }
    Ok(WebResponse::new().with_status(200).with_body(Json(json!({
        "message": "Documents indexed successfully",
        "indexed_count": length,
    })))?)
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
    redis: &RedisClusterClient,
) -> Result<(), Result<WebResponse, WebError>> {
    log::info!("checking if index exists");
    match redis.command(
        Command::builder()
            .any("FT.INFO")
            .arg("document_index")
            .build(),
    ) {
        Ok(definition) => {
            let definition: RedisValue = definition.into();
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
        Err(e) => {
            log::info!("index does not exist, creating a new one: {e:?}");
            match redis.command(
                Command::builder()
                    .any("FT.CREATE")
                    .arg("document_index")
                    .arg("SCHEMA")
                    .arg("vector")
                    .arg("VECTOR")
                    .arg("HNSW")
                    .arg("6")
                    .arg("TYPE")
                    .arg("FLOAT32")
                    .arg("DIM")
                    .arg(dimensions.to_string())
                    .arg("DISTANCE_METRIC")
                    .arg("COSINE")
                    .build(),
            ) {
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

fn setup_logging() -> WebResult<()> {
    let function_env = FunctionEnvironment::get_function_environment();
    momento_functions_log::configure_logs([LogConfiguration::new(LogDestination::Topic {
        topic: function_env.function_name().to_string(),
    })
    .with_log_level(log::LevelFilter::Debug)])?;
    Ok(())
}
