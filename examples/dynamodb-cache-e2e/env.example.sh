# Copy to env.sh, fill in, and `source env.sh` before setup/deploy/tests.
export AWS_PROFILE=dev                     # account that owns the table + federated role
export AWS_REGION=us-west-2
# Toolchain for building the Functions; blank = default. Cells whose mr2 predates
# momentohq/mr2rs#3703 (2026-11-12) need Rust < 1.100, e.g. 1.89-aarch64-apple-darwin.
export FUNCTIONS_TOOLCHAIN=
export TABLE_NAME=ddb-cache-e2e
export ROLE_NAME=ddb-cache-e2e-federated
# Role Momento chains through to assume ROLE_NAME (preprod; prod differs).
export FEDERATION_PRINCIPAL=arn:aws:iam::534466876564:role/central-assume-role-preprod

export MOMENTO_API_KEY=                    # API key for the dev cell
export MOMENTO_ENDPOINT=                   # cell hostname, e.g. cell-us-west-2-1.<...>.momentohq.com
export MOMENTO_CACHE=taco                  # cache the Functions are namespaced under
export MOMENTO_ACCOUNT_ID='*'              # narrows the role's sts:ExternalId to "<id>/*" if known

export PROXY_FUNCTION=ddb-cache-proxy
export FEDERATED_FUNCTION=ddb-cache-federated
export PROXY_FUNCTION_URL="https://api.cache.$MOMENTO_ENDPOINT/functions/$MOMENTO_CACHE/$PROXY_FUNCTION"
export FEDERATED_FUNCTION_URL="https://api.cache.$MOMENTO_ENDPOINT/functions/$MOMENTO_CACHE/$FEDERATED_FUNCTION"
