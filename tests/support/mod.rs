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
