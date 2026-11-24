"""End-to-end checks for the DynamoDB caching Functions, run against both."""

import os

import pytest
from botocore.config import Config
from botocore.exceptions import ClientError

from clients import function_client, proxy_client


def value(response):
    return response["Item"]["v"]["S"] if "Item" in response else None


def put_direct(ddb, table, pk, v):
    ddb.put_item(TableName=table, Item={**pk, "v": {"S": v}})


# Passes the Function's DynamoDB-endpoint check but doesn't resolve, so the
# forward fails with a 5xx. Only the proxy Function takes its upstream from
# the client (x-uri), so tests that need a failing upstream are proxy-only.
NOWHERE_DYNAMODB = "https://dynamodb.xx-nowhere-1.amazonaws.com/"


def proxy_with_upstream(upstream, total_max_attempts):
    client = proxy_client(
        os.environ["PROXY_FUNCTION_URL"],
        os.environ["MOMENTO_API_KEY"],
        Config(retries={"mode": "standard", "total_max_attempts": total_max_attempts}),
    )

    def override_upstream(request, **_):
        request.headers["x-uri"] = upstream

    # Registered after the proxy hook, so it overrides x-uri.
    client.meta.events.register("before-send.dynamodb", override_upstream)
    return client


def test_read_miss_then_hit(fn, ddb, table, pk):
    put_direct(ddb, table, pk, "v1")
    assert value(fn.get_item(TableName=table, Key=pk)) == "v1"  # miss, fills cache

    # Change the item behind the Function's back: only a cache hit still sees v1.
    put_direct(ddb, table, pk, "v2")
    assert value(fn.get_item(TableName=table, Key=pk)) == "v1"

    # Strongly consistent and projected reads bypass the cache.
    assert value(fn.get_item(TableName=table, Key=pk, ConsistentRead=True)) == "v2"
    projected = fn.get_item(TableName=table, Key=pk, ProjectionExpression="v")
    assert projected["Item"] == {"v": {"S": "v2"}}
    # A cached response has no ConsumedCapacity, so this must read through too.
    measured = fn.get_item(TableName=table, Key=pk, ReturnConsumedCapacity="TOTAL")
    assert value(measured) == "v2"
    assert measured["ConsumedCapacity"]["TableName"] == table


def test_put_item_invalidates(fn, ddb, table, pk):
    put_direct(ddb, table, pk, "v1")
    fn.get_item(TableName=table, Key=pk)
    fn.put_item(TableName=table, Item={**pk, "v": {"S": "v2"}})
    assert value(fn.get_item(TableName=table, Key=pk)) == "v2"


def test_update_item_invalidates(fn, ddb, table, pk):
    put_direct(ddb, table, pk, "v1")
    fn.get_item(TableName=table, Key=pk)
    fn.update_item(
        TableName=table,
        Key=pk,
        UpdateExpression="SET v = :v",
        ExpressionAttributeValues={":v": {"S": "v2"}},
    )
    assert value(fn.get_item(TableName=table, Key=pk)) == "v2"


def test_delete_item_invalidates(fn, ddb, table, pk):
    put_direct(ddb, table, pk, "v1")
    fn.get_item(TableName=table, Key=pk)
    fn.delete_item(TableName=table, Key=pk)
    assert value(fn.get_item(TableName=table, Key=pk)) is None


def test_batch_write_item_invalidates(fn, ddb, table, pk):
    other = {"pk": {"S": pk["pk"]["S"] + "-other"}}
    put_direct(ddb, table, pk, "v1")
    put_direct(ddb, table, other, "v1")
    fn.get_item(TableName=table, Key=pk)
    fn.get_item(TableName=table, Key=other)

    fn.batch_write_item(
        RequestItems={
            table: [
                {"PutRequest": {"Item": {**pk, "v": {"S": "v2"}}}},
                {"DeleteRequest": {"Key": other}},
            ]
        }
    )
    assert value(fn.get_item(TableName=table, Key=pk)) == "v2"
    assert value(fn.get_item(TableName=table, Key=other)) is None


def test_missing_item_then_put_creates_it(fn, table, pk):
    # The empty GetItem response is a 2xx and gets cached...
    assert value(fn.get_item(TableName=table, Key=pk)) is None
    # ...so the PutItem must find it via KEY_ATTRIBUTES and delete it.
    fn.put_item(TableName=table, Item={**pk, "v": {"S": "created"}})
    assert value(fn.get_item(TableName=table, Key=pk)) == "created"


def test_errors_reach_sdk_intact(mode, fn, ddb, table, pk):
    # The federated role may only touch the e2e table, so IAM denies the call
    # before DynamoDB can report the table missing.
    expected = "AccessDeniedException" if mode == "federated" else "ResourceNotFoundException"
    with pytest.raises(ClientError) as e:
        fn.get_item(TableName="ddb-cache-e2e-does-not-exist", Key=pk)
    assert e.value.response["Error"]["Code"] == expected

    with pytest.raises(ClientError) as e:
        fn.get_item(TableName=table, Key={"wrong": {"S": "x"}})
    assert e.value.response["Error"]["Code"] == "ValidationException"

    put_direct(ddb, table, pk, "v1")
    with pytest.raises(ClientError) as e:
        fn.put_item(
            TableName=table,
            Item={**pk, "v": {"S": "v2"}},
            ConditionExpression="attribute_not_exists(pk)",
            ReturnValuesOnConditionCheckFailure="ALL_OLD",
        )
    assert e.value.response["Error"]["Code"] == "ConditionalCheckFailedException"
    assert e.value.response["Item"]["v"]["S"] == "v1"


def test_errors_are_not_cached(ddb, table, pk):
    """A failed GetItem must not be cached: the same request succeeds once the
    upstream works again. Proxy-only (needs a failing upstream)."""
    put_direct(ddb, table, pk, "v1")
    with pytest.raises(ClientError) as e:
        proxy_with_upstream(NOWHERE_DYNAMODB, 1).get_item(TableName=table, Key=pk)
    assert e.value.response["ResponseMetadata"]["HTTPStatusCode"] >= 500

    working = function_client("proxy")
    assert value(working.get_item(TableName=table, Key=pk)) == "v1"


def test_transact_write_items_invalidates(fn, ddb, table, pk):
    other = {"pk": {"S": pk["pk"]["S"] + "-other"}}
    put_direct(ddb, table, pk, "v1")
    put_direct(ddb, table, other, "v1")
    fn.get_item(TableName=table, Key=pk)
    fn.get_item(TableName=table, Key=other)

    fn.transact_write_items(
        TransactItems=[
            {"Put": {"TableName": table, "Item": {**pk, "v": {"S": "v2"}}}},
            {
                "Update": {
                    "TableName": table,
                    "Key": other,
                    "UpdateExpression": "SET v = :v",
                    "ExpressionAttributeValues": {":v": {"S": "v2"}},
                }
            },
        ]
    )
    assert value(fn.get_item(TableName=table, Key=pk)) == "v2"
    assert value(fn.get_item(TableName=table, Key=other)) == "v2"


def test_retries_reach_sdk():
    """A 5xx from the Function is retried by the SDK like any DynamoDB 5xx."""
    with pytest.raises(ClientError) as e:
        proxy_with_upstream(NOWHERE_DYNAMODB, 3).list_tables()
    assert e.value.response["ResponseMetadata"]["HTTPStatusCode"] >= 500
    assert e.value.response["ResponseMetadata"]["RetryAttempts"] == 2


def test_proxy_rejects_non_dynamodb_upstream(table, pk):
    """Responses get cached, so the proxy must only forward to DynamoDB; any
    other x-uri is a non-retryable 400."""
    with pytest.raises(ClientError) as e:
        proxy_with_upstream("https://example.com/", 3).get_item(TableName=table, Key=pk)
    assert e.value.response["ResponseMetadata"]["HTTPStatusCode"] == 400
    assert e.value.response["Error"]["Code"] == "InvalidUpstreamException"
    assert e.value.response["ResponseMetadata"]["RetryAttempts"] == 0


def test_momento_auth_required(mode, table, pk):
    """Momento rejects a bad x-momento-authorization before the Function runs,
    so an unauthenticated caller can never read the cache."""
    client = function_client(
        mode, Config(retries={"max_attempts": 1}), momento_api_key="not-a-valid-momento-key"
    )
    with pytest.raises(ClientError) as e:
        client.get_item(TableName=table, Key=pk)
    assert 400 <= e.value.response["ResponseMetadata"]["HTTPStatusCode"] < 500
