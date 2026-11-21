use serde_json::Value;

use crate::ItemRef;

/// The item a GetItem request reads, if the request is safe to serve from cache.
///
/// These pass straight through:
/// - strongly consistent reads, which must see the latest write;
/// - projected reads, which return a subset of the item that a write could not
///   find to invalidate;
/// - reads asking for `ConsumedCapacity`, which a cached response wouldn't carry.
pub fn cacheable_get_item(request: &Value) -> Option<ItemRef> {
    let wants_capacity = request
        .get("ReturnConsumedCapacity")
        .and_then(Value::as_str)
        .is_some_and(|c| c != "NONE");
    if request.get("ConsistentRead").and_then(Value::as_bool) == Some(true)
        || request.get("ProjectionExpression").is_some()
        || request.get("AttributesToGet").is_some()
        || wants_capacity
    {
        return None;
    }
    Some(ItemRef {
        table_name: request.get("TableName")?.as_str()?.to_string(),
        key: request.get("Key")?.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).expect("valid test JSON")
    }

    #[test]
    fn plain_get_is_cacheable() {
        let item = cacheable_get_item(&parse(r#"{"TableName":"t","Key":{"pk":{"S":"a"}}}"#));
        assert_eq!(item.expect("test value present").table_name, "t");
    }

    #[test]
    fn no_consumed_capacity_is_cacheable() {
        let request =
            parse(r#"{"TableName":"t","Key":{"pk":{"S":"a"}},"ReturnConsumedCapacity":"NONE"}"#);
        assert!(cacheable_get_item(&request).is_some());
    }

    #[test]
    fn eventually_consistent_flag_is_cacheable() {
        let request = parse(r#"{"TableName":"t","Key":{"pk":{"S":"a"}},"ConsistentRead":false}"#);
        assert!(cacheable_get_item(&request).is_some());
    }

    #[test]
    fn consistent_or_projected_reads_pass_through() {
        for request in [
            r#"{"TableName":"t","Key":{"pk":{"S":"a"}},"ConsistentRead":true}"#,
            r#"{"TableName":"t","Key":{"pk":{"S":"a"}},"ProjectionExpression":"pk"}"#,
            r#"{"TableName":"t","Key":{"pk":{"S":"a"}},"AttributesToGet":["pk"]}"#,
            r#"{"TableName":"t","Key":{"pk":{"S":"a"}},"ReturnConsumedCapacity":"TOTAL"}"#,
            r#"{"Key":{"pk":{"S":"a"}}}"#,
        ] {
            assert!(cacheable_get_item(&parse(request)).is_none(), "{request}");
        }
    }
}
