use momento_functions_host::encoding::Json;

#[derive(serde::Deserialize)]
struct Request {
    name: String,
}

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

momento_functions::post!(greet);
fn greet(Json(request): Json<Request>) -> FunctionResult<Json<Response>> {
    Ok(Json(Response {
        message: format!("Hello, {}!", request.name),
    }))
}
