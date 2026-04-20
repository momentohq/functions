use momento_functions_bytes::{
    Data,
    encoding::{Encode, Extract},
};
use momento_functions_collections_common::{CollectionTtl, saturate_ttl};

use crate::{
    errors::{
        CacheListConcatenateError, CacheListEraseError, CacheListFetchError, CacheListLengthError,
        CacheListPopError, CacheListPushBackError, CacheListPushFrontError, CacheListRemoveError,
        CacheListRetainError,
    },
    types::{
        EndIndex, EraseRange, EraseResult, LengthResult, PopResult, RemoveResult, RetainResult,
        StartIndex,
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
/// ```rust,no_run
/// use momento_functions_cache_list::{list_push_front, CollectionTtl};
/// # use std::time::Duration;
/// match list_push_front(
///     "my_list",
///     b"new_value".to_vec(),
///     CollectionTtl::of(Duration::from_secs(60)),
///     None,
/// ) {
///     Ok(list_length) => { /* use list_length */ }
///     Err(e) => log::error!("list_push_front failed: {e}"),
/// }
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
/// ```rust,no_run
/// use momento_functions_cache_list::{list_push_back, CollectionTtl};
/// # use std::time::Duration;
/// match list_push_back(
///     "my_list",
///     b"new_value".to_vec(),
///     CollectionTtl::of(Duration::from_secs(60)),
///     None,
/// ) {
///     Ok(list_length) => { /* use list_length */ }
///     Err(e) => log::error!("list_push_back failed: {e}"),
/// }
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
/// ```rust,no_run
/// use momento_functions_cache_list::{list_pop_front, PopResult};
///
/// match list_pop_front::<Vec<u8>>("my_list") {
///     Ok(PopResult::Found { value, list_length }) => {
///         // use the popped value
///     }
///     Ok(PopResult::Missing) => {
///         // list not found
///     }
///     Err(e) => log::error!("pop failed: {e}"),
/// }
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
/// ```rust,no_run
/// use momento_functions_cache_list::{list_pop_back, PopResult};
///
/// match list_pop_back::<Vec<u8>>("my_list") {
///     Ok(PopResult::Found { value, list_length }) => {
///         // use the popped value
///     }
///     Ok(PopResult::Missing) => {
///         // list not found
///     }
///     Err(e) => log::error!("pop failed: {e}"),
/// }
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
/// ```rust,no_run
/// use momento_functions_cache_list::{list_erase, EraseRange};
///
/// match list_erase("my_list", EraseRange::All) {
///     Ok(result) => { /* inspect result */ }
///     Err(e) => log::error!("erase failed: {e}"),
/// }
/// ```
/// ________
/// Erase specific ranges:
/// ```rust,no_run
/// use momento_functions_cache_list::{list_erase, EraseRange, ListRange};
///
/// match list_erase(
///     "my_list",
///     EraseRange::Ranges(vec![ListRange { begin_index: 0, count: 2 }]),
/// ) {
///     Ok(result) => { /* inspect result */ }
///     Err(e) => log::error!("erase failed: {e}"),
/// }
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
/// ```rust,no_run
/// use momento_functions_cache_list::list_remove;
///
/// match list_remove(
///     "my_list",
///     "unwanted",
/// ) {
///     Ok(result) => { /* inspect result */ }
///     Err(e) => log::error!("remove failed: {e}"),
/// }
/// ```
pub fn list_remove<E: Encode>(
    list_name: impl Into<Data>,
    value: E,
) -> Result<RemoveResult, CacheListRemoveError<E::Error>> {
    let encoded = value
        .try_serialize()
        .map_err(|e| CacheListRemoveError::EncodeFailed { cause: e })?;
    cache_list::list_remove(
        list_name.into().into(),
        cache_list::RemoveRange::AllElementsWithValue(encoded.into()),
    )
    .map(Into::into)
    .map_err(Into::into)
}

/// Fetches elements from a list within the specified index range.
///
/// Returns:
/// * `Err` — the request failed.
/// * `Ok(None)` — the list does not exist.
/// * `Ok(Some(iter))` — an iterator over `Result<T, T::Error>`, yielding each
///   element lazily. Extraction and allocation only occur as items are consumed.
///
/// `i32` values convert directly: positive indices count from the front,
/// negative from the back. A plain integer start is inclusive; a plain integer
/// end is exclusive.
///
/// # Examples:
/// ________
/// Collect a slice into a `Vec`, defaulting to empty if the list is missing:
/// ```rust,no_run
/// use momento_functions_cache_list::list_fetch;
///
/// let items: Vec<Vec<u8>> = match list_fetch("my_list", 0, 10) {
///     Ok(maybe_iter) => maybe_iter
///         .into_iter()
///         .flatten()
///         .collect::<Result<_, _>>()
///         .unwrap_or_default(),
///     Err(e) => {
///         log::error!("fetch failed: {e}");
///         Vec::new()
///     }
/// };
/// ```
/// ________
/// Stream elements one at a time without collecting:
/// ```rust,no_run
/// use momento_functions_cache_list::{list_fetch, StartIndex, EndIndex};
///
/// match list_fetch::<Vec<u8>>("my_list", StartIndex::Unbounded, EndIndex::Unbounded) {
///     Ok(Some(items)) => {
///         for result in items {
///             match result {
///                 Ok(value) => { /* use value */ }
///                 Err(e) => { /* handle extraction error */ }
///             }
///         }
///     }
///     Ok(None) => { /* list not found */ }
///     Err(e) => log::error!("fetch failed: {e}"),
/// }
/// ```
pub fn list_fetch<T: Extract>(
    list_name: impl Into<Data>,
    start: impl Into<StartIndex>,
    end: impl Into<EndIndex>,
) -> Result<Option<impl Iterator<Item = Result<T, T::Error>>>, CacheListFetchError> {
    match cache_list::list_fetch(
        list_name.into().into(),
        start.into().into(),
        end.into().into(),
    )? {
        cache_list::FetchResponse::Found(items) => Ok(Some(
            items.into_iter().map(|item| T::extract(Data::from(item))),
        )),
        cache_list::FetchResponse::Missing => Ok(None),
    }
}

/// Gets the length of a list.
///
/// # Examples:
/// ________
/// ```rust,no_run
/// use momento_functions_cache_list::{list_length, LengthResult};
///
/// match list_length("my_list") {
///     Ok(LengthResult::Found(len)) => {
///         // use the length
///     }
///     Ok(LengthResult::Missing) => {
///         // list not found
///     }
///     Err(e) => log::error!("length failed: {e}"),
/// }
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
/// ```rust,no_run
/// use momento_functions_cache_list::{list_concatenate_front, CollectionTtl};
/// # use std::time::Duration;
/// match list_concatenate_front(
///     "my_list",
///     vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
///     CollectionTtl::of(Duration::from_secs(60)),
///     None,
/// ) {
///     Ok(list_length) => { /* use list_length */ }
///     Err(e) => log::error!("concatenate_front failed: {e}"),
/// }
/// ```
pub fn list_concatenate_front<E: Encode>(
    list_name: impl Into<Data>,
    values: impl IntoIterator<Item = E>,
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
/// ```rust,no_run
/// use momento_functions_cache_list::{list_concatenate_back, CollectionTtl};
/// # use std::time::Duration;
/// match list_concatenate_back(
///     "my_list",
///     vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
///     CollectionTtl::of(Duration::from_secs(60)),
///     None,
/// ) {
///     Ok(list_length) => { /* use list_length */ }
///     Err(e) => log::error!("concatenate_back failed: {e}"),
/// }
/// ```
pub fn list_concatenate_back<E: Encode>(
    list_name: impl Into<Data>,
    values: impl IntoIterator<Item = E>,
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
/// ```rust,no_run
/// use momento_functions_cache_list::{list_retain, CollectionTtl, StartIndex, EndIndex};
/// # use std::time::Duration;
/// match list_retain(
///     "my_list",
///     StartIndex::Inclusive(0),
///     EndIndex::Exclusive(10),
///     CollectionTtl::of(Duration::from_secs(60)),
/// ) {
///     Ok(result) => { /* inspect result */ }
///     Err(e) => log::error!("retain failed: {e}"),
/// }
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
