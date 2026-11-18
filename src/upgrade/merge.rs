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
    // An owned optional file the configuration has switched off (ADR 0015). Ownership
    // is dropped and the file is kept exactly as it is: nothing is written or removed.
    // Never produced by `decide`; planning assigns it from ownership alone. A plain
    // comment keeps the schema a single string enum; `Change` documents the shape.
    Release,
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
    use proptest::prelude::*;

    /// Small alphabet so equal values (base == current == candidate) occur often,
    /// plus arbitrary text so unrelated values still occur.
    fn text() -> impl Strategy<Value = String> {
        prop_oneof![
            4 => prop::sample::select(vec!["", "a", "b", "c\r\n", "λ", "a\n"]).prop_map(str::to_owned),
            1 => any::<String>(),
        ]
    }

    proptest! {
        /// Every decision implies the relation between the three images that the
        /// documented table gives it.
        #[test]
        fn decisions_match_the_three_way_contract(
            base in prop::option::of(text()),
            current in prop::option::of(text()),
            candidate in text(),
        ) {
            let decision = decide(base.as_deref(), current.as_deref(), &candidate);
            match decision {
                Decision::Update => {
                    prop_assert!(base.is_some());
                    prop_assert_eq!(&current, &base);
                    prop_assert_ne!(current.as_deref(), Some(candidate.as_str()));
                }
                Decision::Unchanged => {
                    prop_assert_eq!(current.as_deref(), Some(candidate.as_str()));
                    prop_assert!(base.is_none() || base == current);
                }
                Decision::Create => {
                    prop_assert!(base.is_none());
                    prop_assert!(current.is_none());
                }
                Decision::PreserveCustomization => {
                    prop_assert_eq!(base.as_deref(), Some(candidate.as_str()));
                    prop_assert!(current.is_some());
                    prop_assert_ne!(&current, &base);
                }
                // `decide` has no import path; imports come from `caller_extension`.
                Decision::Import => prop_assert!(false, "decide never returns Import"),
                // A release comes from ownership and configuration, never from bytes.
                Decision::Release => prop_assert!(false, "decide never returns Release"),
                Decision::Conflict => {
                    // Conflict is the residue: none of the specific cases applies.
                    let same_as_base = base.is_some() && base == current;
                    let preserve = base.is_some()
                        && current.is_some()
                        && current != base
                        && base.as_deref() == Some(candidate.as_str());
                    let create = base.is_none() && current.is_none();
                    let adopt = base.is_none() && current.as_deref() == Some(candidate.as_str());
                    prop_assert!(!(same_as_base || preserve || create || adopt));
                }
            }
        }

        /// Safety: bytes that differ from the recorded base are never replaced or
        /// recreated, whatever the candidate is. Absence differs from empty text.
        #[test]
        fn customized_bytes_are_never_overwritten(
            base in prop::option::of(text()),
            current in text(),
            candidate in text(),
        ) {
            prop_assume!(base.as_deref() != Some(current.as_str()));
            let decision = decide(base.as_deref(), Some(&current), &candidate);
            prop_assert_ne!(decision, Decision::Update);
            prop_assert_ne!(decision, Decision::Create);
        }

        /// A deleted file that had a base is a conflict, never a silent recreation.
        #[test]
        fn deletion_of_a_based_file_is_a_conflict(base in text(), candidate in text()) {
            prop_assert_eq!(decide(Some(&base), None, &candidate), Decision::Conflict);
        }
    }

    /// Verify that all three images participate without adopting edits to next candidate.
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
    /// Verify that caller import preserves jobs but never replaces compiled role or globals.
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
