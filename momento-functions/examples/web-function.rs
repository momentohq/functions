use std::error::Error;

momento_functions::post!(ping);
fn ping(_payload: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(b"pong".to_vec())
}
