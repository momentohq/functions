#!/usr/bin/env bash
# Deletes the e2e table and federated role. The Functions are left in place.
set -euo pipefail
aws dynamodb delete-table --table-name "$TABLE_NAME" >/dev/null
aws iam delete-role-policy --role-name "$ROLE_NAME" --policy-name dynamodb-table-access
aws iam delete-role --role-name "$ROLE_NAME"
