use momento_functions::WebResult;
use momento_functions_host::{
    encoding::Json,
    logging::{LogConfiguration, LogDestination},
    web_extensions::invocation_id,
};

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

momento_functions::post!(greet);
/// For this example, we demonstrate sending some logs to a CloudWatch logs group
/// passed in from the request, as well as some more advanced log setup.
/// This example presumes an IAM role with proper permissions have been set up.
/// Reach out to `support@momentohq.com` for assisance with how to set up your AWS IAM Role.
fn greet(Json(request): Json<Request>) -> WebResult<Json<Response>> {
    momento_functions_log::configure_logs([
        // Here is a dedicated topic that only has system logs, useful in case you only want to monitor
        // logs sent by Momento
        LogConfiguration::new(LogDestination::topic("logging-extended-system-logs"))
            .with_log_level(log::LevelFilter::Off)
            .with_system_log_level(log::LevelFilter::Debug),
        // Here is a standard topic log that will capture application DEBUG logs and up, as well as any errors
        // sent by Momento.
        LogConfiguration::new(LogDestination::topic("logging-extended"))
            .with_log_level(log::LevelFilter::Debug)
            .with_system_log_level(log::LevelFilter::Error),
        // For our CW log destination, we'll let the default INFO be used for both application and
        // system logs.
        LogConfiguration::new(LogDestination::cloudwatch(
            request.iam_role_arn.clone(),
            request.log_group_name.clone(),
        )),
    ])?;

    let invocation_id = invocation_id();
    log::debug!("Logging a debug message");
    log::info!("invocation ID is {invocation_id}");
    log::info!("Received request: {request:?}");
    log::error!("Logging an error message");

    Ok(Json(Response {
        message: format!("Hello, {}!", request.name),
    }))
}
