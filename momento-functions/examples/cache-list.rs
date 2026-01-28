//! Simple example showcasing how you can use Momento's List API within a function.
//!
//! To see this "working," you'll want to subscribe to the log topic to see results.
//! You can do so via:
//! ```
//! momento topic subscribe --cache-name <your-cache> cache-list
//! ```
use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{cache, encoding::Json, logging::LogDestination};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
struct Response {
    message: String,
}

momento_functions::post!(cache_list);
fn cache_list(_payload: Vec<u8>) -> WebResult<WebResponse> {
    momento_functions_log::configure_logs([LogDestination::topic("cache-list").into()])?;

    let list_name = "my_list";
    let ttl = Duration::from_secs(15);

    // Push some values to the front
    log::info!("Pushing 'first' to front of list");
    let length = cache::list_push_front(
        list_name,
        b"first".to_vec(),
        ttl,
        // You can set this to 'true' to refresh the ttl if you need to
        false,
        // truncate list to 100 items from the back, which should cover our single value
        100,
    )?;
    log::info!("List length after push_front: {}", length);

    log::info!("Pushing 'second' to front of list");
    let length = cache::list_push_front(list_name, b"second".to_vec(), ttl, true, 100)?;
    log::info!("List length after push_front: {}", length);

    // Push some values to the back
    log::info!("Pushing 'third' to back of list");
    let length = cache::list_push_back(list_name, b"third".to_vec(), ttl, true, 100)?;
    log::info!("List length after push_back: {}", length);

    log::info!("Pushing 'fourth' to back of list");
    let length = cache::list_push_back(list_name, b"fourth".to_vec(), ttl, true, 100)?;
    log::info!("List length after push_back: {}", length);

    // Pop from the front
    log::info!("Popping from front of list");
    match cache::list_pop_front::<Vec<u8>>(list_name)? {
        Some((value, length)) => {
            let value_str = String::from_utf8_lossy(&value);
            log::info!(
                "Popped '{}' from front, list length now: {}",
                value_str,
                length
            );
        }
        None => log::warn!("List was empty"),
    }

    // Pop from the back
    log::info!("Popping from back of list");
    match cache::list_pop_back::<Vec<u8>>(list_name)? {
        Some((value, length)) => {
            let value_str = String::from_utf8_lossy(&value);
            log::info!(
                "Popped '{}' from back, list length now: {}",
                value_str,
                length
            );
        }
        None => log::warn!("List was empty"),
    }

    Ok(WebResponse::new().with_body(Json(Response {
        message: "Done! Check your logs for the results".to_string(),
    }))?)
}
