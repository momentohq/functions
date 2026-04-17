use crate::wit::momento::cache_list::cache_list;

/// Result of a pop operation on a list.
pub enum PopResult<T> {
    /// The list was found and a value was popped.
    Found {
        /// The popped value.
        value: T,
        /// The length of the list after the pop.
        list_length: u32,
    },
    /// The list was not found.
    Missing,
}

/// A range of elements in a list, specified by a starting index and count.
pub struct ListRange {
    /// The beginning index of the range.
    pub begin_index: u32,
    /// The number of elements in the range.
    pub count: u32,
}

impl From<ListRange> for cache_list::ListRange {
    fn from(value: ListRange) -> Self {
        Self {
            begin_index: value.begin_index,
            count: value.count,
        }
    }
}

/// Specifies which elements to erase from a list.
pub enum EraseRange {
    /// Erase all elements in the list.
    All,
    /// Erase elements at the specified ranges.
    Ranges(Vec<ListRange>),
}

impl From<EraseRange> for cache_list::EraseRange {
    fn from(value: EraseRange) -> Self {
        match value {
            EraseRange::All => Self::All,
            EraseRange::Ranges(ranges) => Self::Ranges(cache_list::ListRanges {
                ranges: ranges.into_iter().map(Into::into).collect(),
            }),
        }
    }
}

/// Result of an erase operation on a list.
pub enum EraseResult {
    /// The list was found. Contains the length of the list after erasing.
    Found(u32),
    /// The list was not found.
    Missing,
}

impl From<cache_list::EraseResponse> for EraseResult {
    fn from(value: cache_list::EraseResponse) -> Self {
        match value {
            cache_list::EraseResponse::Found(len) => Self::Found(len),
            cache_list::EraseResponse::Missing => Self::Missing,
        }
    }
}

/// Result of a remove operation on a list.
pub enum RemoveResult {
    /// The list was found. Contains the length of the list after removing.
    Found(u32),
    /// The list was not found.
    Missing,
}

impl From<cache_list::RemoveResponse> for RemoveResult {
    fn from(value: cache_list::RemoveResponse) -> Self {
        match value {
            cache_list::RemoveResponse::Found(len) => Self::Found(len),
            cache_list::RemoveResponse::Missing => Self::Missing,
        }
    }
}

/// The start bound for a list fetch or retain operation.
pub enum StartIndex {
    /// Start from the beginning of the list.
    Unbounded,
    /// Start from the given index (inclusive). Negative values count from the end.
    Inclusive(i32),
}

/// Convert an `i32` directly into an inclusive [`StartIndex`].
impl From<i32> for StartIndex {
    fn from(i: i32) -> Self {
        StartIndex::Inclusive(i)
    }
}

impl From<StartIndex> for cache_list::StartIndex {
    fn from(value: StartIndex) -> Self {
        match value {
            StartIndex::Unbounded => Self::Unbounded,
            StartIndex::Inclusive(i) => Self::Inclusive(i),
        }
    }
}

/// The end bound for a list fetch or retain operation.
pub enum EndIndex {
    /// Extend to the end of the list.
    Unbounded,
    /// End at the given index (exclusive). Negative values count from the end.
    Exclusive(i32),
}

/// Convert an `i32` directly into an exclusive [`EndIndex`].
impl From<i32> for EndIndex {
    fn from(i: i32) -> Self {
        EndIndex::Exclusive(i)
    }
}

impl From<EndIndex> for cache_list::EndIndex {
    fn from(value: EndIndex) -> Self {
        match value {
            EndIndex::Unbounded => Self::Unbounded,
            EndIndex::Exclusive(i) => Self::Exclusive(i),
        }
    }
}

/// Result of a list length operation.
pub enum LengthResult {
    /// The list was found. Contains the length.
    Found(u32),
    /// The list was not found.
    Missing,
}

impl From<cache_list::LengthResponse> for LengthResult {
    fn from(value: cache_list::LengthResponse) -> Self {
        match value {
            cache_list::LengthResponse::Found(len) => Self::Found(len),
            cache_list::LengthResponse::Missing => Self::Missing,
        }
    }
}

/// Result of a list retain operation.
pub enum RetainResult {
    /// The list was found. Contains the length of the list after the operation.
    Found(u32),
    /// The list was not found.
    Missing,
}

impl From<cache_list::RetainResponse> for RetainResult {
    fn from(value: cache_list::RetainResponse) -> Self {
        match value {
            cache_list::RetainResponse::Found(len) => Self::Found(len),
            cache_list::RetainResponse::Missing => Self::Missing,
        }
    }
}
