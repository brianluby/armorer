# Review findings — group C_core (23 findings)

─── src/discovery.rs:428-430 ───
[security · medium] `rustc` is only `is_file()`-checked, while `cargo` above is canonicalized and
root-checked. `is_file()` follows symlinks, so a `rustc` symlink in the toolchain bin dir pointing
into the workspace passes this check and is then executed via `RUSTC` during `cargo metadata` —
defeating the "never execute repository binaries" invariant (reachable when forwarded
`RUSTUP_HOME`/`HOME` resolve toolchains under attacker influence). Canonicalize `rustc` and reject
it when it resolves under `root`, mirroring the cargo check.

─── src/discovery.rs:470-470 ───
[bug · medium] Process supervision is not fully bounded or leak-free: the `rustup which` lookup
above uses blocking `.output()` with no timeout, so a hung rustup blocks discovery indefinitely
despite the advertised bound; and here `child.try_wait()?` (and `std::fs::metadata(&stdout_path)?`
below) propagate I/O errors without killing the spawned cargo child, orphaning a running process.
Apply the same timeout/size supervision to the lookup and ensure the child is killed+reaped on every
error exit (e.g. a kill-on-drop guard).

─── src/discovery.rs:147-147 ───
[bug · low] Backslash is replaced unconditionally, but on Unix (the only supported host) `\` is a
legal filename byte. A repo file such as `foo\bar.rs` is rewritten to `foo/bar.rs`, corrupting the
`inputs` key and the snapshot output path (`destination.join(rel)` then targets a nonexistent
directory, or silently collides with a real `foo/bar.rs`). Only normalize separators on Windows, or
reject backslash-containing names like `contained_path` does.

─── src/discovery.rs:130-130 ───
[bug · low] A broken or racy symlink candidate makes `canonicalize()?` abort the entire discovery
instead of skipping to the next PATH entry — inconsistent with the directory `canonicalize` above,
which uses `let Ok(..) else { continue }`. A stray broken `rustup` symlink early in PATH thus fails
discovery even when a valid rustup exists later. Use `let Ok(resolved) = candidate.canonicalize()
else { continue };`.

-         let resolved = candidate.canonicalize()?;
+         let Ok(resolved) = candidate.canonicalize() else {
+             continue;
+         };

─── src/filesystem.rs:3-3 ───
[maintainability · medium] `OpenOptions` is only referenced inside the `#[cfg(unix)]` block of
`private_file`, but it is imported unconditionally. On a non-unix target (which this module
explicitly supports via its `#[cfg(not(unix))]` branches) this triggers an `unused_imports` warning
and fails any `-D warnings` build. Move it into the unix block next to `OpenOptionsExt`.

-     fs::{File, OpenOptions},
+     fs::File,

─── src/filesystem.rs:77-77 ───
[test · low] The symlink is created pointing at an existing file, so `AlreadyExists` here does not
distinguish "no clobber" from "no symlink follow" — the doc promise covers symlinks generally. Add a
dangling-symlink case: if the implementation ever followed links, it would create the missing target
instead of failing.

─── src/apply.rs:506-510 ───
[bug · medium] `is_ok_and` swallows every `optional_bytes` error (permission denied,
symlink/non-regular replacement, oversized file) and reports it as the misleading "committed plan
receipt conflicts with managed files". Everywhere else in this file (e.g. the write loop below)
these errors are propagated with `?`. Propagate here too so an I/O failure isn't misdiagnosed as a
content conflict, e.g.:

```rust
let mut files_match = true;
for change in &plan.changes {
    let bytes = optional_bytes(&root, &change.path)?;
    if bytes.as_deref().map(digest).as_deref() != Some(change.after_sha256.as_str()) {
        files_match = false;
        break;
    }
}
if bases_match && files_match { ... }
```

─── src/apply.rs:314-316 ───
[security · medium] Check-then-act: the regular-file check stats the *path*, then
`OpenOptions::open` re-resolves it and follows symlinks. If `apply.lock` is swapped for a symlink
between the check and the open, the tool opens (and flocks) the link target instead of failing,
defeating the "apply lock must be a regular file" guard. The check is also redundant with
`safe_path` under the stable-filesystem assumption, yet ineffective without it. Cheaper and
race-free: open first, then validate the opened handle — `let file =
OpenOptions::new()...open(path)?; if !file.metadata()?.is_file() { return Err(...); }` — which also
drops the `try_exists` + `symlink_metadata` double stat (and its vanish-between-calls NotFound
error). Note `directory.is_dir()` above likewise follows symlinks, unlike the symlink-rejecting
posture of `safe_path`.

─── src/apply.rs:301-301 ───
[bug · low] The ignored `unlock()` result contradicts the struct's own invariant: the doc on
`TransactionLock` states that closing the parent descriptor does NOT release the flock while a
spawned child holds a duplicate, so the commented "file closure remains a fallback" is exactly the
case that doesn't work. If `unlock()` fails (e.g. `EINTR`), the lock silently stays held and every
later transaction fails with "another Armorer transaction is active" until the child exits. Retry at
least on `ErrorKind::Interrupted` before relying on close.

─── src/apply.rs:601-602 ───
[bug · low] At this point the transaction is fully committed (targets written, `committed=true`
marker persisted). If `remove_journal` fails, the bare I/O error propagates and looks like an apply
failure, even though a subsequent `apply` will refuse with "unfinished transaction requires explicit
recovery" and only `recover` can finalize. The commit-marker failure just above carefully wraps
errors with "recovery is required; journal preserved" — give this path the same context, e.g.
`remove_journal(&root).map_err(|cause| Error::TransactionCause { message: "transaction committed but
journal cleanup failed; recovery is required", cause: Box::new(cause) })?;`.

─── src/lib.rs:31-31 ───
[maintainability · medium] `Error::Io` is not read-only: write-side calls propagate through this
`#[from]` too (`fs::remove_file(&path)?` in apply.rs:398/411 and bootstrap/transaction.rs:240/265,
`fs::create_dir`). A failed delete/rollback therefore reports "cannot read local input", which
misdirects debugging of transaction failures. The message also carries no path (`io::Error`'s
Display omits it), so CI users only see e.g. `cannot read local input: No such file or directory (os
error 2)`. Use a neutral message here and attach the path at the call boundary.

-     #[error("cannot read local input: {0}")]
+     #[error("local filesystem operation failed: {0}")]

─── src/lib.rs:126-128 ───
[security · medium] Check-then-act race: `symlink_metadata(path)` and `File::open(path)` resolve the
path twice. If the entry is swapped for a symlink (or FIFO) between the two, the regular-file guard
is bypassed and the read follows the link outside `root` — the same window exists between
`safe_path`'s per-component check and the caller's open/write. On Unix, open with `O_NOFOLLOW`
(`OpenOptionsExt::custom_flags`) and validate `file.metadata()` from the opened handle; that closes
the final-component window at no extra syscall cost. The doc's "assumes a stable filesystem" caveat
does not hold for a tool whose input tree is an untrusted checkout.

─── src/bootstrap/transaction.rs:426-427 ───
[bug · medium] After the commit marker is durably persisted, a failure here (e.g. unlink/fsync
error) propagates a bare I/O error even though the transaction is already applied. Callers cannot
distinguish this from a failed transaction, and the message gives no hint that `recover` will
finalize the committed journal. Wrap with the same `Error::TransactionCause` context used for the
commit-marker failure above. The committed branch of `recover` has the identical gap.

-     remove_journal(&root)?;
+     remove_journal(&root).map_err(|cause| Error::TransactionCause {
+         message: "bootstrap transaction committed; journal cleanup failed; run recovery to finalize",
+         cause: Box::new(cause),
+     })?;
      Ok(receipt(plan, "applied", true))

─── src/bootstrap/transaction.rs:503-504 ───
[bug · medium] Same post-commit context gap as in `transaction`: the committed inventory has been
fully verified and the transaction is applied, but a cleanup failure here surfaces as a bare I/O
error with no indication that retrying recovery is safe and sufficient.

-         remove_journal(&root)?;
+         remove_journal(&root).map_err(|cause| Error::TransactionCause {
+             message: "bootstrap commit recovered; journal cleanup failed; retry recovery to finalize",
+             cause: Box::new(cause),
+         })?;
          Ok(receipt(&journal.plan, "commit-recovered", true))

─── src/bootstrap/transaction.rs:225-226 ───
[security · medium] `directory.is_dir()` follows symlinks, so the `AlreadyExists` arm accepts a
symlink-to-directory swapped in after the preceding `safe_path` check — weakening this function's
documented "recheck for symlinks" guarantee (the subsequent `create_dir`/`replace` would then
operate through the link). Use `symlink_metadata` so an existing symlink is rejected instead of
silently accepted. This does not eliminate the general check-then-act window (the codebase assumes a
stable filesystem), but it removes the one place that actively follows a link.

              Err(error)
-                 if error.kind() == std::io::ErrorKind::AlreadyExists && directory.is_dir() => {}
+                 if error.kind() == std::io::ErrorKind::AlreadyExists
+                     && directory.symlink_metadata().is_ok_and(|m| m.is_dir()) => {}

─── src/config.rs:210-210 ───
[maintainability · medium] Both loaders collapse every parse failure into a bare `Error::Toml`,
discarding the TOML error's line/column span and error kind (unknown field, type mismatch). For a
config with up to 128 deliverables, "invalid TOML input (values omitted)" gives users nothing to
locate the mistake. The crate's no-echo policy only requires omitting *values* —
`toml::de::Error::span()` and the message kind (e.g. the unknown field name) can be preserved
without echoing config values. Consider carrying the span (and mapping the UTF-8 failure to a
distinct message, since it isn't a TOML syntax problem). Same pattern in `load_lock` below.

─── src/config.rs:325-325 ───
[security · medium] Check-then-act race: `try_exists()` and the later `read_small()` open are two
separate operations, so the file can be swapped between them. Worse, `try_exists()` follows
symlinks: a dangling symlink at `armorer.lock` returns `false` and is silently treated as "no lock",
downgrading a pinned project to unpinned without any error. `optional_bytes` in apply.rs already
handles this correctly via `symlink_metadata` + read. Prefer a single metadata/open step here:
genuine absence → `Ok(None)`, dangling symlink or other I/O failure → propagate. The same
validate-then-reopen window applies to `safe_path` + `read_small` in `load_config`; opening once
with the safety checks applied to the opened handle would close it.

─── src/config.rs:57-57 ───
[maintainability · low] Schema hardcodes `max = 3` while `validate_shape` enforces `d.targets.len()
<= TARGETS.len()`. The moment `TARGETS` gains a fourth triple, the published schema rejects
configurations the runtime accepts (and the enum in `items` below already tracks `TARGETS`
automatically, so only this bound drifts). Derive the bound from the constant instead.

-     #[schemars(length(min = 1, max = 3))]
+     #[schemars(extend("minItems" = 1, "maxItems" = TARGETS.len(), "uniqueItems" = true, "items" = serde_json::json!({"enum": crate::config::TARGETS})))]

─── src/config.rs:78-78 ───
[maintainability · medium] Schema/runtime drift: the schema constrains only the array length and
uniqueness, but `validate_shape` also rejects any feature failing `feature_name` (1–100 bytes,
leading alphanumeric, charset `[A-Za-z0-9_+.-]`). Inputs like `"my feature"` or a 101-char name are
schema-valid yet rejected at runtime, defeating the point of publishing the schema. Add the per-item
pattern so the schema expresses the same contract.

-     #[schemars(extend("uniqueItems" = true))]
+     #[schemars(extend("uniqueItems" = true, "items" = serde_json::json!({"pattern": "^[A-Za-z0-9][A-Za-z0-9_+.-]{0,99}$(?![\\s\\S])"})))]

─── src/config.rs:85-85 ───
[maintainability · low] `license_file` is the only path-bearing string with no schema constraint at
all — the published schema accepts `""`, absolute paths, and traversal strings that
`validate()`/`safe_path` reject at runtime. (Verified the `validate_shape`-only callers in
bootstrap/upgrade plans still route reads through `optional_bytes` → `safe_path`, so there is no
runtime bypass — this is purely a schema-contract gap.) At minimum add `minLength = 1` to keep the
schema aligned with runtime rejection of empty paths.

+     #[schemars(length(min = 1))]
      pub license_file: String,

─── src/config.rs:340-340 ───
[bug · low] Exact string equality against `CARGO_PKG_VERSION` is a defensible strict-pinning
contract, but it interacts badly with the `Lock.runtime_version` schema pattern, which only allows
`x.y.z`. If the crate is ever built as a prerelease (e.g. `0.2.0-rc.1`), no lock file can
simultaneously satisfy the schema regex and this equality check, so `load_lock` fails for *every*
input on prerelease builds. Add a test (or compile-time assertion) that `env!("CARGO_PKG_VERSION")`
matches the same `x.y.z` pattern, or compare parsed `semver::Version` values and relax the pattern
deliberately.

─── src/bootstrap/plan.rs:148-158 ───
[security · medium] The overlap guard compares the config-supplied `license_file` verbatim, but
`safe_path` (used by `optional_bytes` below) accepts any path whose components are all `Normal`, and
`Path::components()` normalizes away repeated separators, trailing slashes, and interior `.`. So
aliases like `".armorer//ci-policy.toml"`, `"armorer.lock/"`, or `".armorer/./ci-policy.toml"` slip
past this check and read a bootstrap control/generated file into the approval-bound license preimage
— exactly what this guard exists to prevent (apply would then overwrite the configured license file
with generated content). Case aliases (e.g. `ARMORER.toml`) also bypass it on case-insensitive Unix
filesystems (macOS). Normalize via `Path::components()` before comparing so the guard matches the
same file identity `safe_path` resolves; `validate()` only calls `validate_shape()` and relies on
this check having run at inspection.

-     if TARGETS.contains(&intent.policy.license_file.as_str())
-         || [
+     let license_path = Path::new(intent.policy.license_file.as_str())
+         .components()
+         .collect::<std::path::PathBuf>();
+     let controls = [
-             STATE,
+         STATE,
-             JOURNAL,
+         JOURNAL,
-             V1_STATE,
+         V1_STATE,
-             V1_JOURNAL,
+         V1_JOURNAL,
-             ".armorer/apply.lock",
+         ".armorer/apply.lock",
-             "armorer.toml",
+         "armorer.toml",
-         ]
-         .contains(&intent.policy.license_file.as_str())
+     ];
+     if TARGETS
+         .iter()
+         .chain(controls.iter())
+         .any(|target| license_path == Path::new(target))
      {

─── src/bootstrap/plan.rs:293-294 ───
[maintainability · low] `plan.changes.len()` is pinned to `TARGETS.len()`, but the `generated` map's
cardinality is never checked before `zip`. Today `bootstrap_files` always yields exactly the 5
TARGETS keys, so this holds only by a cross-module coincidence: if a future catalog revision returns
a different target set, `zip` silently truncates and leaves trailing changes (or generated targets)
unvalidated in this approval-bound verification path. One cheap length assertion keeps the invariant
local.

      let generated = generated(plan)?;
+     if generated.len() != plan.changes.len() {
+         return Err(Error::Transaction(
+             "bootstrap catalog target set differs from approved plan",
+         ));
+     }
      for (change, (path, after)) in plan.changes.iter().zip(generated) {

