use log::LevelFilter;
use momento_functions::WebResult;
use momento_functions_host::{encoding::Json, logging::LogDestination};

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
/// passed in from the request. This example presumes an IAM role with proper
/// permissions have been set up. Reach out to `support@momentohq.com` for assisance
/// with how to set up your AWS IAM Role.
fn greet(Json(request): Json<Request>) -> WebResult<Json<Response>> {
    momento_functions_log::configure_logs(
        LevelFilter::Info,
        [LogDestination::cloudwatch(
            request.iam_role_arn.clone(),
            request.log_group_name.clone(),
        )],
    )?;

    log::info!("Received request: {request:?}");
    log::info!("Logging a line");
    log::info!("Logging another line, the caller was {}", request.name);

    Ok(Json(Response {
        message: format!("Hello, {}!", request.name),
    }))
}
