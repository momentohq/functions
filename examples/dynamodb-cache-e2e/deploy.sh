#!/usr/bin/env bash
# Builds both Functions and deploys them to the dev cell.
set -euo pipefail
cd "$(dirname "$0")/../.."

# Rust >= 1.100 emits wasi:cli/exit@0.2.12 exit-with-code, which only mr2 builds
# with momentohq/mr2rs#3703 can link. Pin an older toolchain for cells without it.
toolchain=${FUNCTIONS_TOOLCHAIN:+"+$FUNCTIONS_TOOLCHAIN"}
target_dir=target/${FUNCTIONS_TOOLCHAIN:-default}
cargo $toolchain build --release --target-dir "$target_dir" \
  -p example-dynamodb-cache-proxy -p example-dynamodb-cache-federated

# Read both from what setup.sh created, so the Functions can't drift from it.
role_arn=$(aws iam get-role --role-name "$ROLE_NAME" --query Role.Arn --output text)
key_attributes="$TABLE_NAME=$(aws dynamodb describe-table --table-name "$TABLE_NAME" \
  --query 'Table.KeySchema[].AttributeName' --output text | tr '\t' ',')"

momento preview function put-function \
  --api-key "$MOMENTO_API_KEY" --endpoint "$MOMENTO_ENDPOINT" --cache-name "$MOMENTO_CACHE" \
  --name "$PROXY_FUNCTION" \
  --wasm-file "$target_dir"/wasm32-wasip2/release/example_dynamodb_cache_proxy.wasm \
  -E "KEY_ATTRIBUTES=$key_attributes"

momento preview function put-function \
  --api-key "$MOMENTO_API_KEY" --endpoint "$MOMENTO_ENDPOINT" --cache-name "$MOMENTO_CACHE" \
  --name "$FEDERATED_FUNCTION" \
  --wasm-file "$target_dir"/wasm32-wasip2/release/example_dynamodb_cache_federated.wasm \
  -E "KEY_ATTRIBUTES=$key_attributes" -E "ROLE_ARN=$role_arn" -E "DYNAMODB_REGION=$AWS_REGION"
