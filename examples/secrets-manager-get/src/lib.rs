//! Demonstrates fetching a secret from AWS Secrets Manager via Momento's
//! host-managed AWS channel. Returns only the length of the secret to avoid
//! leaking the value.
//!
//! Imagine the secret has been stored as:
//! ```json
//! { "token": "my super secret JWT for my service" }
//! ```

use std::time::Duration;

use momento_functions_aws_auth::{Authorization, IamRole, provider};
use momento_functions_aws_secrets_manager::{GetSecretValueRequest, SecretsManagerClient};
use momento_functions_bytes::encoding::Json;
use momento_functions_guest_web::{WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogConfiguration, LogDestination, configure_logs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct Request {
    secret_name: String,
}

#[derive(Debug, Serialize)]
struct Response {
    message: String,
}

#[derive(Debug, Deserialize)]
struct MySecret {
    pub token: String,
}

invoke!(secrets_manager_get);
fn secrets_manager_get(Json(request): Json<Request>) -> WebResult<WebResponse> {
    configure_logs([
        LogConfiguration::new(LogDestination::topic("secrets-manager-get"))
            .with_log_level(log::LevelFilter::Debug),
    ])?;

    // Provide the IAM role ARN that Momento will federate into to read the secret.
    let credentials = provider(
        &Authorization::Federated(IamRole {
            role_arn: "<name of your role ARN>".to_string(),
        }),
        "us-west-2",
    )?;

    let client = SecretsManagerClient::new(&credentials);

    // How long the in-context secret cache may be reused before refetching.
    let allowed_staleness = Duration::from_secs(5 * 60);

    log::info!("Retrieving secret: {}", &request.secret_name);

    let Json(secret): Json<MySecret> = match client.get_secret_value(
        GetSecretValueRequest::new(&request.secret_name),
        allowed_staleness,
    ) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to retrieve secret: {e:?}");
            return Ok(WebResponse::new()
                .with_status(500)
                .with_body(Json(Response {
                    message: format!("Failed to retrieve secret: {e:?}"),
                }))?);
        }
    };

    if secret.token.is_empty() {
        log::error!("The token should have had a value, but we got an empty string instead");
        return Ok(WebResponse::new()
            .with_status(500)
            .with_body(Json(Response {
                message: "Failed to retrieve secret: the value returned was not valid".to_string(),
            }))?);
    }

    log::info!("Successfully retrieved secret");
    Ok(WebResponse::new()
        .with_status(200)
        .with_body(Json(Response {
            message: format!(
                "Secret retrieved successfully! Secret length: {}",
                secret.token.len()
            ),
        }))?)
}
