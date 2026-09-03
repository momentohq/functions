//! S3 Express Spawn Worker - Async S3 Upload
//!
//! This is a spawn function (not a web function) that handles async S3 uploads.
//! It's triggered by the `s3-express-upload` function via `spawn()`.
//!
//! Key difference from web functions:
//! - Uses `momento_functions::spawn!()` macro instead of `post!()`
//! - Has no return value (fire-and-forget)
//! - Receives payload directly, not wrapped in `Json()`
//!
//! ## Environment Variables (set via -E flag at deploy time)
//! - `S3_BUCKET`: S3 Express bucket name (e.g., "my-bucket--usw2-az1--x-s3")
//! - `AWS_REGION`: AWS region (e.g., "us-west-2")
//! - `AWS_ACCESS_KEY_ID`: AWS access key
//! - `AWS_SECRET_ACCESS_KEY`: AWS secret key
//!
//! ## Metrics Logged to Topic
//! This worker logs JSON metrics to the "s3-express-metrics" topic including:
//! - Total latency from original request arrival to S3 upload complete
//! - S3 PUT time
//! - SigV4 signing time

use momento_functions_host::logging::LogDestination;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use hmac::{Hmac, Mac};
use std::time::Instant;
use base64::Engine;

type HmacSha256 = Hmac<Sha256>;

const LOG_TOPIC: &str = "s3-express-metrics";

#[derive(Deserialize)]
struct WorkerRequest {
    key: String,
    content_type: String,
    data: String, // base64-encoded
    arrival_timestamp_ms: u64,
}

#[derive(Serialize)]
struct UploadMetrics {
    key: String,
    size_bytes: usize,
    arrival_timestamp_ms: u64,
    s3_complete_timestamp_ms: u64,
    total_latency_ms: u64,
    s3_put_ms: u64,
    decode_ms: u64,
    signing_ms: u64,
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

// Note: spawn!() macro, not post!()
momento_functions::spawn!(s3_upload_worker, WorkerRequest);

fn s3_upload_worker(request: WorkerRequest) {
    // Configure logging to topic
    let _ = momento_functions_log::configure_logs([LogDestination::topic(LOG_TOPIC).into()]);

    log::info!("Worker started for key: {}", request.key);

    let access_key = match get_env("AWS_ACCESS_KEY_ID") {
        Ok(v) => v,
        Err(e) => {
            log::error!("Missing AWS credentials: {}", e);
            return;
        }
    };
    let secret_key = match get_env("AWS_SECRET_ACCESS_KEY") {
        Ok(v) => v,
        Err(e) => {
            log::error!("Missing AWS credentials: {}", e);
            return;
        }
    };
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-west-2".to_string());
    let bucket = match get_env("S3_BUCKET") {
        Ok(v) => v,
        Err(e) => {
            log::error!("Missing S3_BUCKET: {}", e);
            return;
        }
    };

    // Decode base64 data
    let start_decode = Instant::now();
    let data = match base64::engine::general_purpose::STANDARD.decode(&request.data) {
        Ok(d) => d,
        Err(e) => {
            log::error!("Failed to decode base64: {}", e);
            return;
        }
    };
    let decode_ms = start_decode.elapsed().as_millis() as u64;
    let size_bytes = data.len();

    // Upload to S3 Express
    let start_s3 = Instant::now();
    let signing_ms;

    match s3_put_with_timing(
        &bucket,
        &request.key,
        &request.content_type,
        &data,
        &access_key,
        &secret_key,
        &region,
    ) {
        Ok(s_ms) => {
            signing_ms = s_ms;
        }
        Err(e) => {
            log::error!("S3 upload failed for key {}: {}", request.key, e);
            return;
        }
    }
    let s3_put_ms = start_s3.elapsed().as_millis() as u64;

    // Calculate timestamps and latency
    let s3_complete_timestamp_ms = current_timestamp_ms();
    let total_latency_ms = s3_complete_timestamp_ms.saturating_sub(request.arrival_timestamp_ms);

    // Log metrics as JSON
    let metrics = UploadMetrics {
        key: request.key,
        size_bytes,
        arrival_timestamp_ms: request.arrival_timestamp_ms,
        s3_complete_timestamp_ms,
        total_latency_ms,
        s3_put_ms,
        decode_ms,
        signing_ms,
    };

    log::info!("{}", serde_json::to_string(&metrics).unwrap_or_default());
}

// ============================================================================
// S3 Express PUT with AWS SigV4 Signing
// ============================================================================

fn extract_az_id(bucket: &str) -> Option<&str> {
    let parts: Vec<&str> = bucket.split("--").collect();
    if parts.len() >= 2 {
        Some(parts[1])
    } else {
        None
    }
}

fn s3_put_with_timing(
    bucket: &str,
    key: &str,
    content_type: &str,
    data: &[u8],
    access_key: &str,
    secret_key: &str,
    region: &str,
) -> Result<u64, String> {
    let start_signing = Instant::now();

    // S3 Express endpoint format
    let az_id = extract_az_id(bucket).unwrap_or("usw2-az1");
    let host = format!("{}.s3express-{}.{}.amazonaws.com", bucket, az_id, region);
    let url = format!("https://{}/{}", host, key);

    let amz_date = get_amz_date();
    let date_stamp = &amz_date[0..8];

    let method = "PUT";
    let canonical_uri = format!("/{}", key);
    let payload_hash = hex::encode(Sha256::digest(data));

    // Build canonical request
    let canonical_headers = format!(
        "content-type:{}\nhost:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
        content_type, host, payload_hash, amz_date
    );
    let signed_headers = "content-type;host;x-amz-content-sha256;x-amz-date";

    let canonical_request = format!(
        "{}\n{}\n\n{}\n{}\n{}",
        method, canonical_uri, canonical_headers, signed_headers, payload_hash
    );

    // String to sign (note: s3express for S3 Express, not s3)
    let algorithm = "AWS4-HMAC-SHA256";
    let service = "s3express";
    let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, region, service);
    let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));

    let string_to_sign = format!(
        "{}\n{}\n{}\n{}",
        algorithm, amz_date, credential_scope, canonical_request_hash
    );

    // Calculate signature
    let signing_key = get_signature_key(secret_key, date_stamp, region, service);
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

    let authorization_header = format!(
        "{} Credential={}/{}, SignedHeaders={}, Signature={}",
        algorithm, access_key, credential_scope, signed_headers, signature
    );

    let signing_ms = start_signing.elapsed().as_millis() as u64;

    // Make the request
    let response = momento_functions_host::http::put(
        &url,
        [
            ("Host".to_string(), host),
            ("Content-Type".to_string(), content_type.to_string()),
            ("x-amz-date".to_string(), amz_date),
            ("x-amz-content-sha256".to_string(), payload_hash),
            ("Authorization".to_string(), authorization_header),
        ],
        data.to_vec(),
    );

    match response {
        Ok(resp) if resp.status >= 200 && resp.status < 300 => Ok(signing_ms),
        Ok(resp) => Err(format!(
            "S3 returned {}: {}",
            resp.status,
            String::from_utf8_lossy(&resp.body)
        )),
        Err(e) => Err(format!("HTTP error: {:?}", e)),
    }
}

// ============================================================================
// AWS SigV4 Helpers
// ============================================================================

fn get_amz_date() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    let (year, month, day) = days_to_ymd(days_since_epoch);
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut remaining_days = days as i64;
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }
    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for days_in_month in days_in_months.iter() {
        if remaining_days < *days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }
    (year, month, remaining_days as u64 + 1)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn get_signature_key(secret_key: &str, date_stamp: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(
        format!("AWS4{}", secret_key).as_bytes(),
        date_stamp.as_bytes(),
    );
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}
