package momento.ddbcache;

import java.net.URI;
import software.amazon.awssdk.http.ExecutableHttpRequest;
import software.amazon.awssdk.http.HttpExecuteRequest;
import software.amazon.awssdk.http.SdkHttpClient;
import software.amazon.awssdk.http.SdkHttpRequest;

/**
 * Reroutes already-signed DynamoDB requests to the proxy Function.
 *
 * <p>SDK v2 has no interceptor hook that runs after signing and may still modify the
 * request ({@code modifyHttpRequest} runs before signing), so the rewrite happens in the
 * HTTP client, which only ever sees the signed request.
 */
final class ProxyHttpClient implements SdkHttpClient {
    private final SdkHttpClient delegate;
    private final URI functionUri;
    private final String momentoApiKey;
    private final URI upstreamOverride;

    ProxyHttpClient(SdkHttpClient delegate, URI functionUri, String momentoApiKey) {
        this(delegate, functionUri, momentoApiKey, null);
    }

    /** {@code upstreamOverride} replaces x-uri; tests use it to force an upstream failure. */
    ProxyHttpClient(
            SdkHttpClient delegate, URI functionUri, String momentoApiKey, URI upstreamOverride) {
        this.delegate = delegate;
        this.functionUri = functionUri;
        this.momentoApiKey = momentoApiKey;
        this.upstreamOverride = upstreamOverride;
    }

    @Override
    public ExecutableHttpRequest prepareRequest(HttpExecuteRequest request) {
        SdkHttpRequest signed = request.httpRequest();
        URI upstream = upstreamOverride != null ? upstreamOverride : signed.getUri();
        SdkHttpRequest rerouted =
                FunctionUri.retarget(signed.toBuilder(), functionUri)
                        // The signed Host names DynamoDB; the Function's host must route the call.
                        .removeHeader("Host")
                        .putHeader("x-uri", upstream.toString())
                        .putHeader("x-momento-authorization", momentoApiKey)
                        .build();
        return delegate.prepareRequest(
                HttpExecuteRequest.builder()
                        .request(rerouted)
                        .contentStreamProvider(request.contentStreamProvider().orElse(null))
                        .metricCollector(request.metricCollector().orElse(null))
                        .build());
    }

    @Override
    public String clientName() {
        return "MomentoProxy" + delegate.clientName();
    }

    @Override
    public void close() {
        delegate.close();
    }
}
