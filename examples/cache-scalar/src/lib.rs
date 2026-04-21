//! Demonstrates cache scalar operations: get, set, set_if, and delete.
//! Subscribe to the function's topic to watch the emitted log lines.

use std::time::Duration;

use momento_functions_bytes::Data;
use momento_functions_cache::{self as cache, ConditionalSetResult, SetIfCondition};
use momento_functions_guest_web::{WebEnvironment, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};

fn describe<T>(result: &ConditionalSetResult<T>) -> &'static str {
    match result {
        ConditionalSetResult::Stored(_) => "Stored",
        ConditionalSetResult::NotStored => "NotStored",
    }
}

invoke!(demo);
fn demo(_payload: Data) -> WebResult<&'static str> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;

    let ttl = Duration::from_secs(60);

    log::info!("Setting 'greeting' to 'Hello, World!'");
    cache::set("greeting", b"Hello, World!".to_vec(), ttl)?;

    let value: Option<Vec<u8>> = cache::get("greeting")?;
    log::info!(
        "Got 'greeting': {:?}",
        value.map(|v| String::from_utf8_lossy(&v).into_owned())
    );

    log::info!("Setting 'counter' to '0'");
    cache::set("counter", b"0".to_vec(), ttl)?;

    log::info!("Setting 'temp-data' to 'temporary'");
    cache::set("temp-data", b"temporary".to_vec(), ttl)?;

    log::info!("Deleting 'temp-data'");
    cache::delete("temp-data")?;

    let deleted_value: Option<Vec<u8>> = cache::get("temp-data")?;
    log::info!("Got 'temp-data' after delete: {deleted_value:?}");

    log::info!("set_if 'new-key' with condition Absent (key doesn't exist yet)");
    let result = cache::set_if(
        "new-key",
        b"first-value".to_vec(),
        ttl,
        SetIfCondition::Absent,
    )?;
    log::info!("Result: {}", describe(&result));

    log::info!("set_if 'new-key' with condition Absent again (key now exists)");
    let result = cache::set_if(
        "new-key",
        b"second-value".to_vec(),
        ttl,
        SetIfCondition::Absent,
    )?;
    log::info!(
        "Result: {} (not stored because key already exists)",
        describe(&result)
    );

    log::info!(
        "set_if 'counter' to '1' with condition NotEqual('999') (will succeed, value is '0')"
    );
    let result = cache::set_if(
        "counter",
        b"1".to_vec(),
        ttl,
        SetIfCondition::NotEqual(b"999".to_vec().into()),
    )?;
    log::info!("Result: {}", describe(&result));

    let value: Option<Vec<u8>> = cache::get("counter")?;
    log::info!(
        "Final 'counter' value: {:?}",
        value.map(|v| String::from_utf8_lossy(&v).into_owned())
    );

    Ok("Cache scalar demo completed! Check logs for details.")
}
