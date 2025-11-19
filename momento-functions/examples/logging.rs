use momento_functions::WebResult;
use momento_functions_host::{encoding::Json, logging::LogDestination};

#[derive(serde::Deserialize, Debug)]
struct Request {
    name: String,
}

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

momento_functions::post!(greet);
fn greet(Json(request): Json<Request>) -> WebResult<Json<Response>> {
    // Demonstrates a simple topic destination. This uses the default log level of INFO
    // for both system and function logs.
    momento_functions_log::configure_logs([LogDestination::topic("logging-example").into()])?;

    log::info!("Received request: {request:?}");

    Ok(Json(Response {
        message: format!("Hello, {}!", request.name),
    }))
}
