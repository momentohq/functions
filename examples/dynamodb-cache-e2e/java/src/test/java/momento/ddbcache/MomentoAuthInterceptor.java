package momento.ddbcache;

import java.net.URI;
import software.amazon.awssdk.core.interceptor.Context;
import software.amazon.awssdk.core.interceptor.ExecutionAttributes;
import software.amazon.awssdk.core.interceptor.ExecutionInterceptor;
import software.amazon.awssdk.http.SdkHttpRequest;

/** Adds the Momento API key and pins the exact Function URL for the federated Function. */
final class MomentoAuthInterceptor implements ExecutionInterceptor {
    private final URI functionUri;
    private final String momentoApiKey;

    MomentoAuthInterceptor(URI functionUri, String momentoApiKey) {
        this.functionUri = functionUri;
        this.momentoApiKey = momentoApiKey;
    }

    @Override
    public SdkHttpRequest modifyHttpRequest(
            Context.ModifyHttpRequest context, ExecutionAttributes executionAttributes) {
        // endpointOverride + DynamoDB's "/" request path leaves a trailing slash.
        return FunctionUri.retarget(context.httpRequest().toBuilder(), functionUri)
                .putHeader("x-momento-authorization", momentoApiKey)
                .build();
    }
}
