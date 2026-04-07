use momento_functions_bytes::Data;

use crate::wit::momento::cache_scalar::cache_scalar;

/// Conditionally set a value in the cache.
pub enum SetIfCondition {
    /// Set the value only if the key is already present in the cache.
    Present,
    /// Set the value only if the key is already present in the cache and its current value is not equal to a specific value.
    PresentAndNotEqual(Data),
    /// Set the value only if the key is either not present in the cache or its current value is not equal to a specific value.
    NotEqual(Data),
    /// Set the value only if the key is not already present in the cache.
    Absent,
    /// Set the value only if the current value for the key is equal to a specific value.
    Equal(Data),
    /// Set the value only if the key is either not present in the cache or its current value is equal to a specific value.
    AbsentOrEqual(Data),
}

impl From<SetIfCondition> for cache_scalar::SetIfCondition {
    fn from(value: SetIfCondition) -> Self {
        match value {
            SetIfCondition::Present => Self::Present,
            SetIfCondition::PresentAndNotEqual(data) => Self::PresentAndNotEqual(data.into()),
            SetIfCondition::NotEqual(data) => Self::NotEqual(data.into()),
            SetIfCondition::Absent => Self::Absent,
            SetIfCondition::Equal(data) => Self::Equal(data.into()),
            SetIfCondition::AbsentOrEqual(data) => Self::AbsentOrEqual(data.into()),
        }
    }
}

/// Result of a conditional set operation.
pub enum ConditionalSetResult<Value> {
    /// The value was stored in the cache.
    Stored(Value),
    /// The value was not stored in the cache because the condition was not met.
    NotStored,
}

impl From<cache_scalar::SetIfResult> for ConditionalSetResult<()> {
    fn from(value: cache_scalar::SetIfResult) -> Self {
        match value {
            cache_scalar::SetIfResult::Stored => Self::Stored(()),
            cache_scalar::SetIfResult::NotStored => Self::NotStored,
        }
    }
}

/// Condition for set-if-hash operations (comparing hashes instead of full values).
pub enum SetIfHashCondition {
    /// Set only if the key exists and the hash of its current value does not match the provided hash.
    PresentAndNotHashEqual(Data),
    /// Set only if the key exists and the hash of its current value matches the provided hash.
    PresentAndHashEqual(Data),
    /// Set if the key does not exist, or if it exists and the hash of its current value matches the provided hash.
    AbsentOrHashEqual(Data),
    /// Set if the key does not exist, or if it exists and the hash of its current value does not match the provided hash.
    AbsentOrNotHashEqual(Data),
    /// Unconditionally set the value.
    Unconditional,
}

impl From<SetIfHashCondition> for cache_scalar::SetIfHashCondition {
    fn from(value: SetIfHashCondition) -> Self {
        match value {
            SetIfHashCondition::PresentAndNotHashEqual(data) => {
                Self::PresentAndNotHashEqual(data.into())
            }
            SetIfHashCondition::PresentAndHashEqual(data) => Self::PresentAndHashEqual(data.into()),
            SetIfHashCondition::AbsentOrHashEqual(data) => Self::AbsentOrHashEqual(data.into()),
            SetIfHashCondition::AbsentOrNotHashEqual(data) => {
                Self::AbsentOrNotHashEqual(data.into())
            }
            SetIfHashCondition::Unconditional => Self::Unconditional,
        }
    }
}

/// Result of a set-if-hash operation.
pub enum SetIfHashResult {
    /// The value was stored. Contains the hash computed on the newly stored value.
    Stored(Vec<u8>),
    /// The value was not stored because the condition was not met.
    NotStored,
}

impl From<cache_scalar::SetIfHashResult> for SetIfHashResult {
    fn from(value: cache_scalar::SetIfHashResult) -> Self {
        match value {
            cache_scalar::SetIfHashResult::Stored(data) => {
                Self::Stored(Data::from(data).into_bytes())
            }
            cache_scalar::SetIfHashResult::NotStored => Self::NotStored,
        }
    }
}

/// A value retrieved from the cache along with its hash.
pub struct GetWithHashValue<T> {
    /// The extracted value.
    pub value: T,
    /// The hash of the value.
    pub hash: Vec<u8>,
}
