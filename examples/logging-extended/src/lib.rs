//! Demonstrates more advanced log configuration: one system-only topic, one
//! function-log topic filtered at DEBUG, and a CloudWatch log group. Assumes
//! an IAM role has already been configured — reach out to
//! `support@momentohq.com` for assistance with IAM role setup.

use momento_functions_bytes::encoding::Json;
use momento_functions_guest_web::{WebEnvironment, WebResult, invoke};
use momento_functions_host_log::{LogConfiguration, LogDestination, configure_logs};

#[derive(serde::Deserialize, Debug)]
struct Request {
    iam_role_arn: String,
    log_group_name: String,
    name: String,
}

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

invoke!(greet);
fn greet(Json(request): Json<Request>) -> WebResult<Json<Response>> {
    let env = WebEnvironment::load();
    configure_logs([
        // Dedicated topic that only receives system logs.
        LogConfiguration::new(LogDestination::topic(format!(
            "{}-system-logs",
            env.function_name()
        )))
        .with_log_level(log::LevelFilter::Off)
        .with_system_log_level(log::LevelFilter::Debug),
        // Standard topic for DEBUG+ application logs and ERROR+ system logs.
        LogConfiguration::new(LogDestination::topic(env.function_name()))
            .with_log_level(log::LevelFilter::Debug)
            .with_system_log_level(log::LevelFilter::Error),
        // CloudWatch destination at the default INFO level for both streams.
        LogConfiguration::new(LogDestination::cloudwatch(
            request.iam_role_arn.clone(),
            request.log_group_name.clone(),
        )),
    ])?;

    log::debug!("Logging a debug message");
    log::info!("Received request: {request:?}");
    log::error!("Logging an error message");

    Ok(Json(Response {
        message: format!("Hello, {}!", request.name),
    }))
}
