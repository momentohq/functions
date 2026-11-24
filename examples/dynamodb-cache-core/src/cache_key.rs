use serde_json::{Value, json};

/// The cache key for one DynamoDB item: its table name and primary key.
///
/// `serde_json` stores objects in a sorted map (the workspace does not enable
/// `preserve_order`), so a key built from a GetItem `Key` and one built from a
/// PutItem `Item` serialize identically regardless of attribute order.
pub fn cache_key(table_name: &str, key: &Value) -> Vec<u8> {
    let mut bytes = b"ddb:".to_vec();
    bytes.extend(
        json!({ "TableName": table_name, "Key": key })
            .to_string()
            .into_bytes(),
    );
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_order_does_not_matter() {
        let a: Value =
            serde_json::from_str(r#"{"pk":{"S":"a"},"sk":{"N":"1"}}"#).expect("valid test JSON");
        let b: Value =
            serde_json::from_str(r#"{"sk":{"N":"1"},"pk":{"S":"a"}}"#).expect("valid test JSON");
        assert_eq!(cache_key("t", &a), cache_key("t", &b));
    }

    #[test]
    fn table_is_part_of_the_key() {
        let key: Value = serde_json::from_str(r#"{"pk":{"S":"a"}}"#).expect("valid test JSON");
        assert_ne!(cache_key("t1", &key), cache_key("t2", &key));
    }
}
