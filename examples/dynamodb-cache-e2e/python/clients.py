"""boto3 DynamoDB clients for each way of reaching the caching Functions."""

import os

import boto3


def direct_client():
    """Talks to DynamoDB directly, bypassing the Function and its cache."""
    return boto3.Session().client("dynamodb")


def proxy_client(function_url, momento_api_key, config=None):
    """Signs with the caller's own AWS credentials, then reroutes to the Function.

    `before-send` fires after SigV4 signing, so the signature still covers the
    real DynamoDB host. The Function reads that URL back from `x-uri`.
    """
    client = boto3.Session().client("dynamodb", config=config)

    def point_at_function(request, **_):
        request.headers["x-uri"] = request.url
        request.headers["x-momento-authorization"] = momento_api_key
        request.url = function_url

    client.meta.events.register("before-send.dynamodb", point_at_function)
    return client


def federated_client(function_url, momento_api_key, config=None):
    """Points the endpoint at the Function; the Function signs as its IAM role.

    The SDK insists on signing, so it gets placeholder credentials that the
    Function strips before re-signing.
    """
    client = boto3.Session().client(
        "dynamodb",
        endpoint_url=function_url,
        aws_access_key_id="placeholder",
        aws_secret_access_key="placeholder",
        config=config,
    )

    def add_momento_auth(request, **_):
        request.headers["x-momento-authorization"] = momento_api_key
        # botocore appends DynamoDB's "/" request path to the endpoint.
        request.url = function_url

    client.meta.events.register("before-send.dynamodb", add_momento_auth)
    return client


def function_client(mode, config=None, momento_api_key=None):
    key = momento_api_key or os.environ["MOMENTO_API_KEY"]
    if mode == "proxy":
        return proxy_client(os.environ["PROXY_FUNCTION_URL"], key, config)
    return federated_client(os.environ["FEDERATED_FUNCTION_URL"], key, config)
