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
}

#[derive(Debug, Serialize)]
struct Response {
    message: String,
}

#[derive(Debug, Deserialize)]
struct MyStructure {
    a_number: u32,
    a_string: String,
}

momento_functions::post!(s3_put);
fn s3_put(Json(request): Json<Request>) -> WebResult<WebResponse> {
    momento_functions_log::configure_logs([LogDestination::topic("s3-get").into()])?;
    let client = momento_functions_host::aws::s3::S3Client::new(&AwsCredentialsProvider::new(
        "us-west-2",
        build_environment_aws_credentials!(),
    )?);

    log::info!(
        "getting object from s3, bucket {}, key {}",
        &request.bucket,
        &request.key
    );
    let response: MyStructure = match client.get(&request.bucket, &request.key) {
        Ok(resp) => match resp {
            Some(Json(r)) => r,
            None => {
                return Ok(WebResponse::new()
                    .with_status(404)
                    .with_body(Json(Response {
                        message: "Not found".to_string(),
                    }))?);
            }
        },
        Err(e) => {
            return Ok(WebResponse::new()
                .with_status(500)
                .with_body(Json(Response {
                    message: format!("Failed to get object from S3: {e:?}"),
                }))?);
        }
    };

    log::info!(
        "found response with a_number: {}, a_string: {}",
        response.a_number,
        response.a_string
    );
    Ok(WebResponse::new()
        .with_status(200)
        .with_body(Json(Response {
            message: format!("Found: {response:?}"),
        }))?)
}
