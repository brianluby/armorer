//! Shared probes for environment-dependent test oracles.

/// The independent reconstruction oracle requires GNU patch; BSD patch handles
/// CRLF and no-newline hunks differently, so skip visibly instead of failing.
pub fn gnu_patch_available() -> bool {
    match std::process::Command::new("patch")
        .arg("--version")
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).contains("GNU"),
        Err(_) => false,
    }
}

/// Lowercase SHA-256 hex of `bytes`, independent of the sha2 digest type's formatting traits.
pub fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let digest = Sha256::digest(bytes.as_ref());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}
