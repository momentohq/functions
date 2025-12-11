use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{
    aws::auth::AwsCredentialsProvider, build_environment_aws_credentials, encoding::Json,
    logging::LogDestination,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct Request {
    bucket: String,
    key: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct Response {
    message: String,
}

#[derive(Debug, Serialize)]
struct MyStructure {
    a_number: u32,
    a_string: String,
}

momento_functions::post!(s3_put);
fn s3_put(Json(request): Json<Request>) -> WebResult<WebResponse> {
    momento_functions_log::configure_logs([LogDestination::topic("s3-put").into()])?;
    let client = momento_functions_host::aws::s3::S3Client::new(&AwsCredentialsProvider::new(
        "us-west-2",
        build_environment_aws_credentials!(),
    )?);

    log::info!(
        "putting object to s3, bucket: {}; key {}; size of value {}",
        &request.bucket,
        &request.key,
        &request.value.len()
    );

    if let Err(e) = client.put(
        &request.bucket,
        &request.key,
        Json(MyStructure {
            a_number: 42,
            a_string: request.value,
        }),
    ) {
        let response = Response {
            message: format!("Failed to put object {e:?}"),
        };
        return Ok(WebResponse::new()
            .with_status(500)
            .with_body(Json(response))?);
    }

    let response = Response {
        message: "Successfully put object".to_string(),
    };
    Ok(WebResponse::new()
        .with_status(200)
        .with_body(Json(response))?)
}
