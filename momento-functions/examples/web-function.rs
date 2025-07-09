use momento_functions::WebResponse;

momento_functions::post!(ping);
fn ping(_payload: Vec<u8>) -> WebResponse {
    "pong".into()
}
