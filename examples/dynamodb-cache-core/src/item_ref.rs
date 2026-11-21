use serde_json::Value;

/// One DynamoDB item, identified by table and primary key.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemRef {
    pub table_name: String,
    pub key: Value,
}

impl ItemRef {
    /// The cache key holding this item's GetItem response.
    pub fn cache_key(&self) -> Vec<u8> {
        crate::cache_key::cache_key(&self.table_name, &self.key)
    }
}
