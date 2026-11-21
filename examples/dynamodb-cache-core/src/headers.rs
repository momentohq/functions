use std::collections::HashMap;

use crate::BadRequest;

/// Look up a header case-insensitively. A missing header is the caller's
/// mistake, so it's a (non-retryable) 400 rather than a 500.
pub fn require_header(
    header: &str,
    headers: &HashMap<String, String>,
) -> Result<String, BadRequest> {
    headers
        .iter()
        .find_map(|(name, value)| name.eq_ignore_ascii_case(header).then(|| value.to_string()))
        .ok_or_else(|| {
            BadRequest::new("MissingHeaderException", format!("Missing {header} header"))
        })
}

/// Hop-by-hop headers (RFC 7230 §6.1) describe the caller's connection to this
/// Function, not the request. The HTTP client drops them after the federated
/// signer has signed them, so forwarding one (e.g. Java's `Connection:
/// Keep-Alive`) makes DynamoDB reject the signature.
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// The caller's Momento API key. It authenticates the call to this Function
/// and must never reach AWS.
const MOMENTO_AUTHORIZATION: &str = "x-momento-authorization";

fn is_hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP.iter().any(|h| name.eq_ignore_ascii_case(h))
}

/// The headers that may be forwarded to DynamoDB: everything except hop-by-hop
/// headers and the Momento API key. Each Function drops its own auth headers
/// too; this is the last filter before the request leaves.
pub fn forwardable(headers: &[(String, String)]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(name, _)| {
            !is_hop_by_hop(name) && !name.eq_ignore_ascii_case(MOMENTO_AUTHORIZATION)
        })
        .cloned()
        .collect()
}

/// Copy `headers`, dropping every header named in `excluded` (case-insensitive).
pub fn without_headers(
    headers: &HashMap<String, String>,
    excluded: &[&str],
) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(name, _)| !excluded.iter().any(|e| name.eq_ignore_ascii_case(e)))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(headers: &[(&str, &str)]) -> Vec<String> {
        let headers: Vec<(String, String)> = headers
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect();
        forwardable(&headers).into_iter().map(|(n, _)| n).collect()
    }

    #[test]
    fn momento_key_and_hop_by_hop_never_forwarded() {
        let forwarded = names(&[
            ("X-Momento-Authorization", "secret"),
            ("x-momento-authorization", "secret"),
            ("Connection", "Keep-Alive"),
            ("transfer-encoding", "chunked"),
            ("X-Amz-Target", "DynamoDB_20120810.GetItem"),
            ("content-length", "2"),
            ("Authorization", "AWS4-HMAC-SHA256 ..."),
        ]);
        assert_eq!(
            forwarded,
            vec!["X-Amz-Target", "content-length", "Authorization"]
        );
    }
}
