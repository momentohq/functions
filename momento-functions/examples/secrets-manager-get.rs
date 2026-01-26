use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{
    aws::{
        auth::{AwsCredentialsProvider, Credentials},
        secrets_manager::{GetSecretValueRequest, SecretsManagerClient},
    },
    encoding::Json,
    logging::{LogConfiguration, LogDestination},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Request {
    secret_name: String,
}

#[derive(Debug, Serialize)]
struct Response {
    message: String,
}

/// In this example, imagine a secret has been stored in Secrets Manager as
/// ```json
/// {
///     "token": "my super secret JWT for my service"
/// }
/// ```
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

    // In this example, you would provide the full path to your IAM Role Arn that gives Momento permission to
    // federate into your role and execute the Secrets Manager request on your behalf to retrieve your secret.
    let credentials = AwsCredentialsProvider::new(
        "us-west-2",
        Credentials::Federated {
            role_arn: "<name of your role ARN>".to_string(),
        },
    )?;

    let client = SecretsManagerClient::new(&credentials);

    // If you'd like your secret securely cached within your funciton's context, how long will you allow it to remain
    // stale before it is retrieved from Secrets Manager again? This is compared against the first time the secret is stored,
    // regardless of the function invocation.
    let allowed_staleness = Duration::from_mins(5);

    log::info!("Retrieving secret: {}", &request.secret_name);

    let Json(secret): Json<MySecret> = match client.get_secret_value(
        GetSecretValueRequest::new(&request.secret_name),
        allowed_staleness,
    ) {
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

    // Obviously we don't want to leak secrets in this example! We can return the length of the secret instead
    Ok(WebResponse::new()
        .with_status(200)
        .with_body(Json(Response {
            message: format!(
                "Secret retrieved successfully! Secret length: {}",
                secret.token.len()
            ),
        }))?)
}
