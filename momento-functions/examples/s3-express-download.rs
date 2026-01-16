//! S3 Express Download with Cache-First Logic
//!
//! This example demonstrates a cache-first read pattern:
//! 1. Check Momento Cache first (fast, single-digit ms)
//! 2. On cache miss, fetch from S3 Express and cache the result
//!
//! ## Environment Variables (set via -E flag at deploy time)
//! - `S3_BUCKET`: S3 Express bucket name (e.g., "my-bucket--usw2-az1--x-s3")
//! - `AWS_REGION`: AWS region (e.g., "us-west-2")
//! - `AWS_ACCESS_KEY_ID`: AWS access key
//! - `AWS_SECRET_ACCESS_KEY`: AWS secret key
//!
//! ## Related Examples
//! - `s3-express-upload.rs`: Upload with async S3 durability
//! - `s3-express-worker.rs`: The spawn worker that uploads to S3

use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{cache, encoding::Json};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use hmac::{Hmac, Mac};
use std::time::{Duration, Instant};
use base64::Engine;

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
struct DownloadRequest {
    key: String,
}

#[derive(Serialize)]
struct DownloadResponse {
    key: String,
    data: String, // base64-encoded
    size_bytes: usize,
    source: String, // "cache" or "s3"
    server_time_ms: u64,
    cache_lookup_ms: Option<u64>,
    s3_fetch_ms: Option<u64>,
    encode_ms: u64,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    details: Option<String>,
}

fn get_env(name: &str) -> Result<String, String> {
    let value = std::env::var(name).unwrap_or_default();
    if value.is_empty() {
        return Err(format!("Missing {name} env var"));
    }
    Ok(value)
}

momento_functions::post!(download);

fn download(Json(request): Json<DownloadRequest>) -> WebResult<WebResponse> {
    let start_total = Instant::now();

    let bucket = match get_env("S3_BUCKET") {
        Ok(v) => v,
        Err(e) => return error_response(500, &e, None),
    };

    let cache_key = format!("s3:{}/{}", bucket, request.key);

    // Try cache first
    let start_cache = Instant::now();
    match cache::get::<Vec<u8>>(&cache_key) {
        Ok(Some(data)) => {
            let cache_lookup_ms = start_cache.elapsed().as_millis() as u64;

            // Cache hit - encode and return
            let start_encode = Instant::now();
            let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
            let encode_ms = start_encode.elapsed().as_millis() as u64;

            let server_time_ms = start_total.elapsed().as_millis() as u64;

            return Ok(WebResponse::new().with_status(200).with_body(Json(DownloadResponse {
                key: request.key,
                data: encoded,
                size_bytes: data.len(),
                source: "cache".to_string(),
                server_time_ms,
                cache_lookup_ms: Some(cache_lookup_ms),
                s3_fetch_ms: None,
                encode_ms,
            }))?);
        }
        Ok(None) => { /* Cache miss, fall through to S3 */ }
        Err(e) => {
            // Log but continue to S3
            log::warn!("Cache lookup failed: {:?}", e);
        }
    }

    // Cache miss - fetch from S3 Express
    let access_key = match get_env("AWS_ACCESS_KEY_ID") {
        Ok(v) => v,
        Err(e) => return error_response(500, &e, None),
    };
    let secret_key = match get_env("AWS_SECRET_ACCESS_KEY") {
        Ok(v) => v,
        Err(e) => return error_response(500, &e, None),
    };
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-west-2".to_string());

    let start_s3 = Instant::now();
    let data = match s3_get(&bucket, &request.key, &access_key, &secret_key, &region) {
        Ok(d) => d,
        Err(e) => return error_response(502, "S3 fetch failed", Some(&e)),
    };
    let s3_fetch_ms = start_s3.elapsed().as_millis() as u64;

    // Cache for next time (ignore errors)
    let _ = cache::set(&cache_key, data.clone(), Duration::from_secs(60));

    // Encode response
    let start_encode = Instant::now();
    let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
    let encode_ms = start_encode.elapsed().as_millis() as u64;

    let server_time_ms = start_total.elapsed().as_millis() as u64;

    Ok(WebResponse::new().with_status(200).with_body(Json(DownloadResponse {
        key: request.key,
        data: encoded,
        size_bytes: data.len(),
        source: "s3".to_string(),
        server_time_ms,
        cache_lookup_ms: None,
        s3_fetch_ms: Some(s3_fetch_ms),
        encode_ms,
    }))?)
}

fn error_response(status: u16, error: &str, details: Option<&str>) -> WebResult<WebResponse> {
    Ok(WebResponse::new().with_status(status).with_body(Json(ErrorResponse {
        error: error.to_string(),
        details: details.map(|s| s.to_string()),
    }))?)
}

// ============================================================================
// S3 Express GET with AWS SigV4 Signing
// ============================================================================

fn extract_az_id(bucket: &str) -> Option<&str> {
    let parts: Vec<&str> = bucket.split("--").collect();
    if parts.len() >= 2 {
        Some(parts[1])
    } else {
        None
    }
}

fn s3_get(
    bucket: &str,
    key: &str,
    access_key: &str,
    secret_key: &str,
    region: &str,
) -> Result<Vec<u8>, String> {
    // S3 Express endpoint format
    let az_id = extract_az_id(bucket).unwrap_or("usw2-az1");
    let host = format!("{}.s3express-{}.{}.amazonaws.com", bucket, az_id, region);
    let url = format!("https://{}/{}", host, key);

    let amz_date = get_amz_date();
    let date_stamp = &amz_date[0..8];

    let method = "GET";
    let canonical_uri = format!("/{}", key);
    let payload_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"; // empty body

    // Build canonical request
    let canonical_headers = format!(
        "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
        host, payload_hash, amz_date
    );
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";

    let canonical_request = format!(
        "{}\n{}\n\n{}\n{}\n{}",
        method, canonical_uri, canonical_headers, signed_headers, payload_hash
    );

    // String to sign
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

    // Make the request
    let response = momento_functions_host::http::get(
        &url,
        [
            ("Host".to_string(), host),
            ("x-amz-date".to_string(), amz_date),
            ("x-amz-content-sha256".to_string(), payload_hash.to_string()),
            ("Authorization".to_string(), authorization_header),
        ],
    );

    match response {
        Ok(resp) if resp.status >= 200 && resp.status < 300 => Ok(resp.body),
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
