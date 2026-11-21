/// Whether `url` is a public DynamoDB endpoint root: the regional
/// `https://dynamodb.<region>.amazonaws.com/` or the account-based
/// `https://<account id>.ddb.<region>.amazonaws.com/` that newer SDKs use.
///
/// The proxy Function forwards to a caller-supplied URL and caches what comes
/// back, so anything else would let a caller cache arbitrary responses.
pub fn is_dynamodb_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    if !path.is_empty() {
        return false;
    }
    let labels: Vec<&str> = host.split('.').collect();
    match labels.as_slice() {
        ["dynamodb", region, "amazonaws", "com"] => is_region(region),
        [account, "ddb", region, "amazonaws", "com"] => {
            account.len() == 12 && account.bytes().all(|b| b.is_ascii_digit()) && is_region(region)
        }
        _ => false,
    }
}

fn is_region(label: &str) -> bool {
    label.contains('-')
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_regional_and_account_endpoints() {
        assert!(is_dynamodb_url("https://dynamodb.us-west-2.amazonaws.com/"));
        assert!(is_dynamodb_url("https://dynamodb.us-west-2.amazonaws.com"));
        assert!(is_dynamodb_url(
            "https://123456789012.ddb.us-west-2.amazonaws.com/"
        ));
    }

    #[test]
    fn rejects_everything_else() {
        for url in [
            "http://dynamodb.us-west-2.amazonaws.com/",
            "https://evil.example.com/",
            "https://dynamodb.us-west-2.amazonaws.com.evil.com/",
            "https://dynamodb.us-west-2.amazonaws.com:8443/",
            "https://user@dynamodb.us-west-2.amazonaws.com/",
            "https://dynamodb.us-west-2.amazonaws.com/other",
            "https://dynamodb.US-WEST-2.amazonaws.com/",
            "https://12345.ddb.us-west-2.amazonaws.com/",
            "https://s3.us-west-2.amazonaws.com/",
        ] {
            assert!(!is_dynamodb_url(url), "{url}");
        }
    }
}
