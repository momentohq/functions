use momento_functions_aws_auth::{Authorization, IamRole, provider};
use momento_functions_aws_s3::{PutObjectRequest, S3Client};
use momento_functions_bytes::encoding::Json;
use momento_functions_guest_web::{WebEnvironment, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct Request {
    role_arn: String,
    bucket: String,
    key: String,
    value: String,
    #[serde(default)]
    metadata: Vec<(String, String)>,
}

#[derive(Debug, Serialize)]
struct Response {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    etag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expiration: Option<String>,
}

#[derive(Debug, Serialize)]
struct MyStructure {
    a_number: u32,
    a_string: String,
}

invoke!(s3_put);
fn s3_put(Json(request): Json<Request>) -> WebResult<WebResponse> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;

    let credentials = provider(
        &Authorization::Federated(IamRole {
            role_arn: request.role_arn,
        }),
        "us-west-2",
    )?;
    let client = S3Client::new(&credentials);

    log::info!(
        "putting object to s3, bucket: {}; key {}; size of value {}",
        &request.bucket,
        &request.key,
        &request.value.len()
    );

    let put_response = match client.put(
        PutObjectRequest::new(
            &request.bucket,
            &request.key,
            Json(MyStructure {
                a_number: 42,
                a_string: request.value,
            }),
        )
        .with_metadata(request.metadata),
    ) {
        Ok(response) => response,
        Err(e) => {
            return Ok(WebResponse::new()
                .with_status(500)
                .with_body(Json(Response {
                    message: format!("Failed to put object {e:?}"),
                    etag: None,
                    version_id: None,
                    expiration: None,
                }))?);
        }
    };

    log::info!(
        "put object succeeded, etag: {:?}, version_id: {:?}",
        put_response.etag,
        put_response.version_id,
    );

    Ok(WebResponse::new()
        .with_status(200)
        .with_body(Json(Response {
            message: "Successfully put object".to_string(),
            etag: put_response.etag,
            version_id: put_response.version_id,
            expiration: put_response.expiration,
        }))?)
}
