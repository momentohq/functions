#[derive(serde::Deserialize)]
struct Request {
    _name: String,
}

momento_functions::spawn!(spawned, Request);
fn spawned(_payload: Request) -> FunctionResult<()> {
    Ok(())
}
