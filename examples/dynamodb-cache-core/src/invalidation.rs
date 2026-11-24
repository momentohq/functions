use serde_json::Value;

use crate::{ItemRef, KeyAttributes, WriteAction};

/// The items a write may have changed, whose cache entries must be invalidated.
///
/// Puts only carry the full `Item`, so their key comes from [`KeyAttributes`];
/// a put on an unconfigured table is logged and skipped.
pub fn items_to_invalidate(
    action: WriteAction,
    request: &Value,
    key_attributes: &KeyAttributes,
) -> Vec<ItemRef> {
    match action {
        WriteAction::UpdateItem | WriteAction::DeleteItem => {
            keyed_item_ref(request).into_iter().collect()
        }
        WriteAction::PutItem => put_request_ref(request, key_attributes)
            .into_iter()
            .collect(),
        WriteAction::BatchWriteItem => batch_write_items(request, key_attributes),
        WriteAction::TransactWriteItems => transact_write_items(request, key_attributes),
    }
}

/// `{TableName, Key}`, as in UpdateItem, DeleteItem and their transact forms.
fn keyed_item_ref(request: &Value) -> Option<ItemRef> {
    Some(ItemRef {
        table_name: request.get("TableName")?.as_str()?.to_string(),
        key: request.get("Key")?.clone(),
    })
}

/// `{TableName, Item}`, as in PutItem and a transact Put.
fn put_request_ref(request: &Value, key_attributes: &KeyAttributes) -> Option<ItemRef> {
    let table_name = request.get("TableName")?.as_str()?;
    put_item_ref(table_name, request.get("Item")?, key_attributes)
}

fn put_item_ref(table_name: &str, item: &Value, key_attributes: &KeyAttributes) -> Option<ItemRef> {
    let Some(key) = key_attributes.key_from_item(table_name, item) else {
        log::warn!("no KEY_ATTRIBUTES for table {table_name}; cannot invalidate PutItem");
        return None;
    };
    Some(ItemRef {
        table_name: table_name.to_string(),
        key,
    })
}

fn batch_write_items(request: &Value, key_attributes: &KeyAttributes) -> Vec<ItemRef> {
    let Some(tables) = request.get("RequestItems").and_then(Value::as_object) else {
        return Vec::new();
    };
    tables
        .iter()
        .flat_map(|(table_name, writes)| {
            writes
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(move |write| {
                    if let Some(key) = write.pointer("/DeleteRequest/Key") {
                        Some(ItemRef {
                            table_name: table_name.clone(),
                            key: key.clone(),
                        })
                    } else {
                        let item = write.pointer("/PutRequest/Item")?;
                        put_item_ref(table_name, item, key_attributes)
                    }
                })
        })
        .collect()
}

/// Puts, Updates and Deletes in `TransactItems`. ConditionChecks write nothing.
fn transact_write_items(request: &Value, key_attributes: &KeyAttributes) -> Vec<ItemRef> {
    let Some(items) = request.get("TransactItems").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            if let Some(put) = item.get("Put") {
                put_request_ref(put, key_attributes)
            } else {
                keyed_item_ref(item.get("Update").or_else(|| item.get("Delete"))?)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).expect("valid test JSON")
    }

    fn item(table_name: &str, key: &str) -> ItemRef {
        ItemRef {
            table_name: table_name.to_string(),
            key: parse(key),
        }
    }

    #[test]
    fn update_and_delete_use_key() {
        let attrs = KeyAttributes::default();
        let request = parse(r#"{"TableName":"t","Key":{"pk":{"S":"a"}},"UpdateExpression":"x"}"#);
        for action in [WriteAction::UpdateItem, WriteAction::DeleteItem] {
            assert_eq!(
                items_to_invalidate(action, &request, &attrs),
                vec![item("t", r#"{"pk":{"S":"a"}}"#)]
            );
        }
    }

    #[test]
    fn put_item_projects_key_attributes() {
        let attrs = KeyAttributes::parse("t=pk");
        let request = parse(r#"{"TableName":"t","Item":{"pk":{"S":"a"},"v":{"N":"1"}}}"#);
        assert_eq!(
            items_to_invalidate(WriteAction::PutItem, &request, &attrs),
            vec![item("t", r#"{"pk":{"S":"a"}}"#)]
        );
    }

    #[test]
    fn put_item_on_unconfigured_table_is_skipped() {
        let request = parse(r#"{"TableName":"t","Item":{"pk":{"S":"a"}}}"#);
        let attrs = KeyAttributes::default();
        assert!(items_to_invalidate(WriteAction::PutItem, &request, &attrs).is_empty());
    }

    #[test]
    fn batch_write_covers_puts_and_deletes_across_tables() {
        let attrs = KeyAttributes::parse("t1=pk;t2=id");
        let request = parse(
            r#"{"RequestItems":{
                "t1":[{"PutRequest":{"Item":{"pk":{"S":"a"},"v":{"N":"1"}}}},
                      {"DeleteRequest":{"Key":{"pk":{"S":"b"}}}}],
                "t2":[{"PutRequest":{"Item":{"id":{"S":"c"}}}}]
            }}"#,
        );
        assert_eq!(
            items_to_invalidate(WriteAction::BatchWriteItem, &request, &attrs),
            vec![
                item("t1", r#"{"pk":{"S":"a"}}"#),
                item("t1", r#"{"pk":{"S":"b"}}"#),
                item("t2", r#"{"id":{"S":"c"}}"#),
            ]
        );
    }

    #[test]
    fn transact_write_covers_put_update_delete_but_not_condition_check() {
        let attrs = KeyAttributes::parse("t=pk");
        let request = parse(
            r#"{"TransactItems":[
                {"Put":{"TableName":"t","Item":{"pk":{"S":"a"},"v":{"N":"1"}}}},
                {"Update":{"TableName":"t","Key":{"pk":{"S":"b"}},"UpdateExpression":"x"}},
                {"Delete":{"TableName":"t","Key":{"pk":{"S":"c"}}}},
                {"ConditionCheck":{"TableName":"t","Key":{"pk":{"S":"d"}},"ConditionExpression":"x"}}
            ]}"#,
        );
        assert_eq!(
            items_to_invalidate(WriteAction::TransactWriteItems, &request, &attrs),
            vec![
                item("t", r#"{"pk":{"S":"a"}}"#),
                item("t", r#"{"pk":{"S":"b"}}"#),
                item("t", r#"{"pk":{"S":"c"}}"#),
            ]
        );
    }
}
