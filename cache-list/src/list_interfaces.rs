use std::time::Duration;

use momento_functions_bytes::{
    Data,
    encoding::{Encode, Extract},
};

use crate::{
    collection_ttl::CollectionTtl,
    errors::{
        CacheListConcatenateError, CacheListEraseError, CacheListFetchError, CacheListLengthError,
        CacheListPopError, CacheListPushBackError, CacheListPushFrontError, CacheListRemoveError,
        CacheListRetainError,
    },
    types::{
        EndIndex, EraseRange, EraseResult, LengthResult, PopResult, RemoveRange, RemoveResult,
        RetainResult, StartIndex,
    },
    wit::momento::cache_list::cache_list,
};

/// Adds an element to the front of the given list. Creates the list if it does not already exist.
///
/// # Arguments
/// * `list_name` - The name of the list.
/// * `value` - The value to prepend to the list.
/// * `collection_ttl` - The time-to-live for the list.
/// * `truncate_back_to_size` - If the list exceeds this length, remove excess from the back of the list.
///
/// # Examples:
/// ________
/// Prepend a value to the front of a list:
/// ```rust
/// use momento_functions_cache_list::{list_push_front, CacheListPushFrontError, CollectionTtl};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheListPushFrontError<std::convert::Infallible>> {
/// let list_length = list_push_front(
///     "my_list",
///     b"new_value".to_vec(),
///     CollectionTtl::of(Duration::from_secs(60)),
///     None,
/// )?;
///
/// log::info!("New length of my_list: {}", list_length);
/// # Ok(()) }
/// ```
pub fn list_push_front<E: Encode>(
    list_name: impl Into<Data>,
    value: E,
    collection_ttl: CollectionTtl,
    truncate_back_to_size: Option<u32>,
) -> Result<u32, CacheListPushFrontError<E::Error>> {
    cache_list::list_push_front(
        list_name.into().into(),
        value
            .try_serialize()
            .map_err(|e| CacheListPushFrontError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(collection_ttl.ttl()),
        collection_ttl.refresh(),
        truncate_back_to_size.unwrap_or(0),
    )
    .map_err(Into::into)
}

/// Adds an element to the back of the given list. Creates the list if it does not already exist.
///
/// # Arguments
/// * `list_name` - The name of the list.
/// * `value` - The value to append to the list.
/// * `collection_ttl` - The time-to-live for the list.
/// * `truncate_front_to_size` - If the list exceeds this length, remove excess from the front of the list.
///
/// # Examples:
/// ________
/// Append a value to the back of a list:
/// ```rust
/// use momento_functions_cache_list::{list_push_back, CacheListPushBackError, CollectionTtl};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheListPushBackError<std::convert::Infallible>> {
/// let list_length = list_push_back(
///     "my_list",
///     b"new_value".to_vec(),
///     CollectionTtl::of(Duration::from_secs(60)),
///     None,
/// )?;
///
/// log::info!("New length of my_list: {}", list_length);
/// # Ok(()) }
/// ```
pub fn list_push_back<E: Encode>(
    list_name: impl Into<Data>,
    value: E,
    collection_ttl: CollectionTtl,
    truncate_front_to_size: Option<u32>,
) -> Result<u32, CacheListPushBackError<E::Error>> {
    cache_list::list_push_back(
        list_name.into().into(),
        value
            .try_serialize()
            .map_err(|e| CacheListPushBackError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(collection_ttl.ttl()),
        collection_ttl.refresh(),
        truncate_front_to_size.unwrap_or(0),
    )
    .map_err(Into::into)
}

/// Removes and returns the first element of a list.
///
/// # Examples:
/// ________
/// ```rust
/// use momento_functions_cache_list::{list_pop_front, CacheListPopError, PopResult};
///
/// # fn f() -> Result<(), CacheListPopError<std::convert::Infallible>> {
/// let result: PopResult<Vec<u8>> = list_pop_front("my_list")?;
/// match result {
///     PopResult::Found { value, list_length } => {
///         log::info!("Popped value, {} elements remaining", list_length);
///     }
///     PopResult::Missing => {
///         log::info!("List not found");
///     }
/// }
/// # Ok(()) }
/// ```
pub fn list_pop_front<T: Extract>(
    list_name: impl Into<Data>,
) -> Result<PopResult<T>, CacheListPopError<T::Error>> {
    match cache_list::list_pop_front(list_name.into().into())? {
        cache_list::PopResponse::Found(found) => {
            let value = T::extract(Data::from(found.value))
                .map_err(|e| CacheListPopError::ExtractFailed { cause: e })?;
            Ok(PopResult::Found {
                value,
                list_length: found.list_length,
            })
        }
        cache_list::PopResponse::Missing => Ok(PopResult::Missing),
    }
}

/// Removes and returns the last element of a list.
///
/// # Examples:
/// ________
/// ```rust
/// use momento_functions_cache_list::{list_pop_back, CacheListPopError, PopResult};
///
/// # fn f() -> Result<(), CacheListPopError<std::convert::Infallible>> {
/// let result: PopResult<Vec<u8>> = list_pop_back("my_list")?;
/// match result {
///     PopResult::Found { value, list_length } => {
///         log::info!("Popped value, {} elements remaining", list_length);
///     }
///     PopResult::Missing => {
///         log::info!("List not found");
///     }
/// }
/// # Ok(()) }
/// ```
pub fn list_pop_back<T: Extract>(
    list_name: impl Into<Data>,
) -> Result<PopResult<T>, CacheListPopError<T::Error>> {
    match cache_list::list_pop_back(list_name.into().into())? {
        cache_list::PopResponse::Found(found) => {
            let value = T::extract(Data::from(found.value))
                .map_err(|e| CacheListPopError::ExtractFailed { cause: e })?;
            Ok(PopResult::Found {
                value,
                list_length: found.list_length,
            })
        }
        cache_list::PopResponse::Missing => Ok(PopResult::Missing),
    }
}

/// Erases elements from a list by index ranges, or erases the entire list.
///
/// # Examples:
/// ________
/// Erase all elements:
/// ```rust
/// use momento_functions_cache_list::{list_erase, CacheListEraseError, EraseRange, EraseResult};
///
/// # fn f() -> Result<(), CacheListEraseError> {
/// let result = list_erase("my_list", EraseRange::All)?;
/// # Ok(()) }
/// ```
/// ________
/// Erase specific ranges:
/// ```rust
/// use momento_functions_cache_list::{list_erase, CacheListEraseError, EraseRange, EraseResult, ListRange};
///
/// # fn f() -> Result<(), CacheListEraseError> {
/// let result = list_erase(
///     "my_list",
///     EraseRange::Ranges(vec![ListRange { begin_index: 0, count: 2 }]),
/// )?;
/// # Ok(()) }
/// ```
pub fn list_erase(
    list_name: impl Into<Data>,
    range: EraseRange,
) -> Result<EraseResult, CacheListEraseError> {
    let range: cache_list::EraseRange = range.into();
    cache_list::list_erase(list_name.into().into(), &range)
        .map(Into::into)
        .map_err(Into::into)
}

/// Removes all elements with a specific value from a list.
///
/// # Examples:
/// ________
/// ```rust
/// use momento_functions_cache_list::{list_remove, CacheListRemoveError, RemoveRange, RemoveResult};
///
/// # fn f() -> Result<(), CacheListRemoveError> {
/// let result = list_remove(
///     "my_list",
///     RemoveRange::AllElementsWithValue("unwanted".into()),
/// )?;
/// # Ok(()) }
/// ```
pub fn list_remove(
    list_name: impl Into<Data>,
    range: RemoveRange,
) -> Result<RemoveResult, CacheListRemoveError> {
    cache_list::list_remove(list_name.into().into(), range.into())
        .map(Into::into)
        .map_err(Into::into)
}

/// Fetches elements from a list within the specified index range.
///
/// # Examples:
/// ________
/// Fetch all elements:
/// ```rust
/// use momento_functions_cache_list::{list_fetch, CacheListFetchError, StartIndex, EndIndex};
///
/// # fn f() -> Result<(), CacheListFetchError<std::convert::Infallible>> {
/// let result: Option<Vec<Vec<u8>>> = list_fetch(
///     "my_list",
///     StartIndex::Unbounded,
///     EndIndex::Unbounded,
/// )?;
/// # Ok(()) }
/// ```
/// ________
/// Fetch a slice:
/// ```rust
/// use momento_functions_cache_list::{list_fetch, CacheListFetchError, StartIndex, EndIndex};
///
/// # fn f() -> Result<(), CacheListFetchError<std::convert::Infallible>> {
/// let result: Option<Vec<Vec<u8>>> = list_fetch(
///     "my_list",
///     StartIndex::Inclusive(0),
///     EndIndex::Exclusive(10),
/// )?;
/// # Ok(()) }
/// ```
pub fn list_fetch<T: Extract>(
    list_name: impl Into<Data>,
    start: StartIndex,
    end: EndIndex,
) -> Result<Option<Vec<T>>, CacheListFetchError<T::Error>> {
    match cache_list::list_fetch(list_name.into().into(), start.into(), end.into())? {
        cache_list::FetchResponse::Found(items) => {
            let mut result = Vec::with_capacity(items.len());
            for item in items {
                let value = T::extract(Data::from(item))
                    .map_err(|e| CacheListFetchError::ExtractFailed { cause: e })?;
                result.push(value);
            }
            Ok(Some(result))
        }
        cache_list::FetchResponse::Missing => Ok(None),
    }
}

/// Gets the length of a list.
///
/// # Examples:
/// ________
/// ```rust
/// use momento_functions_cache_list::{list_length, CacheListLengthError, LengthResult};
///
/// # fn f() -> Result<(), CacheListLengthError> {
/// let result = list_length("my_list")?;
/// match result {
///     LengthResult::Found(len) => {
///         log::info!("List has {} elements", len);
///     }
///     LengthResult::Missing => {
///         log::info!("List not found");
///     }
/// }
/// # Ok(()) }
/// ```
pub fn list_length(list_name: impl Into<Data>) -> Result<LengthResult, CacheListLengthError> {
    cache_list::list_length(list_name.into().into())
        .map(Into::into)
        .map_err(Into::into)
}

/// Adds multiple elements to the front of a list. Creates the list if it does not already exist.
///
/// # Arguments
/// * `list_name` - The name of the list.
/// * `values` - The values to prepend to the list.
/// * `collection_ttl` - The time-to-live for the list.
/// * `truncate_back_to_size` - If the list exceeds this length, remove excess from the back of the list.
///
/// # Examples:
/// ________
/// ```rust
/// use momento_functions_cache_list::{list_concatenate_front, CacheListConcatenateError, CollectionTtl};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheListConcatenateError<std::convert::Infallible>> {
/// let list_length = list_concatenate_front(
///     "my_list",
///     vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
///     CollectionTtl::of(Duration::from_secs(60)),
///     None,
/// )?;
/// # Ok(()) }
/// ```
pub fn list_concatenate_front<E: Encode>(
    list_name: impl Into<Data>,
    values: Vec<E>,
    collection_ttl: CollectionTtl,
    truncate_back_to_size: Option<u32>,
) -> Result<u32, CacheListConcatenateError<E::Error>> {
    let encoded: Vec<_> = values
        .into_iter()
        .map(|v| {
            v.try_serialize()
                .map(Into::into)
                .map_err(|e| CacheListConcatenateError::EncodeFailed { cause: e })
        })
        .collect::<Result<_, _>>()?;
    cache_list::list_concatenate_front(
        list_name.into().into(),
        encoded,
        saturate_ttl(collection_ttl.ttl()),
        collection_ttl.refresh(),
        truncate_back_to_size.unwrap_or(0),
    )
    .map_err(Into::into)
}

/// Adds multiple elements to the back of a list. Creates the list if it does not already exist.
///
/// # Arguments
/// * `list_name` - The name of the list.
/// * `values` - The values to append to the list.
/// * `collection_ttl` - The time-to-live for the list.
/// * `truncate_front_to_size` - If the list exceeds this length, remove excess from the front of the list.
///
/// # Examples:
/// ________
/// ```rust
/// use momento_functions_cache_list::{list_concatenate_back, CacheListConcatenateError, CollectionTtl};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheListConcatenateError<std::convert::Infallible>> {
/// let list_length = list_concatenate_back(
///     "my_list",
///     vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
///     CollectionTtl::of(Duration::from_secs(60)),
///     None,
/// )?;
/// # Ok(()) }
/// ```
pub fn list_concatenate_back<E: Encode>(
    list_name: impl Into<Data>,
    values: Vec<E>,
    collection_ttl: CollectionTtl,
    truncate_front_to_size: Option<u32>,
) -> Result<u32, CacheListConcatenateError<E::Error>> {
    let encoded: Vec<_> = values
        .into_iter()
        .map(|v| {
            v.try_serialize()
                .map(Into::into)
                .map_err(|e| CacheListConcatenateError::EncodeFailed { cause: e })
        })
        .collect::<Result<_, _>>()?;
    cache_list::list_concatenate_back(
        list_name.into().into(),
        encoded,
        saturate_ttl(collection_ttl.ttl()),
        collection_ttl.refresh(),
        truncate_front_to_size.unwrap_or(0),
    )
    .map_err(Into::into)
}

/// Retains only the elements within the specified index range, removing all others.
///
/// # Examples:
/// ________
/// Retain only the first 10 elements:
/// ```rust
/// use momento_functions_cache_list::{list_retain, CacheListRetainError, CollectionTtl, StartIndex, EndIndex, RetainResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheListRetainError> {
/// let result = list_retain(
///     "my_list",
///     StartIndex::Inclusive(0),
///     EndIndex::Exclusive(10),
///     CollectionTtl::of(Duration::from_secs(60)),
/// )?;
/// # Ok(()) }
/// ```
pub fn list_retain(
    list_name: impl Into<Data>,
    start: StartIndex,
    end: EndIndex,
    collection_ttl: CollectionTtl,
) -> Result<RetainResult, CacheListRetainError> {
    cache_list::list_retain(
        list_name.into().into(),
        start.into(),
        end.into(),
        saturate_ttl(collection_ttl.ttl()),
        collection_ttl.refresh(),
    )
    .map(Into::into)
    .map_err(Into::into)
}

fn saturate_ttl(ttl: Duration) -> u64 {
    ttl.as_millis().min(u64::MAX as u128) as u64
}
