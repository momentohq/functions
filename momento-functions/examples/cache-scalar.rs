//! Demonstrates cache scalar operations: get, set, set_if, and delete.
//! You'll want to subscribe to the function's topic so you can see these log values!

use momento_functions::WebResult;
use momento_functions_host::{
    cache::{self, SetIfCondition},
    logging::LogDestination,
    web_extensions::FunctionEnvironment,
};
use std::time::Duration;

momento_functions::post!(demo);
fn demo(_payload: Vec<u8>) -> WebResult<&'static str> {
    let function_env = FunctionEnvironment::get_function_environment();
    momento_functions_log::configure_logs([
        LogDestination::topic(function_env.function_name()).into()
    ])?;

    let ttl = Duration::from_secs(60);

    // Basic set and get
    log::info!("Setting 'greeting' to 'Hello, World!'");
    cache::set("greeting", b"Hello, World!".to_vec(), ttl)?;

    let value: Option<Vec<u8>> = cache::get("greeting")?;
    log::info!(
        "Got 'greeting': {:?}",
        value.map(|v| String::from_utf8_lossy(&v).into_owned())
    );

    // Set multiple items
    log::info!("Setting 'counter' to '0'");
    cache::set("counter", b"0".to_vec(), ttl)?;

    log::info!("Setting 'temp-data' to 'temporary'");
    cache::set("temp-data", b"temporary".to_vec(), ttl)?;

    // Delete
    log::info!("Deleting 'temp-data'");
    cache::delete("temp-data")?;

    let deleted_value: Option<Vec<u8>> = cache::get("temp-data")?;
    log::info!("Got 'temp-data' after delete: {:?}", deleted_value);

    // set_if with Absent (only set if key doesn't exist)
    log::info!("set_if 'new-key' with condition Absent (key doesn't exist yet)");
    let result = cache::set_if(
        "new-key",
        b"first-value".to_vec(),
        ttl,
        SetIfCondition::Absent,
    )?;
    log::info!("Result: {:?}", result);

    log::info!("set_if 'new-key' with condition Absent again (key now exists)");
    let result = cache::set_if(
        "new-key",
        b"second-value".to_vec(),
        ttl,
        SetIfCondition::Absent,
    )?;
    log::info!(
        "Result: {:?} (not stored because key already exists)",
        result
    );

    // set_if with NotEqual (only set if current value differs from condition)
    log::info!(
        "set_if 'counter' to '1' with condition NotEqual('999') (will succeed, value is '0')"
    );
    let result = cache::set_if(
        "counter",
        b"1".to_vec(),
        ttl,
        SetIfCondition::NotEqual(b"999".to_vec()),
    )?;
    log::info!("Result: {:?}", result);

    let value: Option<Vec<u8>> = cache::get("counter")?;
    log::info!(
        "Final 'counter' value: {:?}",
        value.map(|v| String::from_utf8_lossy(&v).into_owned())
    );

    Ok("Cache scalar demo completed! Check logs for details.")
}
