use std::collections::VecDeque;

/// A buffer of bytes, which may be inline or on the host.
///
/// Some bulk data processing Functions may choose to pass data straight from a
/// request or response body to another request or response body. To improve
/// performance on these kinds of Functions, `Data` avoids copying the buffer
/// into your function's memory.
///
/// # Read
/// `Data` implements `std::io::Read`, so you can plug it into a fairly wide
/// variety of libraries that can consume data from a stream.
///
/// Note that when `read()` returns `Ok(0)`, that means the stream has ended.
#[derive(Debug)]
pub struct Data {
    data: Location,
}

impl Data {
    /// Turn the Data into a plain `Vec<u8>`.
    ///
    /// If the data is buffered on the host, this will read the buffer completely
    /// into your function's memory. You should use this with caution!
    ///
    /// For small buffers, this is inconsequential. For larger buffers, this may
    /// cause your function to run out of memory or to run slowly.
    pub fn into_bytes(self) -> Vec<u8> {
        self.data.into_bytes()
    }
}

impl std::io::Read for Data {
    /// Read bytes from the Data into the provided buffer, returning how many bytes were read.
    ///
    /// Always returns `Ok(0 < n)` while the stream is still open.
    /// Returns `Ok(0)` when the stream is done.
    /// Returns `Err` if there was an error reading from the stream.
    #[allow(
        clippy::expect_used,
        reason = "The length is checked before popping from the buffer."
    )]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // okay so the err thing is a lie atm but it probably won't be forever...
        match &mut self.data {
            Location::Inline { buffer } => {
                let bytes_to_read = std::cmp::min(buf.len(), buffer.len());
                for raw_byte in buf.iter_mut().take(bytes_to_read) {
                    *raw_byte = buffer.pop_front().expect("already checked length");
                }
                Ok(bytes_to_read)
            }
            Location::OnHost { resource } => match resource.read(buf.len() as u32) {
                Some(chunk) => {
                    let bytes_read = chunk.len();
                    buf[..bytes_read].copy_from_slice(&chunk);
                    Ok(bytes_read)
                }
                None => Ok(0),
            },
        }
    }
}

impl From<crate::wit::momento::bytes::bytes::Data> for Data {
    fn from(value: crate::wit::momento::bytes::bytes::Data) -> Self {
        match value {
            crate::wit::momento::bytes::bytes::Data::Value(buffer) => Self {
                data: Location::Inline {
                    buffer: buffer.into(),
                },
            },
            crate::wit::momento::bytes::bytes::Data::Buffer(resource) => Self {
                data: Location::OnHost { resource },
            },
        }
    }
}

impl From<Vec<u8>> for Data {
    fn from(value: Vec<u8>) -> Self {
        Self {
            data: Location::Inline {
                buffer: value.into(),
            },
        }
    }
}

impl From<&[u8]> for Data {
    fn from(value: &[u8]) -> Self {
        Self {
            data: Location::Inline {
                buffer: value.to_vec().into(),
            },
        }
    }
}

impl From<String> for Data {
    fn from(value: String) -> Self {
        Self {
            data: Location::Inline {
                buffer: value.into_bytes().into(),
            },
        }
    }
}

impl From<&str> for Data {
    fn from(value: &str) -> Self {
        Self {
            data: Location::Inline {
                buffer: value.as_bytes().to_vec().into(),
            },
        }
    }
}

enum Location {
    Inline {
        buffer: VecDeque<u8>,
    },
    OnHost {
        resource: crate::wit::momento::bytes::bytes::Buffer,
    },
}
impl Location {
    fn into_bytes(self) -> Vec<u8> {
        match self {
            Location::Inline { buffer } => buffer.into(),
            Location::OnHost { resource } => {
                let mut buffer = Vec::with_capacity(resource.remaining() as usize);
                while let Some(chunk) = resource.read(resource.remaining().max(16384)) {
                    buffer.extend(chunk);
                }
                buffer
            }
        }
    }
}

impl std::fmt::Debug for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::Inline { buffer } => f
                .debug_struct("Inline")
                .field("length", &buffer.len())
                .finish(),
            Location::OnHost { resource } => f
                .debug_struct("OnHost")
                .field("remaining", &resource.remaining())
                .finish(),
        }
    }
}

impl From<Data> for crate::wit::momento::bytes::bytes::Data {
    fn from(data: Data) -> Self {
        match data.data {
            Location::Inline { buffer } => Self::Value(buffer.into()),
            Location::OnHost { resource } => Self::Buffer(resource),
        }
    }
}
