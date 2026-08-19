//! Best-effort secret scrubbing for log/error strings on the local control
//! plane.
//!
//! Capto is local-first, but its logs can still carry credential material:
//! the loopback bearer token, or URL query params that embed tokens/keys in
//! error text from FFmpeg or HTTP. `redact` neutralizes the well-known
//! patterns so tracing output never leaks secrets (log_scrubbing).

/// HTTP header used to propagate a per-request ID between the `capto` CLI and
/// the desktop control plane (distributed_tracing).
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Query parameter names whose values are treated as secrets when found in a
/// `name=value` segment of a URL-like string.
const SECRET_QUERY_KEYS: &[&str] = &[
    "token",
    "access_token",
    "refresh_token",
    "api_key",
    "apikey",
    "key",
    "secret",
    "signature",
    "password",
    "sig",
    "auth",
    "session",
];

/// Remove the obvious credential material from `input`:
///
/// - `Bearer <token>` → `Bearer ***`
/// - `?token=abc&x=1` / `&secret=s` → value replaced by `***`
///
/// Non-secret text is preserved byte-for-byte so error messages stay
/// readable. This is intentionally conservative: it only masks the patterns
/// listed above and never attempts full PII redaction.
pub fn redact(input: &str) -> String {
    let out = mask_bearer(input);
    mask_query_values(&out)
}

/// Replace `Bearer <value>` with `Bearer ***`, consuming the value up to the
/// next whitespace so the token text never reaches the log.
fn mask_bearer(s: &str) -> String {
    const NEEDLE: &str = "Bearer ";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        match rest.find(NEEDLE) {
            Some(pos) => {
                let value_after = pos + NEEDLE.len();
                out.push_str(&rest[..value_after]);
                match rest[value_after..].find(char::is_whitespace) {
                    Some(end) => {
                        out.push_str("***");
                        // Resume at the whitespace so the loop's None branch
                        // emits the tail exactly once.
                        rest = &rest[value_after + end..];
                    }
                    None => {
                        out.push_str("***");
                        break;
                    }
                }
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    out
}

fn mask_query_values(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        // Earliest `name=` occurrence among the secret keys.
        let mut best: Option<(usize, &'static str)> = None;
        for key in SECRET_QUERY_KEYS {
            if let Some(pos) = find_key_value(rest, key) {
                if best.is_none_or(|(bp, _)| pos < bp) {
                    best = Some((pos, key));
                }
            }
        }
        let Some((pos, key)) = best else {
            out.push_str(rest);
            break;
        };
        // Copy everything up to the end of `key=`, then mask the value up to
        // the next `&`/`;` separator (or end-of-string) and resume from the
        // separator so the trailing segment is emitted exactly once.
        let value_start = pos + key.len() + 1;
        out.push_str(&rest[..value_start]);
        out.push_str("***");
        match rest[value_start..].find(['&', ';']) {
            Some(sep) => rest = &rest[value_start + sep..],
            None => break,
        }
    }
    out
}

/// Return the byte offset in `s` where `key=` begins, or `None`. The key must
/// be followed by `=` and preceded by the string start or a query delimiter
/// (`?`, `&`, `;`), so plain words like "secretly" are not touched.
fn find_key_value(s: &str, key: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(rel) = s[search_from..].find(key) {
        let pos = search_from + rel;
        let before_ok = pos == 0 || matches!(s.as_bytes().get(pos - 1), Some(b'?' | b'&' | b';'));
        let after_ok = s.as_bytes().get(pos + key.len()) == Some(&b'=');
        if before_ok && after_ok {
            return Some(pos);
        }
        search_from = pos + key.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_bearer_tokens() {
        assert_eq!(
            redact("Authorization: Bearer deadbeef1234 failed"),
            "Authorization: Bearer *** failed"
        );
        assert_eq!(redact("Bearer x"), "Bearer ***");
    }

    #[test]
    fn masks_query_secrets() {
        assert_eq!(
            redact("GET /v1/status?token=abc123&x=1"),
            "GET /v1/status?token=***&x=1"
        );
        assert_eq!(
            redact("url=https://h/api?secret=s3cr3t"),
            "url=https://h/api?secret=***"
        );
        assert_eq!(redact("a=1&password=hunter2;b=2"), "a=1&password=***;b=2");
    }

    #[test]
    fn leaves_benign_text_alone() {
        // "secretly" must not be matched (not a `key=` segment).
        assert_eq!(redact("secretly passwordless"), "secretly passwordless");
        // Unknown keys keep their values.
        assert_eq!(redact("?page=2&sort=asc"), "?page=2&sort=asc");
        // Empty string is fine.
        assert_eq!(redact(""), "");
    }
}
