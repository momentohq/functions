use momento_functions::WebResult;
use momento_functions_host::{
    encoding::Json, logging::LogDestination, web_extensions::FunctionEnvironment,
};

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
    let function_env = FunctionEnvironment::get_function_environment();
    // Demonstrates a simple topic destination. This uses the default log level of INFO
    // for both system and function logs.
    momento_functions_log::configure_logs([
        LogDestination::topic(function_env.function_name()).into()
    ])?;

    log::info!("Received request: {request:?}");

    Ok(Json(Response {
        message: format!("Hello, {}!", request.name),
    }))
}
