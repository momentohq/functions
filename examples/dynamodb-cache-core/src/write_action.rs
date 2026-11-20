/// A DynamoDB write whose items the Function invalidates once it succeeds.
///
/// PartiQL writes (`ExecuteStatement`, `BatchExecuteStatement`,
/// `ExecuteTransaction`) are not here: their keys are inside SQL text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteAction {
    PutItem,
    UpdateItem,
    DeleteItem,
    BatchWriteItem,
    TransactWriteItems,
}

impl WriteAction {
    /// Parse an `X-Amz-Target` header value; `None` for anything that isn't a
    /// write this Function invalidates.
    pub fn from_target(target: &str) -> Option<Self> {
        match target {
            "DynamoDB_20120810.PutItem" => Some(Self::PutItem),
            "DynamoDB_20120810.UpdateItem" => Some(Self::UpdateItem),
            "DynamoDB_20120810.DeleteItem" => Some(Self::DeleteItem),
            "DynamoDB_20120810.BatchWriteItem" => Some(Self::BatchWriteItem),
            "DynamoDB_20120810.TransactWriteItems" => Some(Self::TransactWriteItems),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_writes_only() {
        assert_eq!(
            WriteAction::from_target("DynamoDB_20120810.TransactWriteItems"),
            Some(WriteAction::TransactWriteItems)
        );
        assert_eq!(WriteAction::from_target("DynamoDB_20120810.GetItem"), None);
        assert_eq!(
            WriteAction::from_target("DynamoDB_20120810.ExecuteStatement"),
            None
        );
    }
}
