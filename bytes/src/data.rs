/// A buffer of bytes, which may be inline or on the host.
///
/// Some bulk data processing Functions may choose to pass data straight from a
/// request or response body to another request or response body. To improve
/// performance on these kinds of Functions, `Data` avoids copying the buffer
/// into your function's memory.
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

impl From<Vec<u8>> for Data {
    fn from(value: Vec<u8>) -> Self {
        Self {
            data: Location::Inline { buffer: value },
        }
    }
}

impl From<crate::wit::momento::bytes::bytes::Data> for Data {
    fn from(value: crate::wit::momento::bytes::bytes::Data) -> Self {
        match value {
            crate::wit::momento::bytes::bytes::Data::Value(buffer) => Self {
                data: Location::Inline { buffer },
            },
            crate::wit::momento::bytes::bytes::Data::Buffer(resource) => Self {
                data: Location::OnHost { resource },
            },
        }
    }
}

enum Location {
    Inline {
        buffer: Vec<u8>,
    },
    OnHost {
        resource: crate::wit::momento::bytes::bytes::Buffer,
    },
}
impl Location {
    fn into_bytes(self) -> Vec<u8> {
        match self {
            Location::Inline { buffer } => buffer,
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
            Location::Inline { buffer } => Self::Value(buffer),
            Location::OnHost { resource } => Self::Buffer(resource),
        }
    }
}
