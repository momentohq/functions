use std::collections::HashMap;

use serde_json::{Map, Value};

/// Primary-key attribute names per table, from the `KEY_ATTRIBUTES` env var.
///
/// PutItem carries a whole `Item` rather than a `Key`, so the Function needs to
/// know which attributes make up the key to find the cache entry to delete.
/// Format: `table=pk[,sk];other_table=id`.
#[derive(Debug, Default)]
pub struct KeyAttributes {
    tables: HashMap<String, Vec<String>>,
}

impl KeyAttributes {
    pub fn from_env() -> Self {
        Self::parse(&std::env::var("KEY_ATTRIBUTES").unwrap_or_default())
    }

    pub fn parse(spec: &str) -> Self {
        let tables = spec
            .split(';')
            .filter_map(|entry| {
                let (table, attributes) = entry.split_once('=')?;
                let attributes: Vec<String> = attributes
                    .split(',')
                    .map(str::trim)
                    .filter(|a| !a.is_empty())
                    .map(String::from)
                    .collect();
                (!attributes.is_empty()).then(|| (table.trim().to_string(), attributes))
            })
            .collect();
        Self { tables }
    }

    /// Project an item down to its primary key. `None` if the table is not
    /// configured or the item is missing a key attribute.
    pub fn key_from_item(&self, table_name: &str, item: &Value) -> Option<Value> {
        let item = item.as_object()?;
        let key = self
            .tables
            .get(table_name)?
            .iter()
            .map(|name| Some((name.clone(), item.get(name)?.clone())))
            .collect::<Option<Map<String, Value>>>()?;
        Some(Value::Object(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_tables() {
        let attrs = KeyAttributes::parse("orders=pk, sk;users=id;bad;empty=");
        assert_eq!(attrs.tables["orders"], vec!["pk", "sk"]);
        assert_eq!(attrs.tables["users"], vec!["id"]);
        assert_eq!(attrs.tables.len(), 2);
    }

    #[test]
    fn projects_item_to_key() {
        let attrs = KeyAttributes::parse("orders=pk,sk");
        let item: Value =
            serde_json::from_str(r#"{"pk":{"S":"a"},"sk":{"N":"1"},"total":{"N":"9"}}"#)
                .expect("valid test JSON");
        let key: Value =
            serde_json::from_str(r#"{"pk":{"S":"a"},"sk":{"N":"1"}}"#).expect("valid test JSON");
        assert_eq!(attrs.key_from_item("orders", &item), Some(key));
    }

    #[test]
    fn unknown_table_or_missing_attribute_is_none() {
        let attrs = KeyAttributes::parse("orders=pk,sk");
        let item: Value = serde_json::from_str(r#"{"pk":{"S":"a"}}"#).expect("valid test JSON");
        assert_eq!(attrs.key_from_item("orders", &item), None);
        assert_eq!(attrs.key_from_item("users", &item), None);
    }
}
