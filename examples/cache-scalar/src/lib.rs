//! Demonstrates cache scalar operations: get, set, set_if, and delete.

use std::time::Duration;

use momento_functions_bytes::Data;
use momento_functions_cache::{self as cache, SetIfCondition};
use momento_functions_guest_web::{WebResult, invoke};

invoke!(demo);
fn demo(_payload: Data) -> WebResult<&'static str> {
    let ttl = Duration::from_secs(60);

    // Set 'greeting' to 'Hello, World!'
    cache::set("greeting", b"Hello, World!".to_vec(), ttl)?;

    // Get 'greeting'
    let _value: Option<Vec<u8>> = cache::get("greeting")?;

    // Set 'counter' to '0'
    cache::set("counter", b"0".to_vec(), ttl)?;

    // Set 'temp-data' to 'temporary'
    cache::set("temp-data", b"temporary".to_vec(), ttl)?;

    // Delete 'temp-data'
    cache::delete("temp-data")?;

    // Get 'temp-data' after delete (should be None)
    let _deleted_value: Option<Vec<u8>> = cache::get("temp-data")?;

    // set_if 'new-key' with condition Absent (key doesn't exist yet, will be Stored)
    let _result = cache::set_if(
        "new-key",
        b"first-value".to_vec(),
        ttl,
        SetIfCondition::Absent,
    )?;

    // set_if 'new-key' with condition Absent again (key now exists, will be NotStored)
    let _result = cache::set_if(
        "new-key",
        b"second-value".to_vec(),
        ttl,
        SetIfCondition::Absent,
    )?;

    // set_if 'counter' to '1' with condition NotEqual('999') (will succeed, value is '0')
    let _result = cache::set_if(
        "counter",
        b"1".to_vec(),
        ttl,
        SetIfCondition::NotEqual(b"999".to_vec().into()),
    )?;

    // Get final 'counter' value
    let _value: Option<Vec<u8>> = cache::get("counter")?;

    Ok("Cache scalar demo completed!")
}
