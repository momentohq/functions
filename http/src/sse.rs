//! Support for parsing Server-Sent Events (SSE) streams from HTTP responses.
//! This module helps you consume SSE streams from servers. It uses the `Data`
//! type from `momento_functions_bytes` to read the stream, so it can be used
//! directly with the body of an HTTP response.
//! Make sure your server is actually sending SSE streams (for example, with
//! the `Content-Type: text/event-stream` header) before using this, as it
//! assumes the stream is formatted according to the SSE spec: https://html.spec.whatwg.org/multipage/server-sent-events.html

use momento_functions_bytes::{Data, encoding::Extract};

/// An event from an SSE stream
pub struct SseEvent {
    /// The event name, if provided by the server.
    event: Option<Vec<u8>>,
    /// The event ID, if provided by the server.
    id: Option<Vec<u8>>,
    /// The event data.
    data: Option<Vec<u8>>,
}
impl std::fmt::Debug for SseEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SseEvent")
            .field("event", &self.event.as_deref().map(String::from_utf8_lossy))
            .field("id", &self.id.as_deref().map(String::from_utf8_lossy))
            .field("data", &self.data.as_deref().map(String::from_utf8_lossy))
            .finish()
    }
}

impl SseEvent {
    /// Get the event name, if provided by the server.
    pub fn event_raw(&self) -> Option<&[u8]> {
        self.event.as_deref()
    }

    /// Get the event name as a string, if it is valid UTF-8.
    pub fn event(&self) -> Option<&str> {
        self.event
            .as_deref()
            .and_then(|e| std::str::from_utf8(e).ok())
    }

    /// Get the event ID, if provided by the server.
    pub fn event_id_raw(&self) -> Option<&[u8]> {
        self.id.as_deref()
    }

    /// Get the event ID as a string, if it is valid UTF-8.
    pub fn event_id(&self) -> Option<&str> {
        self.id
            .as_deref()
            .and_then(|id| std::str::from_utf8(id).ok())
    }

    /// Get the event data.
    pub fn data_raw(&self) -> Option<&[u8]> {
        self.data.as_deref()
    }

    /// Get the event data as a string, if it is valid UTF-8.
    pub fn data(&self) -> Option<&str> {
        self.data
            .as_deref()
            .and_then(|d| std::str::from_utf8(d).ok())
    }

    /// Consume the event into deserialized data.
    ///
    /// ```rust,no_run
    /// use momento_functions_bytes::encoding::Extract;
    /// use momento_functions_http::sse::SseEvent;
    ///
    /// fn extract_event(event: SseEvent) -> Result<String, std::string::FromUtf8Error> {
    ///     event.extract()
    /// }
    /// ```
    pub fn extract<T: Extract>(self) -> Result<T, T::Error> {
        T::extract(self.data.unwrap_or_default().into())
    }
}

/// Non-value response from reading an SSE stream
#[derive(thiserror::Error, Debug)]
pub enum SseStatus {
    #[error("The SSE stream has ended.")]
    Closed,
    #[error("Bad SSE stream: {0}")]
    ProtocolError(String),
}

/// A stream of SSE events from a server, as defined by the SSE specification: https://html.spec.whatwg.org/multipage/server-sent-events.html
///
/// If your server returns SSE streams, this is a way you can parse the events live as they come in.
///
/// It's called a stream, but it implements `Iterator`. That way you can use it in a `for` loop or with your familiar iterator tools.
/// Just keep in mind that each call to `next()` may block while waiting for the next event to arrive from the server (or for the stream to end).
pub struct SseStream {
    buffer: Vec<u8>,
    data: Data,
}
impl SseStream {
    /// Create a new SseStream from a Data. This is for use with the body of an HTTP response that is an SSE stream.
    pub fn from_data(data: Data) -> Self {
        Self {
            buffer: Vec::new(),
            data,
        }
    }

    /// Get the next event from the stream, if available.
    ///
    /// Returns None when the stream is done.
    fn read_next_event_from_buffer(&mut self) -> Result<Option<SseEvent>, SseStatus> {
        let next_line_end = self
            .buffer
            .iter()
            .zip(self.buffer.iter().skip(1))
            .position(|(one, two)| *one == b'\n' && *two == b'\n');
        if let Some(end_position) = next_line_end {
            let message_bytes = &self.buffer[..end_position];
            let mut event = SseEvent {
                event: None,
                id: None,
                data: None,
            };
            // in sse, the trailing newline is defined to be control character - so it's not included in the data.
            // That said, because sse is so special, multiple data: lines mean a newline goes between them. But
            // pedantically, it's not the control newline - it's a newline because of another data: line.
            for line in message_bytes.split(|&b| b == b'\n') {
                if line.starts_with(b"data:") {
                    if let Some(data) = event.data.as_mut() {
                        data.push(b'\n');
                    }
                    // a space after the colon is optional. If it's there it is supposed to be ignored.
                    let offset = if 5 < line.len() && line[5] == b' ' {
                        6
                    } else {
                        5
                    };
                    event
                        .data
                        .get_or_insert(Vec::new())
                        .extend_from_slice(&line[offset..]);
                } else if line.starts_with(b"event:") {
                    if event.event.is_some() {
                        return Err(SseStatus::ProtocolError(format!(
                            "Multiple event fields in one message: {}",
                            String::from_utf8_lossy(line)
                        )));
                    }
                    let offset = if 6 < line.len() && line[6] == b' ' {
                        7
                    } else {
                        6
                    };
                    event.event = Some(line[offset..].to_vec());
                } else if line.starts_with(b"id:") {
                    if event.id.is_some() {
                        return Err(SseStatus::ProtocolError(format!(
                            "Multiple id fields in one message: {}",
                            String::from_utf8_lossy(line)
                        )));
                    }
                    let offset = if 3 < line.len() && line[3] == b' ' {
                        4
                    } else {
                        3
                    };
                    event.id = Some(line[offset..].to_vec());
                }
            }
            // +2 for \n\n
            let new_length = self.buffer.len() - (end_position + 2);
            if 0 < new_length {
                // shift the buffer down. I could use a VecDeque but sse events are typically small and temporally separated. I expect this to be
                // infrequently exercised.
                self.buffer.copy_within(end_position + 2.., 0);
            }
            self.buffer.truncate(new_length);
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }

    /// Read more data from the underlying Data into the buffer, returning any protocol errors or end-of-stream status
    fn read_more_data(&mut self) -> Result<(), SseStatus> {
        let starting_length = self.buffer.len();
        // allow larger chunks if this sse stream is loading a lot - but ensure at least 1kb.
        // if the buffer has already grown past 1kb, we can go ahead and let it fill up. No
        // sense in doing more syscalls or more allocation than necessary.
        let chunk_size = 1024.max(self.buffer.capacity() - self.buffer.len());
        self.buffer.resize(starting_length + chunk_size, 0);
        let bytes_read = std::io::Read::read(&mut self.data, &mut self.buffer[starting_length..])
            .map_err(|e| {
            SseStatus::ProtocolError(format!("Failed to read from SSE stream: {e}"))
        })?;
        if bytes_read == 0 {
            return Err(SseStatus::Closed);
        }
        self.buffer.truncate(starting_length + bytes_read);
        Ok(())
    }
}

impl Iterator for SseStream {
    type Item = Result<SseEvent, SseStatus>;

    /// return the next event from the stream.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.read_next_event_from_buffer() {
                Ok(Some(event)) => return Some(Ok(event)),
                Ok(None) => {
                    if let Err(e) = self.read_more_data() {
                        match e {
                            SseStatus::Closed => return None,
                            SseStatus::ProtocolError(_) => return Some(Err(e)),
                        }
                    }
                }
                Err(e) => match e {
                    SseStatus::Closed => return None,
                    SseStatus::ProtocolError(_) => return Some(Err(e)),
                },
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_event_eq(
        actual: &SseEvent,
        event: Option<&str>,
        event_id: Option<&str>,
        data: Option<&str>,
    ) {
        assert_eq!(actual.event(), event, "event name mismatch");
        assert_eq!(actual.event_id(), event_id, "event id mismatch");
        assert_eq!(actual.data(), data, "data mismatch");
    }

    /// Drive a stream to completion, collecting the UTF-8 `data` of every event.
    /// Panics on a protocol error so tests that expect clean streams stay terse.
    fn collect_data(input: impl Into<Data>) -> Vec<String> {
        SseStream::from_data(input.into())
            .map(|r| {
                r.expect("stream should not error")
                    .data()
                    .unwrap_or("")
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn single_data_event() {
        let mut stream = SseStream::from_data("data: hello\n\n".into());
        let event = stream.next().expect("an event").expect("no error");
        assert_event_eq(&event, None, None, Some("hello"));
        assert!(
            stream.next().is_none(),
            "stream should end after the single event"
        );
    }

    #[test]
    fn space_after_colon_is_optional_and_only_one_is_stripped() {
        // No space after the colon.
        assert_eq!(collect_data("data:nospace\n\n"), vec!["nospace"]);
        // Exactly one leading space is consumed; a second space is part of the data.
        assert_eq!(collect_data("data:  two-spaces\n\n"), vec![" two-spaces"]);
    }

    #[test]
    fn event_id_and_data_fields_are_all_parsed() {
        let mut stream = SseStream::from_data("event: update\nid: 42\ndata: payload\n\n".into());
        let event = stream.next().expect("an event").expect("no error");
        assert_event_eq(&event, Some("update"), Some("42"), Some("payload"));
    }

    #[test]
    fn multiple_data_lines_are_joined_with_newline() {
        let mut stream = SseStream::from_data("data: line1\ndata: line2\ndata: line3\n\n".into());
        let event = stream.next().expect("an event").expect("no error");
        assert_event_eq(&event, None, None, Some("line1\nline2\nline3"));
    }

    #[test]
    fn empty_data_field_yields_empty_string_not_none() {
        let mut stream = SseStream::from_data("data:\n\n".into());
        let event = stream.next().expect("an event").expect("no error");
        assert_event_eq(&event, None, None, Some(""));
        assert_eq!(event.data_raw(), Some(&b""[..]));
    }

    #[test]
    fn comment_and_unknown_fields_are_ignored() {
        let mut stream =
            SseStream::from_data(": this is a keepalive comment\nfoo: bar\ndata: real\n\n".into());
        let event = stream.next().expect("an event").expect("no error");
        assert_event_eq(&event, None, None, Some("real"));
    }

    #[test]
    fn consumes_multiple_events_in_order() {
        // Regression guard: the buffer must be drained after each event so the
        // iterator advances instead of re-yielding the first event forever.
        assert_eq!(
            collect_data("data: a\n\ndata: b\n\ndata: c\n\n"),
            vec!["a", "b", "c"],
        );
    }

    #[test]
    fn clean_eof_terminates_the_iterator() {
        let mut stream = SseStream::from_data("data: only\n\n".into());
        assert_event_eq(
            &stream.next().expect("an event").expect("no error"),
            None,
            None,
            Some("only"),
        );
        assert!(stream.next().is_none(), "first poll past the end is None");
        assert!(
            stream.next().is_none(),
            "subsequent polls past the end stay None"
        );
    }

    #[test]
    fn trailing_event_without_blank_line_is_not_dispatched() {
        // Per the SSE spec a block is only dispatched on a blank line, so the
        // unterminated trailing event is dropped when the stream closes.
        assert_eq!(collect_data("data: a\n\ndata: dangling\n"), vec!["a"]);
    }

    #[test]
    fn duplicate_event_field_is_a_protocol_error() {
        let mut stream = SseStream::from_data("event: first\nevent: second\n\n".into());
        let err = stream
            .next()
            .expect("a result")
            .expect_err("should be a protocol error");
        assert!(matches!(err, SseStatus::ProtocolError(_)), "got {err:?}");
    }

    #[test]
    fn duplicate_id_field_is_a_protocol_error() {
        let mut stream = SseStream::from_data("id: 1\nid: 2\n\n".into());
        let err = stream
            .next()
            .expect("a result")
            .expect_err("should be a protocol error");
        assert!(matches!(err, SseStatus::ProtocolError(_)), "got {err:?}");
    }

    #[test]
    fn extract_decodes_event_data() {
        let mut stream = SseStream::from_data("data: hello world\n\n".into());
        let event = stream.next().expect("an event").expect("no error");
        let decoded: String = event.extract().expect("valid utf-8");
        assert_eq!(decoded, "hello world");
    }

    #[test]
    fn invalid_utf8_data_is_available_raw_but_not_as_str() {
        let mut bytes = b"data: ".to_vec();
        bytes.extend_from_slice(&[0xff, 0xfe]);
        bytes.extend_from_slice(b"\n\n");
        let mut stream = SseStream::from_data(bytes.into());
        let event = stream.next().expect("an event").expect("no error");
        assert_eq!(event.data_raw(), Some(&[0xff, 0xfe][..]));
        assert_eq!(event.data(), None, "invalid utf-8 is not exposed as &str");
    }

    #[test]
    fn many_events_spanning_multiple_reads_are_all_consumed_in_order() {
        // Far more than the 1 KiB read chunk, so consumption crosses several
        // `read_more_data` calls and exercises the buffer shift between events.
        let count = 500;
        let mut input = String::new();
        for i in 0..count {
            input.push_str(&format!("data: {i}\n\n"));
        }
        let parsed: Vec<usize> = collect_data(input)
            .iter()
            .map(|d| d.parse().expect("numeric data"))
            .collect();
        assert_eq!(parsed, (0..count).collect::<Vec<_>>());
    }

    #[test]
    fn single_event_larger_than_one_read_chunk_is_reassembled() {
        // A data payload bigger than the 1 KiB read chunk must be stitched back
        // together across reads before the terminating blank line is seen.
        let big = "x".repeat(4096);
        let mut stream = SseStream::from_data(format!("data: {big}\n\n").into());
        let event = stream.next().expect("an event").expect("no error");
        assert_event_eq(&event, None, None, Some(big.as_str()));
        assert!(stream.next().is_none());
    }

    #[test]
    fn test_for_loop() {
        // Just make sure the Iterator impl doesn't do anything weird in a for loop.
        let input = "data: a\n\ndata: b\n\ndata: c\n\n".to_string();
        let mut events = Vec::new();
        for event in SseStream::from_data(input.into()) {
            events.push(event.expect("no error").data().unwrap_or("").to_owned());
        }
        assert_eq!(events, vec!["a", "b", "c"]);
    }
}
