package momento.ddbcache;

import java.net.URI;
import software.amazon.awssdk.auth.credentials.AwsBasicCredentials;
import software.amazon.awssdk.auth.credentials.StaticCredentialsProvider;
import software.amazon.awssdk.awscore.retry.AwsRetryStrategy;
import software.amazon.awssdk.http.apache.ApacheHttpClient;
import software.amazon.awssdk.services.dynamodb.DynamoDbClient;

/** DynamoDB clients for each way of reaching the caching Functions. */
final class Clients {
    static final String MOMENTO_API_KEY = env("MOMENTO_API_KEY");
    static final URI PROXY_URI = URI.create(env("PROXY_FUNCTION_URL"));
    static final URI FEDERATED_URI = URI.create(env("FEDERATED_FUNCTION_URL"));

    private Clients() {}

    /** Talks to DynamoDB directly, bypassing the Function and its cache. */
    static DynamoDbClient direct() {
        return DynamoDbClient.create();
    }

    static DynamoDbClient function(String mode) {
        return function(mode, MOMENTO_API_KEY);
    }

    static DynamoDbClient function(String mode, String momentoApiKey) {
        return "proxy".equals(mode) ? proxy(momentoApiKey) : federated(momentoApiKey);
    }

    /** Signs with the caller's own AWS credentials; the HTTP client reroutes after signing. */
    static DynamoDbClient proxy(String momentoApiKey) {
        return DynamoDbClient.builder()
                .httpClient(new ProxyHttpClient(ApacheHttpClient.create(), PROXY_URI, momentoApiKey))
                .build();
    }

    /** Proxy client whose Function is told to forward to {@code upstream}; for retry tests. */
    static DynamoDbClient proxyWithUpstream(URI upstream, int maxAttempts) {
        return DynamoDbClient.builder()
                .httpClient(
                        new ProxyHttpClient(
                                ApacheHttpClient.create(), PROXY_URI, MOMENTO_API_KEY, upstream))
                .overrideConfiguration(
                        c ->
                                c.retryStrategy(
                                        AwsRetryStrategy.standardRetryStrategy().toBuilder()
                                                .maxAttempts(maxAttempts)
                                                .build()))
                .build();
    }

    /** Points the endpoint at the Function; the Function signs as its IAM role. */
    static DynamoDbClient federated(String momentoApiKey) {
        return DynamoDbClient.builder()
                .endpointOverride(FEDERATED_URI)
                // The SDK always signs; the Function strips this placeholder signature.
                .credentialsProvider(
                        StaticCredentialsProvider.create(
                                AwsBasicCredentials.create("placeholder", "placeholder")))
                .overrideConfiguration(
                        c -> c.addExecutionInterceptor(
                                new MomentoAuthInterceptor(FEDERATED_URI, momentoApiKey)))
                .build();
    }

    private static String env(String name) {
        String value = System.getenv(name);
        if (value == null || value.isEmpty()) {
            throw new IllegalStateException("Missing env var " + name + "; source env.sh");
        }
        return value;
    }
}
