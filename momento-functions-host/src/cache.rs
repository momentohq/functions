//! Host interfaces for working with Momento Cache apis

use std::time::Duration;

use crate::encoding::{Encode, EncodeError, Extract, ExtractError};
use momento_functions_wit::host::momento::functions::cache_list;
use momento_functions_wit::host::momento::functions::cache_scalar;

pub use cache_scalar::GetWithHashFound;
pub use cache_scalar::GetWithHashResult;
pub use cache_scalar::SetIfCondition;
pub use cache_scalar::SetIfHashCondition;
pub use cache_scalar::SetIfHashResult;
pub use cache_scalar::SetIfResult;

/// An error occurred when setting a value in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheSetError<E: EncodeError> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}

/// An error occurred when getting a value from the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheGetError<E: ExtractError> {
    /// The value could not be extracted with the provided implementation.
    #[error("Failed to extract value.")]
    ExtractFailed {
        /// The underlying error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}
/// Get a value from the cache.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheGetError;
///
/// # fn f() -> Result<(), CacheGetError<Vec<u8>>> {
/// let value: Option<Vec<u8>> = cache::get("my_key")?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheGetError;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Deserialize)]
/// struct MyStruct {
///   message: String
/// }
///
/// # fn f() -> Result<(), CacheGetError<Json<MyStruct>>> {
/// let value: Option<Json<MyStruct>> = cache::get("my_key")?;
/// # Ok(()) }
/// ```
pub fn get<T: Extract>(key: impl AsRef<[u8]>) -> Result<Option<T>, CacheGetError<T::Error>> {
    match cache_scalar::get(key.as_ref())? {
        Some(v) => T::extract(v)
            .map(Some)
            .map_err(|e| CacheGetError::ExtractFailed { cause: e }),
        None => Ok(None),
    }
}

/// Set a value in the cache with a time-to-live.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheSetError;
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetError<&'static str>> {
/// cache::set(
///     "my_key",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
/// )?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheSetError;
/// # use std::time::Duration;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> Result<(), CacheSetError<Json<MyStruct>>> {
/// cache::set(
///     "my_key",
///     Json(MyStruct { hello: "hello".to_string() }),
///     Duration::from_secs(60),
/// )?;
/// # Ok(()) }
/// ```
pub fn set<E: Encode>(
    key: impl AsRef<[u8]>,
    value: E,
    ttl: Duration,
) -> Result<(), CacheSetError<E::Error>> {
    cache_scalar::set(
        key.as_ref(),
        &value
            .try_serialize()
            .map_err(|e| CacheSetError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(ttl),
    )
    .map_err(Into::into)
}

fn saturate_ttl(ttl: Duration) -> u64 {
    ttl.as_millis().clamp(0, u64::MAX as u128) as u64
}

/// An error occurred when conditionally setting a value in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheSetIfError<E: EncodeError> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}

/// An error occurred when deleting a value from the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheDeleteError {
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}

/// Conditionally set a value in the cache based on a condition.
///
/// Examples:
/// ________
/// Set only if absent:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheSetIfError, SetIfCondition, SetIfResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfError<&'static str>> {
/// let result: SetIfResult = cache::set_if(
///     "my_key",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Absent,
/// )?;
/// match result {
///     SetIfResult::Stored => {
///         // Do something
///     },
///     SetIfResult::NotStored => {
///         // Do something else
///     },
/// }
/// # Ok(()) }
/// ```
/// ________
/// Set only if present:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheSetIfError, SetIfCondition, SetIfResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfError<&'static str>> {
/// let result = cache::set_if(
///     "my_key",
///     b"updated".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Present,
/// )?;
/// # Ok(()) }
/// ```
/// ________
/// Set only if equal to a specific value:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheSetIfError, SetIfCondition, SetIfResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfError<&'static str>> {
/// let result = cache::set_if(
///     "my_key",
///     b"new_value".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Equal(b"old_value".to_vec()),
/// )?;
/// # Ok(()) }
/// ```
pub fn set_if<E: Encode>(
    key: impl AsRef<[u8]>,
    value: E,
    ttl: Duration,
    condition: SetIfCondition,
) -> Result<SetIfResult, CacheSetIfError<E::Error>> {
    cache_scalar::set_if(
        key.as_ref(),
        &value
            .try_serialize()
            .map_err(|e| CacheSetIfError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(ttl),
        &condition,
    )
    .map_err(Into::into)
}

/// Delete a value from the cache.
///
/// Note: This operation is idempotent. Deleting a key that does not exist
/// will not return an error.
///
/// Examples:
/// ________
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheDeleteError;
///
/// # fn f() -> Result<(), CacheDeleteError> {
/// cache::delete("my_key")?;
/// # Ok(()) }
/// ```
pub fn delete(key: impl AsRef<[u8]>) -> Result<(), CacheDeleteError> {
    cache_scalar::delete(key.as_ref()).map_err(Into::into)
}

/// An error occurred when getting a value with its hash from the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheGetWithHashError<E: ExtractError> {
    /// The value could not be extracted with the provided implementation.
    #[error("Failed to extract value.")]
    ExtractFailed {
        /// The underlying error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}

/// An error occurred when conditionally setting a value based on hash comparison.
#[derive(thiserror::Error, Debug)]
pub enum CacheSetIfHashError<E: EncodeError> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}

/// A value retrieved from the cache along with its hash.
pub struct GetWithHashValue<T> {
    /// The extracted value.
    pub value: T,
    /// The hash of the value.
    pub hash: Vec<u8>,
}

/// Get a value from the cache along with its hash.
///
/// The hash can be used with [`set_if_hash`] to perform conditional updates
/// based on whether the value has changed since it was read.
///
/// Examples:
/// ________
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheGetWithHashError, GetWithHashValue};
///
/// # fn f() -> Result<(), CacheGetWithHashError<Vec<u8>>> {
/// let result: Option<GetWithHashValue<Vec<u8>>> = cache::get_with_hash("my_key")?;
/// if let Some(entry) = result {
///     log::info!("Value: {:?}, Hash: {:?}", entry.value, entry.hash);
/// }
/// # Ok(()) }
/// ```
pub fn get_with_hash<T: Extract>(
    key: impl AsRef<[u8]>,
) -> Result<Option<GetWithHashValue<T>>, CacheGetWithHashError<T::Error>> {
    match cache_scalar::get_with_hash(key.as_ref())? {
        GetWithHashResult::Found(found) => {
            let value = T::extract(found.value)
                .map_err(|e| CacheGetWithHashError::ExtractFailed { cause: e })?;
            Ok(Some(GetWithHashValue {
                value,
                hash: found.hash,
            }))
        }
        GetWithHashResult::Missing => Ok(None),
    }
}

/// Conditionally set a value in the cache based on a hash comparison.
///
/// This is useful for optimistic concurrency control where you want to update
/// a value only if it hasn't changed since you last read it, without needing
/// to compare the full value.
///
/// Examples:
/// ________
/// Update only if the hash matches (value hasn't changed):
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheSetIfHashError, SetIfHashCondition, SetIfHashResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfHashError<&'static str>> {
/// // First, get the current value and its hash
/// // let entry = cache::get_with_hash("my_key")?;
/// let previous_hash = vec![1, 2, 3]; // Hash from a previous get_with_hash call
///
/// let result = cache::set_if_hash(
///     "my_key",
///     b"new_value".to_vec(),
///     Duration::from_secs(60),
///     SetIfHashCondition::PresentAndHashEqual(previous_hash),
/// )?;
/// match result {
///     SetIfHashResult::Stored(new_hash) => {
///         log::info!("Value updated, new hash: {:?}", new_hash);
///     }
///     SetIfHashResult::NotStored => {
///         log::info!("Value was modified by another process");
///     }
/// }
/// # Ok(()) }
/// ```
pub fn set_if_hash<E: Encode>(
    key: impl AsRef<[u8]>,
    value: E,
    ttl: Duration,
    condition: SetIfHashCondition,
) -> Result<SetIfHashResult, CacheSetIfHashError<E::Error>> {
    cache_scalar::set_if_hash(
        key.as_ref(),
        &value
            .try_serialize()
            .map_err(|e| CacheSetIfHashError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(ttl),
        &condition,
    )
    .map_err(Into::into)
}

/// Represents the desired behavior for managing the TTL on collections.
///
/// The first time the collection is created, it needs to set a TTL. For subsequent operations
/// that modify the collection, you may choose to update the TTL in order to prolong the life
/// of the cached collection, or to leave the TTL unmodified to ensure the collection expires
/// at the original TTL.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CollectionTtl {
    ttl: Duration,
    refresh: bool,
}

impl CollectionTtl {
    /// Create a collection TTL with the provided `ttl` and `refresh` settings.
    pub const fn new(ttl: Duration, refresh: bool) -> Self {
        Self { ttl, refresh }
    }

    /// Create a collection TTL that updates the TTL for the collection any time it is
    /// modified.
    pub fn refresh_on_update(ttl: impl Into<Duration>) -> Self {
        Self::new(ttl.into(), true)
    }

    /// Create a collection TTL that will not refresh the TTL for the collection when
    /// it is updated.
    ///
    /// Use this if you want to be sure that the collection expires at the originally
    /// specified time, even if you make modifications to the value of the collection.
    ///
    /// The TTL will still be used when a new collection is created.
    pub fn initialize_only(ttl: impl Into<Duration>) -> Self {
        Self::new(ttl.into(), false)
    }

    /// Return a new collection TTL which uses the same TTL but refreshes on updates.
    pub fn with_refresh_on_update(self) -> Self {
        Self::new(self.ttl(), true)
    }

    /// Return a new collection TTL which uses the same TTL but does not refresh on
    /// updates.
    pub fn with_no_refresh_on_update(self) -> Self {
        Self::new(self.ttl(), false)
    }

    /// Return a new collection TTL which has the same refresh behavior but uses the
    /// provided TTL.
    pub fn with_ttl(self, ttl: impl Into<Duration>) -> Self {
        Self::new(ttl.into(), self.refresh())
    }

    /// Constructs a CollectionTtl with the specified TTL. The TTL for the collection will be
    /// refreshed any time the collection is modified.
    pub fn of(ttl: Duration) -> Self {
        Self::new(ttl, true)
    }

    /// The [`Duration`] after which the cached collection should be expired from the
    /// cache.
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Whether the collection's TTL will be refreshed on every update.
    ///
    /// If true, this will extend the time at which the collection would expire when
    /// an update operation happens. Otherwise, the collection's TTL will only be set
    /// when it is initially created.
    pub fn refresh(&self) -> bool {
        self.refresh
    }
}

/// An error occurred when pushing a value to the back of a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListPushBackError<E: EncodeError> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
}

/// An error occurred when pushing a value to the front of a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListPushFrontError<E: EncodeError> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
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
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheListPushBackError;
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheListPushBackError<&'static str>> {
///
/// let list_length = cache::list_push_back(
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
    list_name: impl AsRef<[u8]>,
    value: E,
    collection_ttl: CollectionTtl,
    truncate_front_to_size: Option<u32>,
) -> Result<u32, CacheListPushBackError<E::Error>> {
    cache_list::list_push_back(
        list_name.as_ref(),
        &value
            .try_serialize()
            .map_err(|e| CacheListPushBackError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(collection_ttl.ttl()),
        collection_ttl.refresh(),
        truncate_front_to_size.unwrap_or(0),
    )
    .map_err(Into::into)
}

/// Adds an element to the front of the given list. Creates the list if it does not already exist.
///
/// # Arguments
/// * `list_name` - The name of the list.
/// * `value` - The value to append to the list.
/// * `collection_ttl` - The time-to-live for the list.
/// * `truncate_back_to_size` - If the list exceeds this length, remove excess from the back of the list.
///
/// # Examples:
/// ________
/// Append a value to the back of a list:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheListPushFrontError, CollectionTtl};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheListPushFrontError<&'static str>> {
///
/// let list_length = cache::list_push_front(
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
    list_name: impl AsRef<[u8]>,
    value: E,
    collection_ttl: CollectionTtl,
    truncate_back_to_size: Option<u32>,
) -> Result<u32, CacheListPushFrontError<E::Error>> {
    cache_list::list_push_front(
        list_name.as_ref(),
        &value
            .try_serialize()
            .map_err(|e| CacheListPushFrontError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(collection_ttl.ttl()),
        collection_ttl.refresh(),
        truncate_back_to_size.unwrap_or(0),
    )
    .map_err(Into::into)
}
