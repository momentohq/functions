import os
import uuid

import pytest

from clients import direct_client, function_client


@pytest.fixture(params=["proxy", "federated"])
def mode(request):
    return request.param


@pytest.fixture
def fn(mode):
    return function_client(mode)


@pytest.fixture
def ddb():
    return direct_client()


@pytest.fixture
def table():
    return os.environ["TABLE_NAME"]


@pytest.fixture
def pk():
    """A fresh item key per test, so cached entries never leak between tests."""
    return {"pk": {"S": f"e2e-{uuid.uuid4()}"}}
