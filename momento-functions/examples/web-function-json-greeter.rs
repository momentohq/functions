use momento_functions_host::encoding::Json;
use std::error::Error;

#[derive(serde::Deserialize)]
struct Request {
    name: String,
}

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

momento_functions::post!(greet);
fn greet(Json(request): Json<Request>) -> Result<Json<Response>, Box<dyn Error>> {
    Ok(Json(Response {
        message: format!("Hello, {}!", request.name),
    }))
}
