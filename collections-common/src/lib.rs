//! Common types shared across Momento Cache collection interfaces.

use std::time::Duration;

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

/// Saturate a Duration to u64 milliseconds, clamping at u64::MAX.
pub fn saturate_ttl(ttl: Duration) -> u64 {
    ttl.as_millis().min(u64::MAX as u128) as u64
}
