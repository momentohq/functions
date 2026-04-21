use momento_functions_bytes::encoding::Json;
use momento_functions_guest_web::{WebEnvironment, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};

#[derive(serde::Deserialize, Debug)]
struct Request {
    name: String,
}

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

invoke!(greet);
fn greet(Json(request): Json<Request>) -> WebResult<Json<Response>> {
    let env = WebEnvironment::load();
    // Simple topic destination. Uses the default log level of INFO for both
    // system and function logs.
    configure_logs([LogDestination::topic(env.function_name()).into()])?;

    log::info!("Received request: {request:?}");

    Ok(Json(Response {
        message: format!("Hello, {}!", request.name),
    }))
}
