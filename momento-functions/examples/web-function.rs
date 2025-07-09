momento_functions::post!(ping);
fn ping(_payload: Vec<u8>) -> &'static str {
    "pong"
}
