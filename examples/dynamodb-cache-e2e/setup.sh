#!/usr/bin/env bash
# Creates the e2e table and the IAM role the federated Function assumes.
set -euo pipefail

account_id=$(aws sts get-caller-identity --query Account --output text)

aws dynamodb create-table \
  --table-name "$TABLE_NAME" \
  --attribute-definitions AttributeName=pk,AttributeType=S \
  --key-schema AttributeName=pk,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST >/dev/null
aws dynamodb wait table-exists --table-name "$TABLE_NAME"

# Momento's hosts chain through a central role (FEDERATION_PRINCIPAL) and assume
# this role from there with sts:ExternalId "<momento account id>/<function id>"
# (older mr2 builds, e.g. 2bf33515, send "<momento account id>:<function id>").
# This is the same trust a customer's role needs, with their environment's
# central role.
trust_policy=$(cat <<JSON
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": {"AWS": "$FEDERATION_PRINCIPAL"},
    "Action": "sts:AssumeRole",
    "Condition": {"StringLike": {"sts:ExternalId": ["$MOMENTO_ACCOUNT_ID/*", "$MOMENTO_ACCOUNT_ID:*"]}}
  }]
}
JSON
)
aws iam create-role --role-name "$ROLE_NAME" --assume-role-policy-document "$trust_policy" >/dev/null

aws iam put-role-policy --role-name "$ROLE_NAME" --policy-name dynamodb-table-access \
  --policy-document "$(cat <<JSON
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Action": "dynamodb:*",
    "Resource": "arn:aws:dynamodb:$AWS_REGION:$account_id:table/$TABLE_NAME"
  }]
}
JSON
)"

