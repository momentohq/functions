# DynamoDB caching Functions: proxy auth vs federated role

Two prototype Functions, sharing one caching core:

| | `dynamodb-cache-proxy` | `dynamodb-cache-federated` |
|---|---|---|
| Who signs for AWS | The client, with its own credentials | Momento, as `ROLE_ARN` (`Authorization::Federated`) |
| How the Function finds DynamoDB | `x-uri` header, set by the client; must be `https://dynamodb.<region>.amazonaws.com/` or `https://<account>.ddb.<region>.amazonaws.com/`, else 400 `InvalidUpstreamException` | `https://dynamodb.$DYNAMODB_REGION.amazonaws.com/` |
| Headers dropped before forwarding | Its routing headers (`STRIPPED_HEADERS` in `lib.rs`); the signature is kept | Its routing headers and the client's placeholder SigV4 headers (`STRIPPED_HEADERS` in `lib.rs`) |
| Env | `KEY_ATTRIBUTES` | `KEY_ATTRIBUTES`, `ROLE_ARN`, `DYNAMODB_REGION` |

Shared behavior (`examples/dynamodb-cache-core`):

- **GetItem**: served from cache, keyed by `{TableName, Key}`. Only 2xx responses are cached, for 60s, so errors and throttles always reach the SDK and its retry logic. `ConsistentRead: true`, `ProjectionExpression`, `AttributesToGet` and `ReturnConsumedCapacity` (other than `NONE`) pass straight through, since a cached response can't honor them. A miss only stores into an *absent* key (`set_if Absent`), so it never overwrites a write's tombstone (below).
- **PutItem / UpdateItem / DeleteItem / BatchWriteItem / TransactWriteItems**: forwarded, then each written item's entry is replaced by a 2s **tombstone**. A GetItem that finds a tombstone reads through without caching. This stops an eventually consistent read right after the write (or a fill that raced the write) from pinning the pre-write item for 60s. Puts carry no `Key`, so their key comes from `KEY_ATTRIBUTES` (`table=pk[,sk];other=id`).
  - Invalidation runs unless DynamoDB returned a **4xx**: a 5xx or a failed forward (e.g. a timeout after commit) may still have applied the write.
  - A failed cache write is logged, not returned, because an error would make the SDK retry a write that already happened.
- **PartiQL** (`ExecuteStatement`, `BatchExecuteStatement`, `ExecuteTransaction`): passed through with a warning. Its keys are inside SQL text, so **writes made through PartiQL are not invalidated**; clients must not mix PartiQL writes with cached GetItem.
- **Everything else**: passed through.
- A missing `X-Amz-Target` (or `x-uri` for the proxy) is a non-retryable 400 `MissingHeaderException`. A missing env var stays a 500: it's the Function's misconfiguration, not the caller's.
- DynamoDB's status, headers and body are returned unchanged (so `x-amz-crc32` still validates).
- Hop-by-hop request headers (`Connection`, `Keep-Alive`, `Transfer-Encoding`, ...) and `x-momento-authorization` are never forwarded (`headers::forwardable`, unit-tested). The federated signer signs every header it is given and the HTTP client then drops hop-by-hop ones, so forwarding Java's `Connection: Keep-Alive` made DynamoDB reject the signature.

## What each client needed

**boto3, proxy**
- A `before-send.dynamodb` event handler, which botocore fires *after* SigV4 signing. It copies `request.url` into `x-uri`, adds `x-momento-authorization`, and sets `request.url` to the Function URL. botocore re-signs and re-fires the hook on every retry, so retries work unchanged.

**boto3, federated**
- `endpoint_url=<Function URL>` and placeholder credentials (`aws_access_key_id="placeholder"`). The SDK always signs, and the Function strips that signature.
- A `before-send.dynamodb` handler to add `x-momento-authorization`. It also resets `request.url` to the exact Function URL, because botocore appends DynamoDB's `/` path to the endpoint.

**Java SDK v2, proxy**
- There is no interceptor hook that runs after signing and can still change the request: `ExecutionInterceptor.modifyHttpRequest` runs before signing. The rewrite instead lives in a wrapping `SdkHttpClient` (`ProxyHttpClient`), which only ever sees the signed request.
- The wrapper must also remove the signed `Host` header (v2 sets it explicitly to the DynamoDB host); otherwise the request is sent to Momento with DynamoDB's Host.
- `HttpExecuteRequest` has no `toBuilder()`, so it is rebuilt with the same body stream.
- Don't use `SdkHttpRequest.Builder.uri(functionUri)`: it *appends* the URI's path to the existing one (`/functions/c/f/functions/c/f/`). Set protocol, host, port and `encodedPath` explicitly (`FunctionUri.retarget`).

**Java SDK v2, federated**
- `endpointOverride(<Function URL>)`, `StaticCredentialsProvider` with placeholder credentials, and an `ExecutionInterceptor` whose `modifyHttpRequest` adds `x-momento-authorization` (and pins the exact URL via `FunctionUri.retarget`, same trailing-slash reason as boto3).
- Apache's HTTP/1.1 client sends `Connection: Keep-Alive`, which the Function now strips (see above). This needs no client change.

## Setting up the federated role

- **Trust principal**: Momento's hosts don't assume the customer's role directly. mr2 first assumes a central role (`arn:aws:iam::534466876564:role/central-assume-role-preprod` in preprod) and assumes the customer's role from there. The customer's trust policy must name that central role, not the cell's account.
- **External ID**: `<momento account id>/<function id>` in current mr2rs, but `<momento account id>:<function id>` (colon) in older builds such as `2bf33515`, which is what the dev cell runs. The e2e role accepts both (`StringLike` on `acct/*` and `acct:*`). Customers on prod should be told which format their cell sends.
- IAM trust-policy changes took a few minutes to apply. During that window the Function returned `Failed to assume role`, and mr2 logs nothing more specific. CloudTrail shows no hop-2 `AssumeRole` event in the role's account while the trust policy is still rejecting.

## Security notes

- Momento authenticates `x-momento-authorization` before the Function runs; both test suites assert a bad key is rejected with a 4xx.
- **The proxy only forwards to DynamoDB endpoints.** Without that check, a Momento key holder could point `x-uri` at their own server and plant arbitrary items in the cache, which the federated Function would then serve too (both share the cache and key format).
- **Proxy cache hits skip AWS authorization.** Only DynamoDB checks the client's SigV4 signature. On a hit, DynamoDB is never called, so any holder of a Momento key for that cache can read items cached in the last 60s, even if their AWS identity can't read that table. On reads, proxy auth is effectively Momento auth, the same as federated. This is fine when Momento key holders and AWS-authorized callers are the same set (e.g. one Spark job, one key). It is not fine when callers with different IAM permissions share a key.
- **Federated: a Momento key carries the role's full DynamoDB permissions.** The Function forwards any action, so anyone who can invoke it can do whatever `ROLE_ARN` allows. The e2e role has `dynamodb:*` on its table, which includes DeleteTable and Scan. A customer role should grant only the actions their clients need (e.g. `GetItem`, `PutItem`, `UpdateItem`, `DeleteItem`, `BatchWriteItem`).
- **Number keys aren't normalized.** Cache keys use the request JSON as sent, so `{"N":"1.0"}` and `{"N":"1"}` name the same DynamoDB item but different cache entries; a write sent with one form doesn't invalidate a read cached with the other. SDKs send numbers as the application formats them, so this only bites when clients format the same key differently. The e2e table uses a string key, so this isn't exercised.
- The cache key has no AWS account or region. Two same-named tables in different accounts/regions reached through the same proxy Function would share entries. Include the `x-uri` host in the key if one Function will serve more than one account.
- Remaining staleness window: a GetItem fill that started before a write but stores *after* the write's 2s tombstone expires (i.e. a read slower than 2s) can still cache the pre-write item.

## Customer's Spark job: which DynamoDB writer?

Not determinable from our side. To ask the customer: *"Which library does the Spark job use to write to DynamoDB? Can you share the DynamoDB-related lines of `build.sbt` and the write call?"*

Likely candidates and what each would need (config names are from memory and should be checked against each library's source once we know which one):

| Writer | AWS SDK | How it would hook in |
|---|---|---|
| `emr-dynamodb-connector` (`DynamoDBOutputFormat`, `saveAsHadoopDataset`) | v1 | Only `dynamodb.endpoint` is configurable through job conf. Federated works by setting that endpoint, but there's no hook to add `x-momento-authorization` without a custom `DynamoDBClient`/credentials patch. Proxy is not possible without forking. |
| `audienceproject/spark-dynamodb` (`.write.dynamodb(...)`) | v1 | Endpoint can be overridden via a system property (`aws.dynamodb.endpoint`); adding a header needs a v1 `RequestHandler2` registered on the client, which the library doesn't expose. Likely needs a fork. |
| Hand-written SDK v2 in `foreachPartition` | v2 | Reuse the Java setup above directly: `ProxyHttpClient` or `endpointOverride` + `MomentoAuthInterceptor`. |
| Hand-written SDK v1 | v1 | Federated: `setEndpoint` + a `RequestHandler2` adding the header. Proxy: v1 signs inside the HTTP layer, so it needs a custom `HttpClient`/`ConnectionSocketFactory`; not attempted. |

Writes go through the Function only to keep the cache coherent. If the Spark job writes but never reads through the cache, it can keep writing directly to DynamoDB, with the readers relying on the 60s TTL instead.

## E2E results (developer-benkug, us-west-2, 2026-11-19)

Run against cache `ddb-cache-e2e` on account `a-QQfcqjVnMfPe`, table `ddb-cache-e2e`, both Functions at version 3.

| Suite | Proxy | Federated | Proxy-only (failing upstream) | Total |
|---|---|---|---|---|
| Python (boto3 1.44.7) | 9/9 | 9/9 | 3/3 | **21/21** |
| Java (SDK v2 2.31.0, Apache client) | 9/9 | 9/9 | 3/3 | **21/21** |

Each mode covers: read miss then hit (proved by changing the item directly in DynamoDB), consistent/projected reads bypassing the cache, invalidation after PutItem / UpdateItem / DeleteItem / BatchWriteItem / TransactWriteItems, GetItem on a missing item followed by a PutItem that creates it, error codes passed through intact (`ResourceNotFoundException`, or `AccessDeniedException` for the table-scoped federated role; `ValidationException`; `ConditionalCheckFailedException` with `ALL_OLD` item), and a bad Momento key rejected with 4xx.

The proxy-only tests point `x-uri` at `dynamodb.xx-nowhere-1.amazonaws.com` (passes the endpoint check, fails DNS → 5xx):
- **Errors are not cached**: a GetItem fails with a 5xx, then the same GetItem through a working upstream returns the item.
- **Retries reach the SDK**: the 5xx is retried (boto3 `RetryAttempts == 2`, Java `numAttempts() == 3`).
- **Non-DynamoDB upstreams are rejected**: `x-uri: https://example.com/` gets a 400 `InvalidUpstreamException` with no retries.

Issues found during the run:
- **Rust 1.100 Functions don't load on the dev cell.** Rust 1.100's wasip2 std imports `wasi:cli/exit@0.2.12#exit-with-code`, which mr2 only links since momentohq/mr2rs#3703 (2026-11-12). The dev cell runs `2bf33515` (2026-10-27), so *every* Function built with 1.100 is rejected at upload, existing examples included. `deploy.sh` builds with `FUNCTIONS_TOOLCHAIN` (set to 1.89 in `env.sh`). Prod cells need checking before customers build with 1.100.
- Federated trust principal and colon external ID (see "Setting up the federated role").
- Hop-by-hop `Connection` header breaking federated signatures (fixed in the Function).
- Java `uri()` path doubling (fixed in the client hooks).

## Running it

```bash
cd examples/dynamodb-cache-e2e
cp env.example.sh env.sh   # fill in MOMENTO_* and, for older cells, FUNCTIONS_TOOLCHAIN
source env.sh
./setup.sh                 # table + federated role
./deploy.sh                # build + deploy both Functions
(cd python && ../.venv/bin/python -m pytest -v)
(cd java && mvn test)
./teardown.sh
```
