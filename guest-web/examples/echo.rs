//! An example Function that echoes the request body back in the response.
//!
//! Note that this example uses the [momento_functions_bytes::Data] type,
//! which allows the actual payload to stay out of WASM memory. This is
//! ideal for large messages, or data that you only want to pass through.
//! Keeping the data on the host makes your Function run much more quickly.
//!
//! Invoke this Function with any body and it will echo it back as an `application/octet-stream`.

use momento_functions_bytes::Data;
use momento_functions_guest_web::invoke;

invoke!(echo);
fn echo(request: Data) -> Data {
    request
}
