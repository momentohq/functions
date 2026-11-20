package momento.ddbcache;

import java.net.URI;
import software.amazon.awssdk.http.SdkHttpRequest;

/** Points a request at exactly the Function URL. */
final class FunctionUri {
    private FunctionUri() {}

    /**
     * {@code SdkHttpRequest.Builder.uri(URI)} appends the URI's path to the request's existing
     * path (e.g. {@code /functions/c/f/functions/c/f/}), so set each part explicitly.
     */
    static SdkHttpRequest.Builder retarget(SdkHttpRequest.Builder request, URI functionUri) {
        return request.protocol(functionUri.getScheme())
                .host(functionUri.getHost())
                .port(functionUri.getPort() == -1 ? null : functionUri.getPort())
                .encodedPath(functionUri.getRawPath());
    }
}
