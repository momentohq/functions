use log::LevelFilter;
use momento_functions_host::encoding::Json;
use momento_functions_log::LogMode;
use std::error::Error;

#[derive(serde::Deserialize, Debug)]
struct Request {
    name: String,
}

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

momento_functions::post!(greet);
fn greet(Json(request): Json<Request>) -> Result<Json<Response>, Box<dyn Error>> {
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
