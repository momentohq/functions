use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{
    aws::{
        auth::AwsCredentialsProvider,
        secrets_manager::{GetSecretValueRequest, SecretsManagerClient},
    },
    build_environment_aws_credentials,
    encoding::Json,
    logging::{LogConfiguration, LogDestination},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Request {
    secret_name: String,
    #[serde(default)]
    use_cache: bool,
}

#[derive(Debug, Serialize)]
struct Response {
    message: String,
}

#[derive(Debug, Deserialize)]
struct MySecret {
    pub token: String,
}

momento_functions::post!(secrets_manager_get);
fn secrets_manager_get(Json(request): Json<Request>) -> WebResult<WebResponse> {
    momento_functions_log::configure_logs([LogConfiguration::new(LogDestination::Topic {
        topic: "secrets-manager-get".to_string(),
    })
    .with_log_level(log::LevelFilter::Debug)])?;

    let credentials =
        AwsCredentialsProvider::new("us-west-2", build_environment_aws_credentials!())?;

    // Create client with or without caching based on request
    let client = if request.use_cache {
        log::info!("Creating Secrets Manager client with 5-minute cache");
        SecretsManagerClient::new_with_cache(&credentials, Duration::from_secs(300))
    } else {
        log::info!("Creating Secrets Manager client without caching");
        SecretsManagerClient::new(&credentials)
    };

    log::info!("Retrieving secret: {}", &request.secret_name);

    let Json(secret): Json<MySecret> =
        match client.get_secret_value(GetSecretValueRequest::new(&request.secret_name)) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to retrieve secret: {:?}", e);
                return Ok(WebResponse::new()
                    .with_status(500)
                    .with_body(Json(Response {
                        message: format!("Failed to retrieve secret: {e:?}"),
                    }))?);
            }
        };

    if !secret.token.is_empty() {
        log::info!("Successfully retrieved secret")
    } else {
        log::error!("The token should have had a value, but we got an empty string instead");
        return Ok(WebResponse::new()
            .with_status(500)
            .with_body(Json(Response {
                message: "Failed to retrieve secret: the value returned was not valid".to_string(),
            }))?);
    }

    Ok(WebResponse::new()
        .with_status(200)
        .with_body(Json(Response {
            message: format!(
                "Secret retrieved successfully! Secret length: {}",
                secret.token.len()
            ),
        }))?)
}
