//! Exact, bounded review views derived from unchanged version-one apply plans.

use crate::{
    Error, Result, digest,
    plan::{Plan, inspect},
    read_small, safe_path,
};
use serde::Serialize;
use std::path::Path;

const LIMIT: usize = 1_048_576;

/// A review view, never an apply authorization or release-readiness receipt.
/// The embedded `plan` retains its existing schema and digest semantics.
#[derive(Debug, Serialize)]
pub struct Preview {
    pub schema_version: u32,
    pub plan: Plan,
    pub files: Vec<FilePreview>,
    pub preserved_inputs: Vec<PreservedInput>,
}

#[derive(Debug, Serialize)]
pub struct FilePreview {
    pub path: String,
    pub disposition: String,
    pub before_content: Option<String>,
    pub proposed_content: String,
    /// A complete replacement hunk; no quadratic matching or omitted lines.
    pub unified_diff: String,
}

/// Exact bytes of a blocking legacy input, preserved rather than scheduled for deletion.
#[derive(Debug, Serialize)]
pub struct PreservedInput {
    pub path: String,
    pub content: String,
    pub sha256: String,
    pub reason: &'static str,
}

/// Read only the exact digest-bound preimage, retaining absence separately from empty text.
fn optional_text(root: &Path, relative: &str, expected: Option<&str>) -> Result<Option<String>> {
    let path = safe_path(root, relative)?;
    let bytes = match std::fs::symlink_metadata(&path) {
        Ok(_) => Some(read_small(&path)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    if bytes.as_deref().map(digest).as_deref() != expected {
        return Err(Error::Transaction(
            "preview input changed; generate a fresh plan",
        ));
    }
    bytes
        .map(|bytes| {
            String::from_utf8(bytes).map_err(|_| {
                Error::Transaction("preview input must be UTF-8; bytes were preserved")
            })
        })
        .transpose()
}

/// Emit every original line verbatim and annotate missing final newlines for patch readers.
fn append_lines(output: &mut String, prefix: char, content: &str) {
    for line in content.split_inclusive('\n') {
        output.push(prefix);
        output.push_str(line);
        if !line.ends_with('\n') {
            output.push_str("\n\\ No newline at end of file\n");
        }
    }
}

/// Render all old and new lines, including CRLF and missing-final-newline markers.
fn diff(path: &str, before: Option<&str>, after: &str) -> String {
    if before == Some(after) {
        return String::new();
    }
    let old = before.unwrap_or_default();
    let old_lines = old.split_inclusive('\n').count();
    let new_lines = after.split_inclusive('\n').count();
    let old_name = before.map_or_else(|| "/dev/null".into(), |_| format!("a/{path}"));
    let mut output = format!(
        "--- {old_name}\n+++ b/{path}\n@@ -{},{old_lines} +{},{new_lines} @@\n",
        usize::from(old_lines != 0),
        usize::from(new_lines != 0),
    );
    append_lines(&mut output, '-', old);
    append_lines(&mut output, '+', after);
    output
}

/// Attach matching display preimages and legacy blockers, then bound the complete JSON view.
fn from_plan(root: &Path, plan: Plan) -> Result<Preview> {
    let mut files = Vec::new();
    for change in &plan.changes {
        let before_content = optional_text(root, &change.path, change.before_sha256.as_deref())?;
        files.push(FilePreview {
            path: change.path.clone(),
            disposition: change.disposition.clone(),
            unified_diff: diff(
                &change.path,
                before_content.as_deref(),
                &change.proposed_content,
            ),
            before_content,
            proposed_content: change.proposed_content.clone(),
        });
    }
    let mut preserved_inputs = Vec::new();
    let expected = plan
        .input_preimages
        .get("rust-toolchain")
        .ok_or(Error::Transaction(
            "preview plan is missing a legacy-toolchain preimage",
        ))?;
    if let Some(content) = optional_text(root, "rust-toolchain", expected.as_deref())? {
        preserved_inputs.push(PreservedInput {
            path: "rust-toolchain".into(),
            sha256: digest(content.as_bytes()),
            content,
            reason: "Legacy toolchain blocks apply; resolve its migration explicitly.",
        });
    }
    let preview = Preview {
        schema_version: 1,
        plan,
        files,
        preserved_inputs,
    };
    // Bound the complete serialized review view, including JSON escaping. Never
    // truncate a conflict or emit a partial view that could conceal customizations.
    if serde_json::to_vec_pretty(&preview)
        .map_err(|_| Error::Json)?
        .len()
        > LIMIT
    {
        return Err(Error::Transaction(
            "preview exceeds 1 MiB; bytes were preserved",
        ));
    }
    Ok(preview)
}

/// Generate an exact read-only view and its approval-bound plan.
///
/// # Errors
/// Propagates inspection/path/read errors. Rejects changed preimages, non-UTF-8
/// input and output exceeding 1 MiB. No file is created or changed in the consumer.
pub fn preview(root: &Path) -> Result<Preview> {
    from_plan(root, inspect(root, "plan")?)
}

/// Review a saved, already digest-validated plan against fresh repository state.
///
/// # Errors
/// Rejects stale plans before rendering, then applies the same exact-byte and
/// size checks as [`preview`]. The caller must use `apply::load_plan` to validate
/// the saved plan's digest and structure before calling this function.
pub fn preview_plan(root: &Path, plan: &Plan) -> Result<Preview> {
    let fresh = inspect(root, "plan")?;
    if plan.digest()? != fresh.digest()? || plan.plan_sha256 != fresh.plan_sha256 {
        return Err(Error::Transaction(
            "saved preview plan is stale; generate a fresh plan",
        ));
    }
    from_plan(root, fresh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Check complete replacement hunks for absent, empty, unchanged and newline-sensitive text.
    fn complete_hunks_preserve_empty_crlf_unicode_and_final_newline() {
        assert_eq!(diff("file", Some("same\n"), "same\n"), "");
        assert_eq!(
            diff("file", None, "new\n"),
            "--- /dev/null\n+++ b/file\n@@ -0,0 +1,1 @@\n+new\n"
        );
        assert_eq!(
            diff("file", Some(""), "new"),
            "--- a/file\n+++ b/file\n@@ -0,0 +1,1 @@\n+new\n\\ No newline at end of file\n"
        );
        assert_eq!(
            diff("file", Some("old\r\nλ"), "new\r\n"),
            "--- a/file\n+++ b/file\n@@ -1,2 +1,1 @@\n-old\r\n-λ\n\\ No newline at end of file\n+new\r\n"
        );
        assert_eq!(
            diff("file", Some("old\n"), ""),
            "--- a/file\n+++ b/file\n@@ -1,1 +0,0 @@\n-old\n"
        );
    }

    #[test]
    /// Reject changed or missing preimages before unreviewed contents can reach the preview.
    fn changed_preimages_are_rejected_before_content_is_rendered() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("file"), "unreviewed").unwrap();
        let error = optional_text(root.path(), "file", Some(&digest(b"reviewed"))).unwrap_err();
        assert!(!error.to_string().contains("unreviewed"));
        assert!(optional_text(root.path(), "file", None).is_err());
        std::fs::remove_file(root.path().join("file")).unwrap();
        assert!(optional_text(root.path(), "file", Some(&digest(b"reviewed"))).is_err());
    }
}
