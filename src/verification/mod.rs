//! Runtime signature verification, separate from producer contract consistency.
pub mod apple;
pub mod bundle;
pub mod context;
pub mod cyclonedx;
pub mod graph;
pub mod historical;
mod io;
pub mod publication;
pub mod release;
pub mod runtime;
pub mod sigstore;

/// Fuzz-only hook: `io` is private to this module, so its strict reader is exposed here.
#[cfg(feature = "fuzzing")]
pub(crate) fn fuzz_strict_json(bytes: &[u8]) -> bool {
    io::parse::<serde_json::Value>(bytes).is_ok()
}
