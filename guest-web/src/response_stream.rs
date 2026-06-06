use momento_functions_bytes::Data;

use crate::wit::momento::web_function::web_function_stream;

/// Create a streaming response sink to reply to this function invocation
/// with a stream of formatted sse events.
pub fn sse_streaming_response() -> SseStreamingResponse {
    SseStreamingResponse
}

/// Create a streaming response sink to reply to this function invocation
/// with a stream of formatted sse events.
/// Make sure you set the right content type and whatnot.
pub fn sse_streaming_response_with_headers(
    code: u16,
    headers: Vec<(String, String)>,
) -> Result<SseStreamingResponse, String> {
    web_function_stream::send_response_start(
        code,
        &headers.into_iter().map(Into::into).collect::<Vec<_>>(),
    )?;

    Ok(SseStreamingResponse)
}

/// Create a streaming response sink to reply to this function invocation
/// with a stream of raw bytes.
///
/// Returns a debug string if there was an error creating the streaming response.
pub fn raw_streaming_response(
    code: u16,
    headers: Vec<(String, String)>,
) -> Result<RawStreamingResponse, String> {
    web_function_stream::send_response_start(
        code,
        &headers.into_iter().map(Into::into).collect::<Vec<_>>(),
    )?;

    Ok(RawStreamingResponse)
}

/// A response sink for responding with a stream of formatted sse events.
pub struct SseStreamingResponse;

impl SseStreamingResponse {
    /// Send an SSE event to the client. The event will be formatted according to the SSE specification: https://html.spec.whatwg.org/multipage/server-sent-events.html
    ///
    /// On error, a debug string is returned.
    pub fn send(&mut self, event: SseEvent) -> Result<(), String> {
        let SseEvent {
            event,
            event_id,
            data,
        } = event;
        web_function_stream::send_sse(event.as_deref(), event_id.as_deref(), data.map(Into::into))
    }
}

/// An SSE event to be sent to the client, formatted according to the SSE specification: https://html.spec.whatwg.org/multipage/server-sent-events.html
pub struct SseEvent {
    event: Option<String>,
    event_id: Option<String>,
    data: Option<Data>,
}
impl SseEvent {
    /// Create a new SSE event with an event type
    pub fn from_event(event: impl Into<String>) -> Self {
        Self {
            event: Some(event.into()),
            event_id: None,
            data: None,
        }
    }

    /// Create a new SSE event with an event ID
    pub fn from_event_id(event: impl Into<String>) -> Self {
        Self {
            event: None,
            event_id: Some(event.into()),
            data: None,
        }
    }

    /// Create a new SSE event with data.
    pub fn from_data(data: impl Into<Data>) -> Self {
        Self {
            event: None,
            event_id: None,
            data: Some(data.into()),
        }
    }

    /// Set the event type of this SSE event.
    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Set the event ID of this SSE event.
    pub fn with_event_id(mut self, event_id: impl Into<String>) -> Self {
        self.event_id = Some(event_id.into());
        self
    }

    /// Set the data of this SSE event.
    pub fn with_data(mut self, data: impl Into<Data>) -> Self {
        self.data = Some(data.into());
        self
    }
}

/// A response sink for responding with a stream of literal raw bytes.
pub struct RawStreamingResponse;

impl RawStreamingResponse {
    /// Send a chunk of bytes to the client. The bytes will be sent as-is, without any formatting.
    ///
    /// On error, a debug string is returned.
    pub fn send(&mut self, chunk: impl Into<Data>) -> Result<(), String> {
        web_function_stream::send_data(chunk.into().into())
    }
}
