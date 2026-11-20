package momento.ddbcache;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.net.URI;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;
import software.amazon.awssdk.core.exception.SdkException;
import software.amazon.awssdk.services.dynamodb.DynamoDbClient;
import software.amazon.awssdk.services.dynamodb.model.AttributeValue;
import software.amazon.awssdk.services.dynamodb.model.ConditionalCheckFailedException;
import software.amazon.awssdk.services.dynamodb.model.DeleteRequest;
import software.amazon.awssdk.services.dynamodb.model.DynamoDbException;
import software.amazon.awssdk.services.dynamodb.model.GetItemResponse;
import software.amazon.awssdk.services.dynamodb.model.Put;
import software.amazon.awssdk.services.dynamodb.model.PutRequest;
import software.amazon.awssdk.services.dynamodb.model.ReturnConsumedCapacity;
import software.amazon.awssdk.services.dynamodb.model.ReturnValuesOnConditionCheckFailure;
import software.amazon.awssdk.services.dynamodb.model.TransactWriteItem;
import software.amazon.awssdk.services.dynamodb.model.Update;
import software.amazon.awssdk.services.dynamodb.model.WriteRequest;

/** End-to-end checks for the DynamoDB caching Functions; mirrors python/test_caching.py. */
class CachingTest {
    private static final String TABLE = System.getenv("TABLE_NAME");
    private final DynamoDbClient ddb = Clients.direct();

    private static Map<String, AttributeValue> newKey() {
        return Map.of("pk", AttributeValue.fromS("e2e-" + UUID.randomUUID()));
    }

    private static Map<String, AttributeValue> item(Map<String, AttributeValue> key, String v) {
        return Map.of("pk", key.get("pk"), "v", AttributeValue.fromS(v));
    }

    private static String value(GetItemResponse response) {
        return response.hasItem() ? response.item().get("v").s() : null;
    }

    private static String get(DynamoDbClient client, Map<String, AttributeValue> key) {
        return value(client.getItem(r -> r.tableName(TABLE).key(key)));
    }

    private void putDirect(Map<String, AttributeValue> key, String v) {
        ddb.putItem(r -> r.tableName(TABLE).item(item(key, v)));
    }

    @ParameterizedTest
    @ValueSource(strings = {"proxy", "federated"})
    void readMissThenHit(String mode) {
        DynamoDbClient fn = Clients.function(mode);
        var key = newKey();
        putDirect(key, "v1");
        assertEquals("v1", get(fn, key)); // miss, fills cache

        // Change the item behind the Function's back: only a cache hit still sees v1.
        putDirect(key, "v2");
        assertEquals("v1", get(fn, key));

        // Strongly consistent and projected reads bypass the cache.
        assertEquals(
                "v2", value(fn.getItem(r -> r.tableName(TABLE).key(key).consistentRead(true))));
        var projected = fn.getItem(r -> r.tableName(TABLE).key(key).projectionExpression("v"));
        assertEquals(Map.of("v", AttributeValue.fromS("v2")), projected.item());
        // A cached response has no ConsumedCapacity, so this must read through too.
        var measured =
                fn.getItem(
                        r -> r.tableName(TABLE)
                                .key(key)
                                .returnConsumedCapacity(ReturnConsumedCapacity.TOTAL));
        assertEquals("v2", value(measured));
        assertEquals(TABLE, measured.consumedCapacity().tableName());
    }

    @ParameterizedTest
    @ValueSource(strings = {"proxy", "federated"})
    void putItemInvalidates(String mode) {
        DynamoDbClient fn = Clients.function(mode);
        var key = newKey();
        putDirect(key, "v1");
        get(fn, key);
        fn.putItem(r -> r.tableName(TABLE).item(item(key, "v2")));
        assertEquals("v2", get(fn, key));
    }

    @ParameterizedTest
    @ValueSource(strings = {"proxy", "federated"})
    void updateItemInvalidates(String mode) {
        DynamoDbClient fn = Clients.function(mode);
        var key = newKey();
        putDirect(key, "v1");
        get(fn, key);
        fn.updateItem(
                r -> r.tableName(TABLE)
                        .key(key)
                        .updateExpression("SET v = :v")
                        .expressionAttributeValues(Map.of(":v", AttributeValue.fromS("v2"))));
        assertEquals("v2", get(fn, key));
    }

    @ParameterizedTest
    @ValueSource(strings = {"proxy", "federated"})
    void deleteItemInvalidates(String mode) {
        DynamoDbClient fn = Clients.function(mode);
        var key = newKey();
        putDirect(key, "v1");
        get(fn, key);
        fn.deleteItem(r -> r.tableName(TABLE).key(key));
        assertNull(get(fn, key));
    }

    @ParameterizedTest
    @ValueSource(strings = {"proxy", "federated"})
    void batchWriteItemInvalidates(String mode) {
        DynamoDbClient fn = Clients.function(mode);
        var key = newKey();
        var other = Map.of("pk", AttributeValue.fromS(key.get("pk").s() + "-other"));
        putDirect(key, "v1");
        putDirect(other, "v1");
        get(fn, key);
        get(fn, other);

        fn.batchWriteItem(
                r -> r.requestItems(
                        Map.of(
                                TABLE,
                                List.of(
                                        WriteRequest.builder()
                                                .putRequest(
                                                        PutRequest.builder()
                                                                .item(item(key, "v2"))
                                                                .build())
                                                .build(),
                                        WriteRequest.builder()
                                                .deleteRequest(
                                                        DeleteRequest.builder().key(other).build())
                                                .build()))));
        assertEquals("v2", get(fn, key));
        assertNull(get(fn, other));
    }

    @ParameterizedTest
    @ValueSource(strings = {"proxy", "federated"})
    void missingItemThenPutCreatesIt(String mode) {
        DynamoDbClient fn = Clients.function(mode);
        var key = newKey();
        // The empty GetItem response is a 2xx and gets cached...
        assertNull(get(fn, key));
        // ...so the PutItem must find it via KEY_ATTRIBUTES and delete it.
        fn.putItem(r -> r.tableName(TABLE).item(item(key, "created")));
        assertEquals("created", get(fn, key));
    }

    @ParameterizedTest
    @ValueSource(strings = {"proxy", "federated"})
    void errorsReachSdkIntact(String mode) {
        DynamoDbClient fn = Clients.function(mode);
        var key = newKey();
        // The federated role may only touch the e2e table, so IAM denies the call
        // before DynamoDB can report the table missing.
        var missing =
                assertThrows(
                        DynamoDbException.class,
                        () -> fn.getItem(
                                r -> r.tableName("ddb-cache-e2e-does-not-exist").key(key)));
        assertEquals(
                "federated".equals(mode) ? "AccessDeniedException" : "ResourceNotFoundException",
                missing.awsErrorDetails().errorCode());

        var invalid =
                assertThrows(
                        DynamoDbException.class,
                        () -> fn.getItem(
                                r -> r.tableName(TABLE)
                                        .key(Map.of("wrong", AttributeValue.fromS("x")))));
        assertEquals("ValidationException", invalid.awsErrorDetails().errorCode());

        putDirect(key, "v1");
        var e =
                assertThrows(
                        ConditionalCheckFailedException.class,
                        () -> fn.putItem(
                                r -> r.tableName(TABLE)
                                        .item(item(key, "v2"))
                                        .conditionExpression("attribute_not_exists(pk)")
                                        .returnValuesOnConditionCheckFailure(
                                                ReturnValuesOnConditionCheckFailure.ALL_OLD)));
        assertEquals("v1", e.item().get("v").s());
    }

    /**
     * Passes the Function's DynamoDB-endpoint check but doesn't resolve, so the forward fails
     * with a 5xx. Only the proxy Function takes its upstream from the client (x-uri), so tests
     * that need a failing upstream are proxy-only.
     */
    private static final URI NOWHERE_DYNAMODB =
            URI.create("https://dynamodb.xx-nowhere-1.amazonaws.com/");

    /** A failed GetItem must not be cached: it succeeds once the upstream works again. */
    @Test
    void errorsAreNotCached() {
        var key = newKey();
        putDirect(key, "v1");
        var e =
                assertThrows(
                        SdkException.class,
                        () -> get(Clients.proxyWithUpstream(NOWHERE_DYNAMODB, 1), key));
        assertEquals(1, e.numAttempts());
        assertEquals("v1", get(Clients.function("proxy"), key));
    }

    @ParameterizedTest
    @ValueSource(strings = {"proxy", "federated"})
    void transactWriteItemsInvalidates(String mode) {
        DynamoDbClient fn = Clients.function(mode);
        var key = newKey();
        var other = Map.of("pk", AttributeValue.fromS(key.get("pk").s() + "-other"));
        putDirect(key, "v1");
        putDirect(other, "v1");
        get(fn, key);
        get(fn, other);

        fn.transactWriteItems(
                r -> r.transactItems(
                        TransactWriteItem.builder()
                                .put(Put.builder().tableName(TABLE).item(item(key, "v2")).build())
                                .build(),
                        TransactWriteItem.builder()
                                .update(
                                        Update.builder()
                                                .tableName(TABLE)
                                                .key(other)
                                                .updateExpression("SET v = :v")
                                                .expressionAttributeValues(
                                                        Map.of(":v", AttributeValue.fromS("v2")))
                                                .build())
                                .build()));
        assertEquals("v2", get(fn, key));
        assertEquals("v2", get(fn, other));
    }

    /** A 5xx from the Function is retried by the SDK like any DynamoDB 5xx. */
    @Test
    void retriesReachSdk() {
        DynamoDbClient fn = Clients.proxyWithUpstream(NOWHERE_DYNAMODB, 3);
        var e = assertThrows(SdkException.class, fn::listTables);
        assertEquals(3, e.numAttempts());
    }

    /** Responses get cached, so the proxy only forwards to DynamoDB; anything else is a 400. */
    @Test
    void proxyRejectsNonDynamoDbUpstream() {
        DynamoDbClient fn = Clients.proxyWithUpstream(URI.create("https://example.com/"), 3);
        var e = assertThrows(DynamoDbException.class, () -> get(fn, newKey()));
        assertEquals(400, e.statusCode());
        assertEquals("InvalidUpstreamException", e.awsErrorDetails().errorCode());
        assertEquals(1, e.numAttempts());
    }

    /** Momento rejects a bad key before the Function runs, so it never reads the cache. */
    @ParameterizedTest
    @ValueSource(strings = {"proxy", "federated"})
    void momentoAuthRequired(String mode) {
        DynamoDbClient fn = Clients.function(mode, "not-a-valid-momento-key");
        var e =
                assertThrows(
                        DynamoDbException.class,
                        () -> fn.getItem(r -> r.tableName(TABLE).key(newKey())));
        assertTrue(e.statusCode() >= 400 && e.statusCode() < 500, "status " + e.statusCode());
    }
}
