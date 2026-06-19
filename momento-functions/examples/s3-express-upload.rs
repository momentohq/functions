//! S3 Express Upload with Async Durability via Spawn
//!
//! This example demonstrates a write-through cache pattern:
//! 1. Write to Momento Cache synchronously (fast, for immediate reads)
//! 2. Spawn an async worker to upload to S3 Express (durable storage)
//!
//! The client gets a fast response while durability happens in the background.
//!
//! ## Environment Variables (set via -E flag at deploy time)
//! - `S3_BUCKET`: S3 Express bucket name (e.g., "my-bucket--usw2-az1--x-s3")
//!
//! ## Related Examples
//! - `s3-express-worker.rs`: The spawn worker that uploads to S3
//! - `s3-express-download.rs`: Cache-first download with S3 fallback

use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{cache, encoding::Json, logging::LogDestination, spawn};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const LOG_TOPIC: &str = "s3-express-metrics";

#[derive(Deserialize)]
struct UploadRequest {
    key: String,
    content_type: String,
    data: String, // base64-encoded
}

#[derive(Serialize)]
struct UploadResponse {
    accepted: bool,
    cache_key: String,
    size_bytes: usize,
    server_time_ms: u64,
    cache_set_ms: u64,
    decode_ms: u64,
    message: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    details: Option<String>,
}

// Payload sent to the spawn worker
#[derive(Serialize)]
struct WorkerPayload {
    key: String,
    content_type: String,
    data: String, // base64-encoded (pass through)
    arrival_timestamp_ms: u64,
}

fn get_env(name: &str) -> Result<String, String> {
    let value = std::env::var(name).unwrap_or_default();
    if value.is_empty() {
        return Err(format!("Missing {name} env var"));
    }
    Ok(value)
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

momento_functions::post!(upload);

fn upload(Json(request): Json<UploadRequest>) -> WebResult<WebResponse> {
    // Configure logging to topic
    momento_functions_log::configure_logs([LogDestination::topic(LOG_TOPIC).into()])?;

    let arrival_timestamp_ms = current_timestamp_ms();
    let start_total = Instant::now();

    log::info!("Upload request received for key: {}", request.key);

    let bucket = match get_env("S3_BUCKET") {
        Ok(v) => v,
        Err(e) => return error_response(500, &e, None),
    };

    // Decode base64 data for cache (and to get size)
    let start_decode = Instant::now();
    let data = match base64::engine::general_purpose::STANDARD.decode(&request.data) {
        Ok(d) => d,
        Err(e) => return error_response(400, "Invalid base64 data", Some(&e.to_string())),
    };
    let decode_ms = start_decode.elapsed().as_millis() as u64;
    let size_bytes = data.len();

    // Write to Momento Cache immediately (synchronous, fast)
    let start_cache = Instant::now();
    let cache_key = format!("s3:{}/{}", bucket, request.key);
    if let Err(e) = cache::set(&cache_key, data, Duration::from_secs(60)) {
        return error_response(502, "Cache write failed", Some(&format!("{:?}", e)));
    }
    let cache_set_ms = start_cache.elapsed().as_millis() as u64;

    // Spawn async S3 upload worker (fire-and-forget)
    let worker_payload = WorkerPayload {
        key: request.key.clone(),
        content_type: request.content_type,
        data: request.data, // pass through base64 encoded
        arrival_timestamp_ms,
    };

    log::info!("Spawning worker for key: {}", request.key);
    match spawn("s3-express-worker", Json(worker_payload)) {
        Ok(_) => log::info!("Worker spawned successfully"),
        Err(e) => {
            log::error!("Failed to spawn worker: {:?}", e);
            return error_response(502, "Failed to spawn S3 worker", Some(&format!("{:?}", e)));
        }
    }

    let server_time_ms = start_total.elapsed().as_millis() as u64;

    // Return 202 Accepted - S3 upload is happening in background
    Ok(WebResponse::new().with_status(202).with_body(Json(UploadResponse {
        accepted: true,
        cache_key,
        size_bytes,
        server_time_ms,
        cache_set_ms,
        decode_ms,
        message: "Data cached, S3 upload in progress".to_string(),
    }))?)
}

fn error_response(status: u16, error: &str, details: Option<&str>) -> WebResult<WebResponse> {
    Ok(WebResponse::new().with_status(status).with_body(Json(ErrorResponse {
        error: error.to_string(),
        details: details.map(|s| s.to_string()),
    }))?)
}
