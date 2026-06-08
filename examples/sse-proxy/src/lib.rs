//! Server-Sent Events proxy.
//!
//! Reads an SSE stream from an upstream server. Relays events back to the
//! caller after repackaging them.
//! The caller's request headers are forwarded to the upstream untouched,
//! except for `x-momento-authorization`.
//!
//! Required env var: `UPSTREAM_SSE_URL` - the upstream SSE endpoint to proxy.
//!
//! ```bash
//! curl -N https://api.cache.$MOMENTO_CELL_HOSTNAME/functions/$MOMENTO_CACHE_NAME/sse-proxy \
//!   -H "x-momento-authorization: $MOMENTO_API_KEY"
//! ```

use momento_functions_bytes::Data;
use momento_functions_guest_web::{
    SseEvent, WebEnvironment, WebError, WebResult, invoke, sse_streaming_response_with_headers,
};
use momento_functions_http::sse::SseStream;
use momento_functions_http::{Request, invoke as http_invoke};

invoke!(proxy);
fn proxy(request_body: Data) -> WebResult<()> {
    let upstream_url = std::env::var("UPSTREAM_SSE_URL")
        .map_err(|_| WebError::message("UPSTREAM_SSE_URL is not configured"))?;

    let request = WebEnvironment::load();

    // Forward every caller header to the upstream except Momento's auth header.
    let forwarded_headers: Vec<(String, String)> = request
        .headers()
        .iter()
        .filter(|(name, _)| !name.eq_ignore_ascii_case("x-momento-authorization"))
        // There seems to be an issue with anthropic's gzip encoding, which is what I'm experimenting with.
        .filter(|(name, _)| !name.eq_ignore_ascii_case("accept-encoding")) // let the upstream decide how to encode the response
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();

    // Open the upstream stream, mirroring the caller's method, headers, and body.
    let upstream = http_invoke(
        Request::new(upstream_url, request.http_method())
            .with_headers(forwarded_headers)
            .with_body(request_body),
    )?;

    // Surface upstream failures before we commit to an SSE response, so the caller
    // gets a normal HTTP error rather than a half-open event stream.
    if !(200..300).contains(&upstream.status) {
        return Err(WebError::message(format!(
            "upstream returned {} before streaming could begin",
            upstream.status
        )));
    }

    let mut response = sse_streaming_response_with_headers(upstream.status, upstream.headers)
        .map_err(WebError::message)?;

    // Iterate the upstream events as they arrive and relay each one downstream.
    for upstream_result in SseStream::from_data(upstream.body) {
        let upstream_event =
            upstream_result.map_err(|e| WebError::message(format!("upstream SSE error: {e}")))?;

        // Preserve whichever fields the upstream event carried.
        let mut forwarded = SseEvent::from_data(upstream_event.data_raw().unwrap_or_default());
        if let Some(name) = upstream_event.event() {
            forwarded = forwarded.with_event(name);
        }
        if let Some(id) = upstream_event.event_id() {
            forwarded = forwarded.with_event_id(id);
        }

        response
            .send(forwarded)
            .map_err(|e| WebError::message(format!("failed to relay SSE event: {e}")))?;
    }

    Ok(())
}
