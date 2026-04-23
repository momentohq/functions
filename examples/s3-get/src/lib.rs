use momento_functions_aws_auth::{Authorization, IamRole, provider};
use momento_functions_aws_s3::S3Client;
use momento_functions_bytes::encoding::Json;
use momento_functions_guest_web::{WebEnvironment, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct Request {
    role_arn: String,
    bucket: String,
    key: String,
}

#[derive(Debug, Serialize)]
struct Response {
    message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    metadata: Vec<(String, String)>,
}

#[derive(Debug, Deserialize)]
struct MyStructure {
    a_number: u32,
    a_string: String,
}

invoke!(s3_get);
fn s3_get(Json(request): Json<Request>) -> WebResult<WebResponse> {
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
        "getting object from s3, bucket {}, key {}",
        &request.bucket,
        &request.key
    );
    let (response, metadata): (MyStructure, Vec<(String, String)>) =
        match client.get(&request.bucket, &request.key) {
            Ok(Some(r)) => {
                let Json(value) = r.body;
                (value, r.metadata)
            }
            Ok(None) => {
                return Ok(WebResponse::new()
                    .with_status(404)
                    .with_body(Json(Response {
                        message: "Not found".to_string(),
                        metadata: Vec::new(),
                    }))?);
            }
            Err(e) => {
                return Ok(WebResponse::new()
                    .with_status(500)
                    .with_body(Json(Response {
                        message: format!("Failed to get object from S3: {e:?}"),
                        metadata: Vec::new(),
                    }))?);
            }
        };

    log::info!(
        "found response with a_number: {}, a_string: {}, metadata entries: {}",
        response.a_number,
        response.a_string,
        metadata.len()
    );
    Ok(WebResponse::new()
        .with_status(200)
        .with_body(Json(Response {
            message: format!("Found: {response:?}"),
            metadata,
        }))?)
}
