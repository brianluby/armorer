//! Approval-bound local transactions. Receipts never assert hosted verification.

use crate::{
    Error, Result,
    config::{VERSION, hex_digest},
    digest,
    plan::{Plan, inspect},
    read_small, safe_path,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions, TryLockError},
    io::Write,
    path::Path,
};

const STATE: &str = ".armorer/state.json";
const JOURNAL: &str = ".armorer/journal.json";
const TOOLCHAIN: &str = "rust-toolchain.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct State {
    schema_version: u32,
    runtime_version: String,
    last_plan_sha256: String,
    pub(crate) managed: BTreeMap<String, Base>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Base {
    pub(crate) sha256: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    plan: Plan,
    committed: bool,
    operations: Vec<FileWrite>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileWrite {
    path: String,
    before: Option<String>,
    after: String,
}

#[derive(Debug, Serialize)]
pub struct Receipt {
    pub plan_sha256: String,
    pub outcome: &'static str,
    pub configured: bool,
    pub toolchain_configured: bool,
    pub ci_verified: bool,
    pub release_rehearsed: bool,
    pub published: bool,
    pub provenance_verified: bool,
}

fn receipt(plan: &Plan, outcome: &'static str, toolchain_configured: bool) -> Receipt {
    Receipt {
        plan_sha256: plan.plan_sha256.clone(),
        outcome,
        configured: false,
        toolchain_configured,
        ci_verified: false,
        release_rehearsed: false,
        published: false,
        provenance_verified: false,
    }
}

fn optional_bytes(root: &Path, relative: &str) -> Result<Option<Vec<u8>>> {
    let path = safe_path(root, relative)?;
    match fs::symlink_metadata(&path) {
        Ok(_) => Ok(Some(read_small(&path)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| Error::Json)?;
    if bytes.len() > 1_048_576 {
        return Err(Error::Transaction("transaction exceeds 1 MiB"));
    }
    Ok(bytes)
}

fn text(bytes: &[u8]) -> Result<String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| Error::Transaction("managed input must be UTF-8"))
}

// serde's Value/BTreeMap parsers otherwise accept repeated object keys. Strict
// parsing also checks nested metadata, whose Cargo-facing structs tolerate extras.
struct StrictValue(serde_json::Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StrictValue;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                value: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_i64<E: serde::de::Error>(
                self,
                value: i64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_u64<E: serde::de::Error>(
                self,
                value: u64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_f64<E: serde::de::Error>(
                self,
                value: f64,
            ) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|number| StrictValue(number.into()))
                    .ok_or_else(|| E::custom("invalid JSON number"))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                value: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                value: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(serde_json::Value::Null))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                self.visit_none()
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(StrictValue(value)) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(StrictValue(values.into()))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, StrictValue(value))) =
                    map.next_entry::<String, StrictValue>()?
                {
                    if values.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                }
                Ok(StrictValue(values.into()))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

fn decode<T: serde::de::DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T> {
    let StrictValue(value) = serde_json::from_slice(bytes)
        .map_err(|_| Error::Transaction("invalid transaction JSON"))?;
    let typed: T = serde_json::from_value(value.clone())
        .map_err(|_| Error::Transaction("invalid transaction JSON"))?;
    if serde_json::to_value(&typed).map_err(|_| Error::Json)? != value {
        return Err(Error::Transaction(
            "unknown or missing transaction JSON fields",
        ));
    }
    Ok(typed)
}

pub(crate) fn load_state(root: &Path) -> Result<Option<(State, Vec<u8>)>> {
    let Some(bytes) = optional_bytes(root, STATE)? else {
        return Ok(None);
    };
    let state: State = decode(&bytes)?;
    validate_state(&state)?;
    Ok(Some((state, bytes)))
}

fn validate_state(state: &State) -> Result<()> {
    if state.schema_version != VERSION
        || state.runtime_version != env!("CARGO_PKG_VERSION")
        || !hex_digest(&state.last_plan_sha256, 64)
        || state.managed.len() > 1
    {
        return Err(Error::Transaction("incompatible ownership state"));
    }
    for (path, base) in &state.managed {
        if path != TOOLCHAIN || digest(base.content.as_bytes()) != base.sha256 {
            return Err(Error::Transaction("invalid managed-file base"));
        }
    }
    Ok(())
}

/// Parse a bounded, strict plan and require its independently approved digest.
pub fn load_plan(path: &Path, expected: &str) -> Result<Plan> {
    let plan: Plan = decode(&read_small(path)?)?;
    validate_plan(&plan, expected)?;
    Ok(plan)
}

fn validate_plan(plan: &Plan, expected: &str) -> Result<()> {
    if !hex_digest(expected, 64)
        || plan.plan_sha256 != expected
        || plan.digest()? != expected
        || plan.schema_version != VERSION
        || plan.runtime_version != env!("CARGO_PKG_VERSION")
        || plan.mode != "plan"
        || plan.changes.len() != 1
    {
        return Err(Error::Transaction(
            "plan does not match the approved digest or runtime",
        ));
    }
    let change = &plan.changes[0];
    let expected_content = format!(
        "[toolchain]\nchannel = \"{}\"\nprofile = \"minimal\"\ncomponents = [\"clippy\", \"rustfmt\"]\n",
        plan.intent.toolchain
    );
    if change.path != TOOLCHAIN
        || change.proposed_content != expected_content
        || change.after_sha256 != digest(expected_content.as_bytes())
        || !matches!(
            change.disposition.as_str(),
            "create" | "unchanged" | "update" | "conflict"
        )
    {
        return Err(Error::Transaction(
            "unsupported generated content or target",
        ));
    }
    Ok(())
}

fn lock(root: &Path) -> Result<File> {
    let directory = safe_path(root, ".armorer")?;
    match fs::create_dir(&directory) {
        Ok(()) => sync_directory(root)?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && directory.is_dir() => {}
        Err(error) => return Err(error.into()),
    }
    let path = safe_path(root, ".armorer/apply.lock")?;
    if path.try_exists()? && !fs::symlink_metadata(&path)?.is_file() {
        return Err(Error::Transaction("apply lock must be a regular file"));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => {
            Err(Error::Transaction("another Armorer transaction is active"))
        }
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

fn sync_directory(path: &Path) -> Result<()> {
    // Initial mutation support is Unix, matching the supported host platforms.
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    {
        let _ = path;
        return Err(Error::Transaction(
            "durable transactions require a supported Unix host",
        ));
    }
    Ok(())
}

fn replace(root: &Path, relative: &str, bytes: &[u8], absent: bool) -> Result<()> {
    let destination = safe_path(root, relative)?;
    let parent = destination
        .parent()
        .ok_or(Error::Transaction("invalid destination"))?;
    let mut staging = tempfile::NamedTempFile::new_in(parent)?;
    staging.write_all(bytes)?;
    if !absent {
        staging
            .as_file()
            .set_permissions(fs::metadata(&destination)?.permissions())?;
    }
    staging.as_file().sync_all()?;
    if absent {
        staging
            .persist_noclobber(&destination)
            .map_err(|error| error.error)?;
    } else {
        staging.persist(&destination).map_err(|error| error.error)?;
    }
    sync_directory(parent)
}

fn matches_bytes(current: Option<&[u8]>, expected: Option<&str>) -> bool {
    current == expected.map(str::as_bytes)
}

fn rollback(root: &Path, journal: &Journal) -> Result<()> {
    // Validate every path before restoring any. Never overwrite intervening edits.
    for operation in &journal.operations {
        let current = optional_bytes(root, &operation.path)?;
        if !matches_bytes(current.as_deref(), operation.before.as_deref())
            && !matches_bytes(current.as_deref(), Some(&operation.after))
        {
            return Err(Error::Transaction(
                "recovery conflicts with an intervening edit; journal preserved",
            ));
        }
    }
    for operation in journal.operations.iter().rev() {
        let current = optional_bytes(root, &operation.path)?;
        if matches_bytes(current.as_deref(), operation.before.as_deref()) {
            continue;
        }
        match &operation.before {
            Some(before) => replace(root, &operation.path, before.as_bytes(), current.is_none())?,
            None => {
                let path = safe_path(root, &operation.path)?;
                fs::remove_file(&path)?;
                sync_directory(
                    path.parent()
                        .ok_or(Error::Transaction("invalid recovery parent"))?,
                )?;
            }
        }
    }
    remove_journal(root)
}

fn remove_journal(root: &Path) -> Result<()> {
    fs::remove_file(safe_path(root, JOURNAL)?)?;
    sync_directory(&safe_path(root, ".armorer")?)
}

fn same_inputs(original: &Plan, fresh: &Plan) -> Result<bool> {
    fn normalized(plan: &Plan) -> Result<String> {
        let mut plan = plan.clone();
        plan.state_sha256 = None;
        for change in &mut plan.changes {
            change.disposition.clear();
            change.before_sha256 = None;
        }
        plan.digest()
    }
    Ok(normalized(original)? == normalized(fresh)?)
}

/// Apply only freshly regenerated, digest-approved generated bytes.
/// Identical existing files remain unowned; matching committed-plan replay is a no-op.
pub fn apply(root: &Path, plan: &Plan, expected: &str) -> Result<Receipt> {
    if !cfg!(unix) {
        return Err(Error::Transaction(
            "durable transactions require a supported Unix host",
        ));
    }
    apply_inner(root, plan, expected, |_| Ok(()))
}

fn apply_inner(
    root: &Path,
    plan: &Plan,
    expected: &str,
    mut after_write: impl FnMut(usize) -> Result<()>,
) -> Result<Receipt> {
    validate_plan(plan, expected)?;
    if plan
        .changes
        .iter()
        .any(|change| change.disposition == "conflict")
    {
        return Err(Error::Transaction(
            "unowned customization conflicts with generated content",
        ));
    }
    let root = root.canonicalize()?;
    let fresh = inspect(&root, "plan")?;
    if !same_inputs(plan, &fresh)? {
        return Err(Error::Transaction(
            "approved inputs changed; generate and review a new plan",
        ));
    }
    let _guard = lock(&root)?;
    if optional_bytes(&root, JOURNAL)?.is_some() {
        return Err(Error::Transaction(
            "unfinished transaction requires explicit recovery",
        ));
    }
    let fresh = inspect(&root, "plan")?;
    let state = load_state(&root)?;
    if !same_inputs(plan, &fresh)? {
        return Err(Error::Transaction(
            "approved inputs changed while acquiring the lock",
        ));
    }
    if state
        .as_ref()
        .is_some_and(|(state, _)| state.last_plan_sha256 == expected)
    {
        let bases_match = state.as_ref().is_some_and(|(state, _)| {
            let change = &plan.changes[0];
            let base = state.managed.get(TOOLCHAIN);
            (change.disposition == "unchanged" || base.is_some())
                && base.is_none_or(|base| {
                    base.sha256 == change.after_sha256 && base.content == change.proposed_content
                })
        });
        if bases_match
            && plan.changes.iter().all(|change| {
                optional_bytes(&root, &change.path).is_ok_and(|bytes| {
                    bytes.as_deref().map(digest).as_deref() == Some(change.after_sha256.as_str())
                })
            })
        {
            return Ok(receipt(plan, "unchanged", true));
        }
        return Err(Error::Transaction(
            "committed plan receipt conflicts with managed files",
        ));
    }
    if fresh.digest()? != expected {
        return Err(Error::Transaction(
            "plan preimages or ownership state changed",
        ));
    }
    let mut managed = state
        .as_ref()
        .map(|(state, _)| state.managed.clone())
        .unwrap_or_default();
    let mut operations = Vec::new();
    for change in &plan.changes {
        if change.disposition == "unchanged" {
            continue;
        }
        let before = optional_bytes(&root, &change.path)?
            .map(|bytes| text(&bytes))
            .transpose()?;
        operations.push(FileWrite {
            path: change.path.clone(),
            before,
            after: change.proposed_content.clone(),
        });
        managed.insert(
            change.path.clone(),
            Base {
                sha256: change.after_sha256.clone(),
                content: change.proposed_content.clone(),
            },
        );
    }
    let next = State {
        schema_version: VERSION,
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        last_plan_sha256: expected.into(),
        managed,
    };
    operations.push(FileWrite {
        path: STATE.into(),
        before: state.map(|(_, bytes)| text(&bytes)).transpose()?,
        after: text(&json(&next)?)?,
    });
    let mut journal = Journal {
        schema_version: VERSION,
        plan: plan.clone(),
        committed: false,
        operations,
    };
    replace(&root, JOURNAL, &json(&journal)?, true)?;
    let result = (|| {
        for (index, operation) in journal.operations.iter().enumerate() {
            let current = optional_bytes(&root, &operation.path)?;
            if !matches_bytes(current.as_deref(), operation.before.as_deref()) {
                return Err(Error::Transaction("file changed before atomic replacement"));
            }
            replace(
                &root,
                &operation.path,
                operation.after.as_bytes(),
                current.is_none(),
            )?;
            after_write(index)?;
        }
        journal.committed = true;
        replace(&root, JOURNAL, &json(&journal)?, false)?;
        Ok(())
    })();
    if let Err(error) = result {
        if rollback(&root, &journal).is_err() {
            return Err(Error::Transaction(
                "apply failed and recovery is required; journal preserved",
            ));
        }
        return Err(error);
    }
    remove_journal(&root)?;
    Ok(receipt(plan, "applied", true))
}

fn validate_journal(journal: &Journal, expected: &str) -> Result<()> {
    validate_plan(&journal.plan, expected)?;
    if journal.schema_version != VERSION
        || journal.plan.changes[0].disposition == "conflict"
        || journal.operations.is_empty()
        || journal.operations.len() > 2
    {
        return Err(Error::Transaction("invalid recovery journal"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for operation in &journal.operations {
        if !seen.insert(&operation.path) {
            return Err(Error::Transaction("duplicate recovery target"));
        }
        match operation.path.as_str() {
            TOOLCHAIN => {
                let change = &journal.plan.changes[0];
                if operation.after != change.proposed_content
                    || operation
                        .before
                        .as_deref()
                        .map(|bytes| digest(bytes.as_bytes()))
                        != change.before_sha256
                {
                    return Err(Error::Transaction(
                        "journal target differs from approved plan",
                    ));
                }
            }
            STATE => {
                if operation
                    .before
                    .as_deref()
                    .map(|bytes| digest(bytes.as_bytes()))
                    != journal.plan.state_sha256
                {
                    return Err(Error::Transaction("journal ownership preimage mismatch"));
                }
                let next: State = decode(operation.after.as_bytes())?;
                validate_state(&next)?;
                if next.last_plan_sha256 != expected {
                    return Err(Error::Transaction("journal receipt mismatch"));
                }
                let prior: Option<State> = operation
                    .before
                    .as_deref()
                    .map(|bytes| decode::<State>(bytes.as_bytes()))
                    .transpose()
                    .map_err(|_| Error::Transaction("invalid journal preimage"))?;
                let mut managed = prior
                    .map(|state| {
                        validate_state(&state)?;
                        Ok::<_, Error>(state.managed)
                    })
                    .transpose()?
                    .unwrap_or_default();
                let change = &journal.plan.changes[0];
                if change.disposition != "unchanged" {
                    managed.insert(
                        TOOLCHAIN.into(),
                        Base {
                            sha256: change.after_sha256.clone(),
                            content: change.proposed_content.clone(),
                        },
                    );
                }
                if serde_json::to_vec(&managed).map_err(|_| Error::Json)?
                    != serde_json::to_vec(&next.managed).map_err(|_| Error::Json)?
                {
                    return Err(Error::Transaction("journal alters unrelated ownership"));
                }
            }
            _ => return Err(Error::Transaction("unsupported recovery target")),
        }
    }
    if !seen.contains(&STATE.to_owned())
        || seen.contains(&TOOLCHAIN.to_owned())
            != (journal.plan.changes[0].disposition != "unchanged")
    {
        return Err(Error::Transaction("incomplete recovery inventory"));
    }
    Ok(())
}

/// Explicitly recover a matching journal. Uncommitted work rolls back; committed
/// work finalizes only after all exact published local bytes match the receipt.
pub fn recover(root: &Path, expected: &str) -> Result<Receipt> {
    if !cfg!(unix) {
        return Err(Error::Transaction(
            "durable transactions require a supported Unix host",
        ));
    }
    let root = root.canonicalize()?;
    let bytes = optional_bytes(&root, JOURNAL)?.ok_or(Error::Transaction("no recovery journal"))?;
    let journal: Journal = decode(&bytes)?;
    validate_journal(&journal, expected)?;
    journal.plan.intent.validate(&root)?;
    let _guard = lock(&root)?;
    if optional_bytes(&root, JOURNAL)?.as_deref() != Some(bytes.as_slice()) {
        return Err(Error::Transaction("journal changed while acquiring lock"));
    }
    if journal.committed {
        for operation in &journal.operations {
            if !matches_bytes(
                optional_bytes(&root, &operation.path)?.as_deref(),
                Some(&operation.after),
            ) {
                return Err(Error::Transaction(
                    "committed transaction conflicts with intervening edits",
                ));
            }
        }
        remove_journal(&root)?;
        Ok(receipt(&journal.plan, "commit-recovered", true))
    } else {
        rollback(&root, &journal)?;
        Ok(receipt(&journal.plan, "rolled-back", false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(
            root.path().join("src/lib.rs"),
            "compile_error!(\"no builds\");",
        )
        .unwrap();
        fs::write(root.path().join("LICENSE"), "MIT").unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname = \"pilot\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(root.path().join("armorer.toml"), "schema_version = 1\nrepository = \"example/pilot\"\ntoolchain = \"1.95.0\"\n[[deliverables]]\nid = \"library\"\nprofile = \"library\"\npackage = \"pilot\"\ntargets = [\"x86_64-unknown-linux-gnu\"]\nfeature_set = \"standard\"\n[feature_sets.standard]\ndefault_features = true\nfeatures = []\n[policy]\nlicense_file = \"LICENSE\"\nattestations = \"required\"\n").unwrap();
        root
    }

    fn interrupt(root: &Path, plan: &Plan, index: usize) {
        assert!(
            std::panic::catch_unwind(|| {
                let _ = apply_inner(root, plan, &plan.plan_sha256, |written| {
                    if written == index {
                        panic!("simulated process interruption");
                    }
                    Ok(())
                });
            })
            .is_err()
        );
    }

    #[test]
    fn failed_write_rolls_back_all_completed_bytes() {
        for index in 0..2 {
            let root = fixture();
            let plan = inspect(root.path(), "plan").unwrap();
            assert!(
                apply_inner(root.path(), &plan, &plan.plan_sha256, |written| {
                    if written == index {
                        return Err(Error::Transaction("simulated write failure"));
                    }
                    Ok(())
                })
                .is_err()
            );
            assert!(!root.path().join(TOOLCHAIN).exists());
            assert!(!root.path().join(STATE).exists());
            assert!(!root.path().join(JOURNAL).exists());
            assert_eq!(
                apply(root.path(), &plan, &plan.plan_sha256)
                    .unwrap()
                    .outcome,
                "applied"
            );
        }
    }

    #[test]
    fn explicit_recovery_rolls_back_after_each_crash_boundary() {
        for index in 0..2 {
            let root = fixture();
            let plan = inspect(root.path(), "plan").unwrap();
            interrupt(root.path(), &plan, index);
            assert!(root.path().join(JOURNAL).exists());
            assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
            assert!(recover(root.path(), &"a".repeat(64)).is_err());
            assert_eq!(
                recover(root.path(), &plan.plan_sha256).unwrap().outcome,
                "rolled-back"
            );
            assert!(!root.path().join(TOOLCHAIN).exists());
            assert!(!root.path().join(STATE).exists());
            assert_eq!(
                apply(root.path(), &plan, &plan.plan_sha256)
                    .unwrap()
                    .outcome,
                "applied"
            );
        }
    }

    #[test]
    fn recovery_conflict_preserves_every_file_and_journal() {
        let root = fixture();
        let plan = inspect(root.path(), "plan").unwrap();
        interrupt(root.path(), &plan, 1);
        fs::write(root.path().join(TOOLCHAIN), "intervening customization").unwrap();
        let journal = fs::read(root.path().join(JOURNAL)).unwrap();
        let state = fs::read(root.path().join(STATE)).unwrap();
        assert!(recover(root.path(), &plan.plan_sha256).is_err());
        assert_eq!(fs::read(root.path().join(JOURNAL)).unwrap(), journal);
        assert_eq!(fs::read(root.path().join(STATE)).unwrap(), state);
        assert_eq!(
            fs::read(root.path().join(TOOLCHAIN)).unwrap(),
            b"intervening customization"
        );
    }

    #[test]
    fn failed_apply_with_intervening_edit_requires_recovery() {
        let root = fixture();
        let plan = inspect(root.path(), "plan").unwrap();
        let error = apply_inner(root.path(), &plan, &plan.plan_sha256, |_| {
            fs::write(root.path().join(TOOLCHAIN), "external edit")?;
            Err(Error::Transaction("injected failure"))
        })
        .unwrap_err();
        assert!(error.to_string().contains("recovery is required"));
        assert!(root.path().join(JOURNAL).exists());
        assert_eq!(
            fs::read(root.path().join(TOOLCHAIN)).unwrap(),
            b"external edit"
        );
    }

    #[test]
    fn committed_journal_finalizes_only_exact_inventory() {
        let root = fixture();
        let plan = inspect(root.path(), "plan").unwrap();
        interrupt(root.path(), &plan, 1);
        let mut journal: Journal = decode(&fs::read(root.path().join(JOURNAL)).unwrap()).unwrap();
        journal.committed = true;
        fs::write(root.path().join(JOURNAL), json(&journal).unwrap()).unwrap();
        assert_eq!(
            recover(root.path(), &plan.plan_sha256).unwrap().outcome,
            "commit-recovered"
        );
        assert_eq!(
            apply(root.path(), &plan, &plan.plan_sha256)
                .unwrap()
                .outcome,
            "unchanged"
        );
    }

    #[test]
    fn malformed_or_redirected_journal_never_writes_other_paths() {
        let root = fixture();
        let plan = inspect(root.path(), "plan").unwrap();
        interrupt(root.path(), &plan, 0);
        let mut journal: Journal = decode(&fs::read(root.path().join(JOURNAL)).unwrap()).unwrap();
        journal.operations[0].path = "LICENSE".into();
        fs::write(root.path().join(JOURNAL), json(&journal).unwrap()).unwrap();
        assert!(recover(root.path(), &plan.plan_sha256).is_err());
        assert_eq!(fs::read(root.path().join("LICENSE")).unwrap(), b"MIT");
        assert!(root.path().join(JOURNAL).exists());
    }

    #[test]
    fn owned_refresh_is_allowed_only_with_matching_base() {
        let root = fixture();
        let plan = inspect(root.path(), "plan").unwrap();
        apply(root.path(), &plan, &plan.plan_sha256).unwrap();
        let mut state = load_state(root.path()).unwrap().unwrap().0;
        // Model an earlier reviewed generated base, without installing another compiler.
        let old = plan.changes[0]
            .proposed_content
            .replace("profile = \"minimal\"", "profile = \"default\"");
        state.managed.insert(
            TOOLCHAIN.into(),
            Base {
                sha256: digest(old.as_bytes()),
                content: old.clone(),
            },
        );
        fs::write(root.path().join(STATE), json(&state).unwrap()).unwrap();
        fs::write(root.path().join(TOOLCHAIN), old).unwrap();
        let next = inspect(root.path(), "plan").unwrap();
        assert_eq!(next.changes[0].disposition, "update");
        assert!(
            apply_inner(root.path(), &next, &next.plan_sha256, |_| Err(
                Error::Transaction("write failure")
            ))
            .is_err()
        );
        assert_eq!(
            inspect(root.path(), "plan").unwrap().changes[0].disposition,
            "update"
        );
        apply(root.path(), &next, &next.plan_sha256).unwrap();
        assert_eq!(
            fs::read(root.path().join(TOOLCHAIN)).unwrap(),
            next.changes[0].proposed_content.as_bytes()
        );
    }
}
