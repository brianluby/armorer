//! Byte-level entry points for the cargo-fuzz targets under `fuzz/`.
//!
//! Compiled only with `--features fuzzing`. Each wrapper feeds untrusted bytes to
//! one internal parser and reports whether it accepted them. Rejection is the
//! normal outcome; a panic inside the parser is the finding a fuzzer looks for.

/// Fixed trusted day for policy parsing: 2026-11-17 in days since the Unix epoch.
const POLICY_TODAY: u64 = 20_774;

/// Stdout cap the production GitHub CLI call applies before parsing a response.
const HTTP_RESPONSE_LIMIT: usize = 4 * 1024 * 1024;

/// The strict, duplicate-key-rejecting verification JSON reader.
pub fn verification_json(bytes: &[u8]) -> bool {
    crate::verification::fuzz_strict_json(bytes)
}

/// The native CLI HTTP response parser, with both exit-status flags.
pub fn http_response(bytes: &[u8]) -> bool {
    // The production capture rejects output over 4 MiB before parsing.
    if bytes.len() > HTTP_RESPONSE_LIMIT {
        return false;
    }
    let success = crate::verification::publication::fuzz_response(true, bytes);
    let failure = crate::verification::publication::fuzz_response(false, bytes);
    success || failure
}

/// The size-bounded duplicate-free contract reader.
pub fn trust_json(bytes: &[u8]) -> bool {
    crate::trust::parse_json::<serde_json::Value>(bytes).is_ok()
}

/// The CI policy parser with a fixed trusted day.
pub fn ci_policy(bytes: &[u8]) -> bool {
    crate::ci_policy::parse_at(bytes, POLICY_TODAY).is_ok()
}

/// `armorer.toml` deserialization followed by shape validation.
pub fn config_toml(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    match toml::from_str::<crate::config::Config>(text) {
        Ok(config) => config.validate_shape().is_ok(),
        Err(_) => false,
    }
}

/// `armorer.lock` deserialization.
pub fn lock_toml(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    toml::from_str::<crate::config::Lock>(text).is_ok()
}

/// The unverified Sigstore bundle transport reader.
pub fn sigstore_bundle(bytes: &[u8]) -> bool {
    crate::verification::bundle::fuzz_bare(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A response the parser accepts is still rejected once it exceeds the production cap.
    #[test]
    fn http_response_applies_the_production_size_cap() {
        let mut bytes = b"HTTP/2.0 200 OK\r\nContent-Type: application/json\r\n\r\n".to_vec();
        bytes.resize(HTTP_RESPONSE_LIMIT, b' ');
        assert!(http_response(&bytes));
        bytes.push(b' ');
        assert!(!http_response(&bytes));
    }
}
