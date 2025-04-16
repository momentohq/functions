momento_functions::post!(ping);
fn ping(_payload: Vec<u8>) -> FunctionResult<Vec<u8>> {
    Ok(b"pong".to_vec())
}
