//! Conservative three-way decisions; customized bytes never disappear silently.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Decision {
    Create,
    Unchanged,
    Update,
    PreserveCustomization,
    Import,
    Conflict,
}

/// Absence is distinct from empty text. An edited/deleted base conflicts when the
/// generated candidate also changed; no line heuristic may erase a bespoke job.
pub fn decide(base: Option<&str>, current: Option<&str>, candidate: &str) -> Decision {
    match (base, current) {
        (Some(base), Some(current)) if current == base => {
            if current == candidate {
                Decision::Unchanged
            } else {
                Decision::Update
            }
        }
        (Some(base), Some(_)) if candidate == base => Decision::PreserveCustomization,
        (Some(_), _) => Decision::Conflict,
        (None, None) => Decision::Create,
        (None, Some(current)) if current == candidate => Decision::Unchanged,
        _ => Decision::Conflict,
    }
}

/// Preserve extra jobs only after the exact compiled unprivileged prefix. New
/// top-level keys, duplicate protected jobs, YAML directives and outdented content
/// conflict. This is a limited import shape, not a general YAML interpreter.
pub(super) fn caller_extension(current: &str, candidate: &str, protected: &str) -> bool {
    let Some(tail) = current.strip_prefix(candidate) else {
        return false;
    };
    let mut names = std::collections::BTreeSet::new();
    let mut job = false;
    for line in tail.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indentation = line.bytes().take_while(|b| *b == b' ').count();
        if indentation == 2 {
            let Some(name) = line[2..].strip_suffix(':') else {
                return false;
            };
            if !crate::config::identifier(name) || name == protected || !names.insert(name) {
                return false;
            }
            job = true;
        } else if indentation < 4 || !job || line[indentation..].starts_with('\t') {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_three_images_participate_without_adopting_edits_to_next_candidate() {
        for (base, current, next, expected) in [
            (None, None, "next", Decision::Create),
            (None, Some(""), "next", Decision::Conflict),
            (None, Some("next"), "next", Decision::Unchanged),
            (Some("base"), Some("base"), "next", Decision::Update),
            (Some("base"), Some("base"), "base", Decision::Unchanged),
            (
                Some("base"),
                Some("custom λ\r\n"),
                "base",
                Decision::PreserveCustomization,
            ),
            (Some("base"), Some("custom"), "next", Decision::Conflict),
            (Some("base"), None, "base", Decision::Conflict),
            (Some("base"), Some("next"), "next", Decision::Conflict),
        ] {
            assert_eq!(decide(base, current, next), expected);
        }
    }
    #[test]
    fn caller_import_preserves_jobs_but_never_replaces_compiled_role_or_globals() {
        let candidate = "jobs:\n  verify:\n    uses: owner/repo/path@fullsha\n";
        assert!(caller_extension(
            &(candidate.to_owned()
                + "  custom:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo local\n"),
            candidate,
            "verify"
        ));
        for tail in [
            "permissions:\n  contents: write\n",
            "  verify:\n    uses: moving\n",
            "  custom: &anchor\n    runs-on: ubuntu-latest\n",
            "    runs-on: ubuntu-latest\n",
            "---\n",
            "  custom:\n\trun: injected\n",
            "  custom:\n    x: y\n  custom:\n    x: z\n",
        ] {
            assert!(
                !caller_extension(&(candidate.to_owned() + tail), candidate, "verify"),
                "{tail}"
            );
        }
    }
}
