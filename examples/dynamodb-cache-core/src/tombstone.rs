use std::time::Duration;

use momento_functions_cache as cache;

/// Stored in place of a cached response when a write invalidates an item.
const TOMBSTONE: &[u8] = b"ddb-cache:tombstone";

/// How long a written item stays uncacheable. DynamoDB's eventually consistent
/// reads usually converge within a second; until then a GetItem miss could
/// read the pre-write item and pin it in the cache for the full TTL.
const HOLD: Duration = Duration::from_secs(2);

/// Replace an item's cache entry with a short-lived tombstone. GetItem fills
/// only store into an absent key, so neither a post-write eventually consistent
/// read nor a fill that raced the write can re-cache the old item.
pub fn invalidate(key: Vec<u8>) {
    if let Err(e) = cache::set(key.clone(), TOMBSTONE.to_vec(), HOLD) {
        log::warn!("tombstone set failed, deleting instead: {e}");
        if let Err(e) = cache::delete(key) {
            log::warn!("cache delete failed: {e}");
        }
    }
}

pub fn is_tombstone(value: &[u8]) -> bool {
    value == TOMBSTONE
}
