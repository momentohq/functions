use log::LevelFilter;
use momento_functions_host::encoding::Json;
use momento_functions_log::LogMode;

#[derive(serde::Deserialize, Debug)]
struct Request {
    name: String,
}

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

momento_functions::post!(greet);
fn greet(Json(request): Json<Request>) -> FunctionResult<Json<Response>> {
    momento_functions_log::configure_logging(
        LevelFilter::Info,
        LogMode::Topic {
            topic: "logging-example".to_string(),
        },
    )?;

    log::info!("Received request: {request:?}");

    Ok(Json(Response {
        message: format!("Hello, {}!", request.name),
    }))
}
