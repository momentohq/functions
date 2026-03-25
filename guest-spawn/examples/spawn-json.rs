use momento_functions_bytes::encoding::Json;
use momento_functions_guest_spawn::spawn;

#[derive(serde::Deserialize)]
struct Request {
    _name: String,
}

spawn!(triggered);
fn triggered(Json(_req): Json<Request>) -> Result<(), String> {
    Ok(())
}
