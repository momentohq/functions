use log::LevelFilter;
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
    momento_functions_log::configure_logs(
        LevelFilter::Info,
        [LogDestination::topic("logging-example")],
    )?;

    log::info!("Received request: {request:?}");

    Ok(Json(Response {
        message: format!("Hello, {}!", request.name),
    }))
}
