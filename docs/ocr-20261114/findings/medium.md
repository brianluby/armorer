# MEDIUM severity findings (101)

─── schemas/publish-set-v1.json:174-179 ───
[bug · medium] `crates` has no `minItems`/`uniqueItems`, so an empty publish set or the same crate
listed twice validates. A publish-set manifest is the approved input to publication;
duplicates/empties should fail closed here rather than relying on downstream code. Note
`uniqueItems` only rejects byte-identical entries — duplicate name+version pairs with different
archives still need a check in the consumer.

      "crates": {
        "items": {
          "$ref": "#/$defs/CrateIntent"
        },
-       "type": "array"
+       "minItems": 1,
+       "type": "array",
+       "uniqueItems": true
      },

─── schemas/publish-set-v1.json:32-35 ───
[bug · medium] Dependency names are unconstrained strings with no `uniqueItems`, while the "names
within this approved set" rule exists only in prose. Reuse the crate-name pattern used across the
other schemas (`^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$`) so dangling/empty/injection-shaped names are
rejected at the boundary; referential integrity itself must still be asserted by the consumer.

            "items": {
+             "pattern": "^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$(?![\\s\\S])",
              "type": "string"
            },
-           "type": "array"
+           "type": "array",
+           "uniqueItems": true

─── schemas/publish-set-v1.json:155-155 ───
[bug · medium] GitHub Actions accepts both `.yml` and `.yaml` workflow files, so a legitimate
`.yaml` workflow identity is rejected here. (The same restriction is repeated in the sibling schemas
— fix the pattern consistently rather than only in this file.)

-           "pattern": "^\\.github/workflows/[A-Za-z0-9][A-Za-z0-9_.-]*\\.yml$(?![\\s\\S])",
+           "pattern": "^\\.github/workflows/[A-Za-z0-9][A-Za-z0-9_.-]*\\.ya?ml$(?![\\s\\S])",

─── schemas/native-catalog-v2.json:403-412 ───
[bug · medium] `previous_catalog` is the catalog-chain link but is absent from the root `required`
list, so an omitted key and an explicit `null` are indistinguishable to any schema-only consumer: a
v2 catalog can silently drop its predecessor identity instead of asserting "no predecessor". The
Rust side (`Option<ByteIdentity>`, always serialized) treats both as `None`, so this only bites
external validators of the published contract — but for an explicitly-selected authority that is
exactly the case worth closing. Add `previous_catalog` to `required` (keeping the `null` branch) so
absence must be stated. Note this file is generated: apply `#[schemars(required)]` to
`NativeCatalogV2::previous_catalog` and regenerate, otherwise the schema-equality test will revert
the edit.

─── scripts/qualify-test-verifier.py:77-77 ───
[security · medium] `exist_ok=True` accepts a pre-existing destination without checking its owner or
permissions, which undermines the `O_EXCL`/0o700 protections below. The documented usage
(`docs/offline-verifier.md`) points at `/tmp/armorer-fixture-gh`; on a shared host another local
user can pre-create that directory world-writable. `O_EXCL` only protects file *creation* — with
write access to the directory the attacker can unlink/replace `gh` (and `qualification.json`) after
this script finishes, so the test would execute an unqualified binary despite all the digest pins.
Since the script's stated contract is "create a new owned output", verify ownership and mode of an
existing directory (or refuse it) before writing.

      destination.mkdir(mode=0o700, parents=True, exist_ok=True)
+     status = destination.stat()
+     if status.st_uid != os.getuid() or status.st_mode & 0o077:
+         raise ValueError("output directory must be owned by the current user and not group/world accessible")

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

─── src/filesystem.rs:3-3 ───
[maintainability · medium] `OpenOptions` is only referenced inside the `#[cfg(unix)]` block of
`private_file`, but it is imported unconditionally. On a non-unix target (which this module
explicitly supports via its `#[cfg(not(unix))]` branches) this triggers an `unused_imports` warning
and fails any `-D warnings` build. Move it into the unix block next to `OpenOptionsExt`.

-     fs::{File, OpenOptions},
+     fs::File,

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

─── src/config.rs:78-78 ───
[maintainability · medium] Schema/runtime drift: the schema constrains only the array length and
uniqueness, but `validate_shape` also rejects any feature failing `feature_name` (1–100 bytes,
leading alphanumeric, charset `[A-Za-z0-9_+.-]`). Inputs like `"my feature"` or a 101-char name are
schema-valid yet rejected at runtime, defeating the point of publishing the schema. Add the per-item
pattern so the schema expresses the same contract.

-     #[schemars(extend("uniqueItems" = true))]
+     #[schemars(extend("uniqueItems" = true, "items" = serde_json::json!({"pattern": "^[A-Za-z0-9][A-Za-z0-9_+.-]{0,99}$(?![\\s\\S])"})))]

─── src/main.rs:390-393 ───
[bug · medium] A missing HOME (and no --owner-home) is an environment/config problem, but it is
surfaced as Error::Json, which renders as "JSON serialization failed" with code "json" (see lib.rs).
Operators get a wrong category and no hint about the real cause. Use a descriptive variant, e.g.
Error::Invalid("owner home not provided and HOME is unset".into()). The same misleading Error::Json
is used below for the missing-approval invariant (approval.as_ref().ok_or(Error::Json)?), which
likewise masks a logic error as a serialization failure.

─── src/main.rs:387-389 ───
[maintainability · medium] The Plan early-return invariant is enforced only by convention across
three separate matches (approval, state_directory/owner_home/read_only, and the final value match),
so both unreachable!() arms here and below will panic in a production CLI path if a variant is added
or the guard is refactored. Prefer handling Plan inside a single match (or returning a typed Error)
so the compiler enforces exhaustiveness rather than relying on a runtime panic with no message.

─── src/main.rs:530-530 ───
[maintainability · medium] This context/verifier/SBOM/AuthenticatedReleaseFiles/Apple verification
sequence duplicates freeze_publication_inputs almost verbatim. Because it is security-sensitive, a
fix or reordering applied to only one copy can silently weaken the other. Extract a single shared
helper that both VerifyRelease and publication freezing call.

─── src/main.rs:571-571 ───
[maintainability · medium] The +1 hard-codes the assumption that exactly one extra bundle (the
inventory_proof) is verified beyond attestations(). It is correct today because inventory_proof is
tracked separately, but if that model changes this security-audit count silently misreports
verification coverage. Derive the total from an explicit API (e.g. a verified_bundle_count() on
AuthenticatedReleaseFiles) instead of a magic constant.

─── src/main.rs:764-764 ───
[bug · medium] On serialization or stdout write failure this exits 1 with no diagnostic at all — the
operator sees empty output and cannot tell what went wrong (and a partial JSON document may already
be on stdout). Emit a short message to stderr before exiting so failures are observable. The
pervasive map_err(|_| Error::Json) similarly discards the underlying serde error detail.

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

─── src/plan.rs:120-120 ───
[maintainability · medium] `toolchain_change` already probed `rust-toolchain` existence, but it
collapses "legacy file present" and "content differs" into the same `"conflict"` string, forcing
this guard to re-probe the filesystem (a third probe happens again in the `input_preimages` loop).
Besides the duplicated I/O, the two probes can disagree under concurrent mutation: if the legacy
file is removed between them, a legacy-caused conflict whose base hash matches gets promoted to
`"update"`. Return the legacy flag from `toolchain_change` (e.g. `(Change, bool)`) or distinguish
the conflict cause, and reuse it here instead of re-checking.

─── src/plan.rs:45-45 ───
[maintainability · medium] `disposition` is a free-form `String` compared against magic literals
(`"conflict"`, `"update"`, `"unchanged"`, `"create"`) here and across apply/bootstrap, and the
generated schema enforces no enum — a typo silently changes plan semantics (e.g. the
conflict-to-update promotion or the `toolchain-conflict` finding would just stop firing). Model it
as an enum (`Create`/`Unchanged`/`Update`/`Conflict`) with `#[serde(rename_all = "lowercase")]` so
the wire format stays identical while the compiler and JSON schema reject invalid states. The same
applies to `mode`, `state`, and `capability_state` on `Plan`.

─── src/trust/capability.rs:164-164 ───
[security · medium] The Apple signing gate hard-codes the single triple `"aarch64-apple-darwin"`
instead of recognizing Apple targets structurally. Today `config::TARGETS` only contains this one
Apple triple, so there is no live bypass — but the gate's correctness silently depends on that
constant in another module never growing. Adding `x86_64-apple-darwin` (or any future macOS triple)
to `TARGETS` would let Apple deliverables pass `CapabilityConfig::validate` without
`AppleSigning`/`ProtectedSigningEnvironment` being Required. Match the platform suffix (or share a
constant/helper with `config::TARGETS`) so the gate can't drift out of sync.

-                 && d.targets.iter().any(|t| t == "aarch64-apple-darwin")
+                 && d.targets.iter().any(|t| t.ends_with("-apple-darwin"))

─── src/trust/capability.rs:275-279 ───
[security · medium] Lock-to-catalog pin binding is ambiguous across adapters: pin-name uniqueness
(`pins.insert(&pin.name)`) is enforced only within a single adapter, and this `.any()` matches a
lock tool against pins from all adapters. Two adapters can therefore pin the same tool name with
different versions/distributions; the lock tool binds to whichever happens to match, while the other
adapter's conflicting pin for the same tool goes unchecked against the lock. Hoist the `pins` name
set above the adapter loop so pin names are unique catalog-wide (each lock tool then maps to exactly
one pinned distribution). Note: `authentication_record` can't be compared here — `config::ToolPin`
carries only version/sha256 — so name uniqueness is the available guard.

─── src/trust/capability.rs:264-265 ───
[security · medium] `pin.version` is only checked for non-emptiness after trim plus a byte-length
cap, so values like `" 1.0 "`, `"../../x"`, or strings containing newlines/control bytes pass
validation — inconsistent with `pin.name` (identifier-checked) and `runtime_versions`
(`stable_version`). Since these sidecar strings feed downstream fixed-location URL/artifact
resolution, constrain them at this trust boundary. The crate already has `super::asset_name` (ASCII
alphanumeric start, then alnum/`_`/`.`/`-`), which covers semver, dates, and commit-style versions;
reuse it instead of the trim check (or `stable_version` if all pin versions are guaranteed SemVer).

-                         && !pin.version.trim().is_empty()
-                         && pin.version.len() <= 100,
+                         && pin.version.len() <= 100
+                         && super::asset_name(&pin.version),

─── src/trust/inventory.rs:64-65 ───
[performance · medium] The 8192-spec cap and name/uniqueness checks run only after every name string
has been allocated. This is a `pub` function that never calls `config.validate_shape()` or
`policy.validate()`, so an unvalidated config with huge `deliverables`/`targets` does unbounded
allocation work before rejection. Add a cheap incremental guard so work is bounded even for
unvalidated input (the final check still enforces exact uniqueness/naming).

          for target in &d.targets {
+             require(specs.len() <= 8192, "invalid-derived-asset-set")?;
              let key = format!("{}--{}--{}", d.id, target, d.feature_set);

─── src/trust/inventory.rs:252-255 ───
[security · medium] The `len == assets.len() + 2` equality only proves an exact published set if
asset names are unique AND exclude `INVENTORY_NAME`/`INVENTORY_BUNDLE_NAME`. Uniqueness is enforced
below, but the reserved-name exclusion lives only in `validate_against`. If this method is called on
an inventory that hasn't been through `validate_against` (the precondition is not documented here),
an asset named `INVENTORY_NAME` overlaps with the reserved key, so the count check passes while
`supplied` contains one extra, never-verified file — fail-open for a "complete published set" check.
Enforce the exclusion here (or reject up front) instead of relying on an unenforced caller
invariant.

+         for a in &self.assets {
+             require(
+                 a.name != INVENTORY_NAME && a.name != INVENTORY_BUNDLE_NAME,
+                 "inventory-hash-cycle",
+             )?;
+         }
          require(
              supplied.len() == self.assets.len() + 2,
              "published-asset-set-mismatch",
          )?;

─── src/trust/inventory.rs:265-269 ───
[performance · medium] Comparing via `serde_json::to_value` builds two full JSON trees, silently
assumes the serde representation stays injective for these types (any future
`skip_serializing_if`/lossy field would make distinct inventories compare equal in this security
check), and collapses failures into a context-free `Error::Json`. `Asset` and `InputIdentity`
already implement `PartialEq`; just add `PartialEq` to `ReleaseInventory`'s derive list and compare
directly — exact, allocation-free, representation-independent.

-         require(
-             serde_json::to_value(&parsed).map_err(|_| crate::Error::Json)?
-                 == serde_json::to_value(self).map_err(|_| crate::Error::Json)?,
-             "published-inventory-content-mismatch",
-         )?;
+         require(parsed == *self, "published-inventory-content-mismatch")?;

─── src/trust/policy.rs:206-208 ───
[security · medium] These public acceptance helpers perform only membership checks, but
`VerificationPolicy` is freely constructible (public fields + public `Deserialize`) and mutable, so
nothing enforces that `validate(now)` ran first. A caller that skips it silently loses repository
pinning, review expiry, duplicate rejection, the five-scope signer baseline, and the hosted-runner
guarantee — exactly the properties this trust policy exists to enforce. All current call sites do
validate first, but that is a convention, not a guarantee, and post-validation mutation of the
public fields also invalidates it. Minimal fix: document the precondition on both helpers (suggested
below). Stronger fix: have `validate()` consume `self` and return a `ValidatedPolicy` newtype that
alone exposes `accept_source`/`accept_signer`, making unvalidated acceptance unrepresentable.

+     /// Membership check only. Callers MUST run `validate(now)` on this policy
+     /// first and not mutate it afterwards; this helper does not re-check
+     /// repository pinning, review expiry, duplicates or the signer baseline.
      pub fn accept_source(&self, source: &Source) -> Result<()> {
          require(self.sources.contains(source), "source-not-allowed")
      }

─── src/trust/mod.rs:71-71 ───
[maintainability · medium] Schema/runtime contract drift: `git_ref` is emitted as an unconstrained
string, while `Source::validate` accepts only safe `refs/heads/` branches (≤255 bytes, no `..`, no
segment ending in `.` or `.lock`), canonical `refs/tags/vX.Y.Z` tags, or canonical
`refs/pull/<n>/merge` refs. Schema-driven consumers (editors, policy validators, other-language
tooling) see a materially weaker contract and will accept documents the runtime rejects — or, if
they trust the schema alone, accept arbitrary refs. Mirror the allowlist in the schema; the
per-segment branch rules can't be fully expressed in one regex, so keep `valid_source_ref`
authoritative and treat the pattern as a close approximation.

+     #[schemars(regex(
+         pattern = "^refs/(heads/[A-Za-z0-9][A-Za-z0-9_./-]{0,254}|tags/v(0|[1-9][0-9]{0,19})\\.(0|[1-9][0-9]{0,19})\\.(0|[1-9][0-9]{0,19})|pull/[1-9][0-9]{0,19}/merge)$(?![\\s\\S])"
+     ))]
      pub git_ref: String,

─── src/trust/mod.rs:322-323 ───
[maintainability · medium] Both decode stages collapse every failure into `Error::Json` (whose
display text is even "JSON serialization failed", misleading for a parse path), so a duplicate-key
rejection — the anti-smuggling signal this visitor exists for — is indistinguishable from a syntax
error or a typed-shape mismatch when diagnosing rejected contracts. Without echoing untrusted
values: in stage 1, `Category::Data` errors can only originate from the duplicate-key visitor (the
parser never emits non-finite floats, and depth/number/trailing-input failures are `Syntax`), so
they can be mapped to a stable distinct code while everything else stays redacted.

-     let strict: UniqueJson = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
+     let strict: UniqueJson = serde_json::from_slice(bytes).map_err(|e| {
+         if e.classify() == serde_json::error::Category::Data {
+             Error::Invalid("duplicate-json-key".into())
+         } else {
+             Error::Json
+         }
+     })?;
      serde_json::from_value(strict.0).map_err(|_| Error::Json)

─── src/trust/evidence.rs:58-58 ───
[maintainability · medium] The key is not injective: `identifier()` (src/config.rs) permits hyphens
with no restriction on consecutive hyphens, so distinct selections can collide, e.g.
deliverable_id="a--x", feature_set="y" vs deliverable_id="a", feature_set="x--y" produce the same
key for the same target. This key determines the expected distributable asset name used to bind the
final step output below (and the same scheme is re-implemented in inventory/verification modules),
so a collision could bind the chain to the wrong artifact. Consider rejecting "--" inside identifier
components used in keys, or using an unambiguous encoding (length-prefixed/escaped separator).

─── src/trust/evidence.rs:527-528 ───
[security · medium] This trust gate decides exception allow-list membership by comparing serialized
JSON values instead of structural equality. `Exception` and all nested types (ByteIdentity, Review,
CapabilityId) already support `PartialEq`, so the serialization round-trip is unnecessary; any
future non-faithful serialization (skip/normalizing attributes) would silently equate distinct
exceptions and weaken the reviewed-exception policy. Also, `map_err(|_| crate::Error::Json)`
discards the serde error context. Derive `PartialEq, Eq` on `Exception` and compare directly.

-                 serde_json::to_value(e).map_err(|_| crate::Error::Json)?
-                     == serde_json::to_value(allowed).map_err(|_| crate::Error::Json)?,
+                 e == allowed,

─── src/upgrade/authority.rs:46-54 ───
[maintainability · medium] This id→catalog binding duplicates `catalog::reviewed_for` (which exists
precisely to "select the compiled catalog for a registry id"), and a third copy of the registry
lives in `catalog::id_for_commit`. A future successor now has to be added to three parallel matches;
if this one drifts (e.g. `"bootstrap-v2"` accidentally bound to `reviewed()`), `select` silently
mints an authority with the wrong workflow pin — and the equality checks in plan/state validation
would happily agree with the wrong pin. Keep the revision mapping here, but delegate catalog
selection to `catalog::reviewed_for(id)` so the id→catalog binding has a single source. The
unknown-id arm still runs first, so the `Error::Transaction` contract is preserved.

-     let (revision, catalog) = match id {
-         "bootstrap-v1" => (1, catalog::reviewed()?),
-         "bootstrap-v2" => (2, catalog::reviewed_v2()?),
+     let revision = match id {
+         "bootstrap-v1" => 1,
+         "bootstrap-v2" => 2,
          _ => {
              return Err(Error::Transaction(
                  "unknown upgrade catalog; no moving pin or fallback is supported",
              ));
          }
      };
+     let catalog = catalog::reviewed_for(id)?;

─── src/trust/publication.rs:85-91 ───
[bug · medium] The doc comment promises per-stage evidence ("a scanner pass cannot advance a
stage"), but validate() only checks that evidence is non-empty: any stage — including
Published/ProvenanceVerified — can be asserted with a single arbitrary ByteIdentity, and
`limitations` is never validated. trust-contracts-v1.md lists "Lifecycle stage, evidence, and
limitations" as the minimum semantic coverage for this record. Either enforce stage-specific
evidence requirements and validate/bound `limitations` (as policy.rs does for its records), or
correct the doc/contract so callers don't assume stage advancement is gated here.

─── src/trust/publication.rs:213-216 ───
[bug · medium] Optional evidence is only validated in the states that require it, and is neither
rejected nor validated when present in other states — unlike `conflict` (exact iff gating) and
`draft_download_receipt` (validated whenever present). Concretely, all of these pass validate(): a
malformed/expired `approval` or `immutable_setting` on an OwnedDraft/Uploaded/Conflict receipt, a
malformed `release_attestation` on Published, a malformed `provenance_verification` on Approved, and
`published_at == 0` or in the future on a non-published receipt. Validate every optional field
whenever present (the pattern already used for draft_download_receipt and registry_bytes) and gate
presence per state. Also consider temporal ordering: for published states, `published_at` is not
ordered against approval.reviewed_at / setting.observed_at, and the immutable-setting freshness is
checked against `now` (validation time) rather than publication time.

─── src/trust/publication.rs:398-406 ───
[bug · medium] `observation` is validated only for RegistryObserved/RegistryBytesVerified; a
malformed ByteIdentity attached to a Prepared/Uploaded/UploadResultUnknown/Conflict receipt passes
unchecked, unlike `registry_bytes` and `index_sha256`, which are validated whenever present.
Validate it whenever present and keep the presence requirement for the observing states.

+             if let Some(o) = &r.observation {
+                 o.validate()?;
+             }
              if matches!(
                  r.state,
                  RegistryState::RegistryObserved | RegistryState::RegistryBytesVerified
              ) {
-                 r.observation
-                     .as_ref()
-                     .ok_or_else(|| crate::Error::Invalid("registry-observation-missing".into()))?
-                     .validate()?;
+                 require(r.observation.is_some(), "registry-observation-missing")?;
              }

─── src/verification/apple.rs:502-503 ───
[bug · medium] These in-process `check_validity` calls run with `kSecCSAllowNetworkAccess` (bit 16)
and have no deadline. Only the `codesign` subprocess is timeout-bounded (60s via `run_process`); the
Security.framework validation here can block on network ticket/cert lookup indefinitely. The
20-minute stage budget (`started.elapsed() <= ...`) is checked only *after* `native()` returns, so a
stalled network validation hangs the verifier rather than being cut off. Consider enforcing a
deadline around the whole native stage (e.g. run it on a bounded worker / watchdog) so network
stalls fail closed within the budget.

─── src/verification/context.rs:287-288 ───
[security · medium] Cargo.lock is the only pinned input whose authenticated bytes are not retained:
config/lock/catalog/policy are digest-checked and parsed from the same read, but here the bytes are
hashed inside the `require!` and dropped. Any later consumer must re-read `Cargo.lock` from disk,
and the file can be swapped after this check (TOCTOU), so the trusted context no longer binds the
content actually used downstream. Read the bytes into a variable, verify the digest, and retain them
(or expose an accessor / require consumers to re-verify every read against
`inputs.cargo_lock_sha256`) so the authenticated snapshot is what gets used.

─── src/verification/context.rs:396-399 ───
[security · medium] Ambiguous composite key: `identifier()` allows consecutive hyphens in
deliverable ids and feature-set names, while targets are fixed `--`-free triples. Distinct artifacts
can therefore produce identical keys, e.g. (id=`x--aarch64-apple-darwin--y`,
target=`aarch64-apple-darwin`, fs=`z`) and (id=`x`, target=`aarch64-apple-darwin`,
fs=`y--aarch64-apple-darwin--z`) both yield `x--aarch64-apple-darwin--y--aarch64-apple-darwin--z`.
`expected_keys` then collapses two config-legal selections into one, the `selections.len() ==
expected_keys.len()` check approves a context covering only one of the two artifacts (the other
silently escapes evidence requirements), and the v3 `root_features` map cannot distinguish them.
Make the encoding injective — minimally reject `--` in `deliverable.id`/`deliverable.feature_set`
here (targets are fixed), or length-prefix/escape components — and apply the same fix to
`Selection::key()` so both sides agree.

─── src/verification/cyclonedx.rs:155-155 ───
[maintainability · medium] All `run_process` failure modes are collapsed into a single misleading
diagnostic. `run_process` can fail due to spawn failure (e.g. exec/loader problem), the 30s timeout,
stdout/stderr exceeding the 1 MiB caps, or a nonzero exit (actual schema errors). Reporting every
one of these as `sbom-schema-validation-failed` tells operators the document is invalid when the
real cause may be infrastructure (a timeout on a large-but-valid 17 MiB SBOM, or a spawn failure),
leading to wrong remediation. This doesn't require echoing untrusted child output: distinguish the
failure classes with distinct fixed codes (e.g. `sbom-validator-execution-failed` /
`sbom-validator-timeout` vs `sbom-schema-validation-failed`) by inspecting the returned error before
discarding it, keeping the fail-closed behavior intact.

─── src/verification/graph.rs:403-406 ───
[bug · medium] `nodes[id]` panics on a missing key. Today the invariant holds because `compare_sbom`
only calls this after `index()` proved node/package bijection and full reachability, but the sibling
reachability walk in `index()` deliberately uses fallible `.get()` for exactly this traversal. If
the coverage invariant ever diverges (e.g. a future caller passes a filtered map, or `index()`
changes), this verification library panics on untrusted input instead of returning a typed error.
Make the walk fallible like `index()`: return `Result<BTreeSet<&'a str>>`, use
`.get(id).ok_or_else(|| crate::Error::Invalid("cargo-graph-node-missing".into()))?`, and propagate
with `?` at the `compare_sbom` call site.

              if required.insert(id) {
+                 let node = nodes
+                     .get(id)
+                     .ok_or_else(|| crate::Error::Invalid("cargo-graph-node-missing".into()))?;
                  pending.extend(
-                     nodes[id]
-                         .dependencies
+                     node.dependencies

─── src/verification/graph.rs:383-385 ───
[bug · medium] `native_linkage` is the only evidence collection here without a uniqueness check:
packages, nodes, edges, features, and contexts all reject duplicates, but repeated `package_id`
entries (up to MAX_PACKAGES) pass, allowing conflicting or inflated linkage evidence for the same
package to be accepted as valid. Deduplicate by `package_id` with a `BTreeSet`, consistent with the
other indexes.

+         let mut native_ids = BTreeSet::new();
          for native in &self.native_linkage {
              require(
-                 packages.contains_key(native.package_id.as_str())
+                 native_ids.insert(native.package_id.as_str())
+                     && packages.contains_key(native.package_id.as_str())

─── src/upgrade/plan.rs:681-681 ───
[security · medium] Offline reconstruction gap: `inspect_bytes` rejects `intent.policy.license_file`
when it overlaps bootstrap targets, state/journal, or control files (`armorer.toml`,
`.armorer/apply.lock`, `rust-toolchain`, ...), but `validate()` only calls `validate_shape()`, which
does not cover that blocklist (verified in src/config.rs). A hand-built packet with a
self-consistent digest and e.g. `license_file = "armorer.lock"` or a state path passes offline
validation, and the rollback consumer relies on offline validation without freshness re-inspection.
Extract the overlap check into a shared helper and call it from both `inspect_bytes` and
`validate()` so the invariant is fixed once at the shared boundary.

─── src/upgrade/transaction.rs:496-496 ───
[bug · medium] After `commit_marker` succeeds the transaction is fully applied, but a failure here
is returned as a bare error with no receipt and no indication of committed state — unlike the
`commit_marker` failure path above, which wraps in `Error::TransactionCause`. Worse:
`remove_journal` deletes the file before `sync_directory`; if the unlink succeeds but the directory
sync fails, the caller gets a plain I/O error and a subsequent `recover()` fails with "no upgrade
recovery journal", leaving no way to confirm the applied state. Wrap this failure with
committed-state context (and recovery guidance) so it is distinguishable from pre-commit failures.

-     remove_journal(&root, &committed)?;
+     if let Err(cause) = remove_journal(&root, &committed) {
+         return Err(Error::TransactionCause {
+             message: "upgrade committed; journal cleanup failed; verify final state or re-run recovery",
+             cause: Box::new(cause),
+         });
+     }

─── src/verification/io.rs:34-36 ───
[security · medium] The dev/ino identity binding only exists on Unix. On other platforms `regular()`
silently degrades to a type+size check, so a same-length file (or symlink) swapped between
`symlink_metadata` and `File::open` passes as "unchanged". The rest of this module fails closed on
non-Unix (`readonly()`, `filesystem::private_file`); do the same here (or add an equivalent identity
check) instead of weakening the guarantee per-platform. Also note that even on Unix, an in-place
mutation before open that preserves inode and size is undetectable — document that content is
trusted as of the opened handle, since all later reads are pinned to it.

─── src/verification/io.rs:151-154 ───
[maintainability · medium] Every distinct failure mode — duplicate keys (a security-relevant
ambiguity/tampering signal), node/depth budget exhaustion, trailing data, and typed conversion —
collapses into `Error::Json`, whose Display is "JSON serialization failed" (misleading: this is
parsing). Callers and logs cannot distinguish an attack signal from a malformed document. Follow the
`require()` pattern used everywhere else in this file: map each stage to a distinct stable code,
e.g. `Error::Invalid("verification-json-structure".into())` / `"verification-json-trailing"` /
`"verification-json-shape"`, without echoing untrusted input.

─── src/verification/publication/mod.rs:413-416 ───
[bug · medium] The journal is never archived or rotated after `Phase::Complete`, and `load`
hard-fails on any plan mismatch. Because the plan identity binds run id/attempt, every subsequent
release (and even a re-run of the same release in a new attempt) for this `repository_id` will hit
`publication-journal-conflict` forever, requiring manual deletion inside the protected state
directory. Archive/rename the journal once the plan reaches `Complete` (e.g., keyed by plan digest)
so the per-repository store can serve the next release.

─── src/verification/publication/mod.rs:393-398 ───
[security · medium] Lock acquisition is check-then-act and the directory trust check is incomplete.
(1) The `symlink_metadata(&path).is_file()` check happens before `OpenOptions::open`; the entry can
be swapped for a symlink between the two calls, so the type guarantee doesn't hold for the file
actually opened/locked — open first (with `O_NOFOLLOW` on unix) and `fstat` the opened descriptor
instead. (2) The permission check verifies mode bits but never verifies the directory is owned by
the current uid; an attacker-owned directory with mode 0700 reachable through a symlinked path
component passes, and that attacker can then swap the journal/lock files at will. Verify ownership
(and re-check after canonicalization) before trusting the store.

─── src/verification/publication/mod.rs:701-704 ───
[maintainability · medium] `Error::Json` is used for state-machine invariant violations, not
serialization failures: missing journal here and in `recover_published`, missing `release_id`
(`journal.release_id.ok_or(Error::Json)`), missing `served_bytes` in approval validation, byte-total
overflow in `freeze`, and unknown asset names in `asset_path`. This mislabels exactly the
wedged-state conditions operators must diagnose (see the Creating/Uploading/Publishing recovery
gaps) as JSON parse errors. Use descriptive `Error::Invalid` codes consistent with the rest of the
module.

          let mut journal = self
              .state
              .load(&self.release.identity)?
-             .ok_or(Error::Json)?;
+             .ok_or_else(|| Error::Invalid("publication-journal-missing".into()))?;

─── src/verification/sigstore.rs:682-683 ───
[bug · medium] The 30s deadline in `run_process_status` only breaks the polling loop; it is not a
hard bound on the call. Two things can block past it:

1. `stop()` does a blocking `wait()` after `kill()`, and both errors are ignored. If `kill()` fails,
or the child is in uninterruptible (D-state) I/O where SIGKILL is only delivered once it leaves that
state, `wait()` blocks indefinitely.
2. After the loop breaks (timeout/error), `std::thread::scope` still joins the two reader threads,
which are blocked in `capture()` until pipe EOF. Killing only the direct child closes its pipe ends,
but if a write-end stays open (e.g. inherited by a descendant) the readers never see EOF and the
join hangs.

For the pinned single-process `gh` this is unlikely, but the stated time bound is not actually
guaranteed, which is a DoS hazard in a verification gate. Consider making teardown non-blocking
(e.g. reap with a bounded/non-blocking wait and ensure reader threads cannot outlive the deadline)
so the timeout holds regardless of child behavior.

─── src/verification/publication/github.rs:729-729 ───
[performance · medium] Upload buffers the entire asset (up to 1 GiB) in memory via `read_bounded`,
then `write` copies it into a temp file that the CLI re-reads, and it is hashed again for the result
comparison — roughly 2x asset size in peak RSS plus duplicated I/O. For large assets this is a
material memory/DoS concern on constrained runners. Prefer streaming the file into the temp input
with an incremental SHA-256/size computation, then compare the provider-returned digest/size against
that streaming identity instead of `ByteIdentity::from_bytes(&bytes)`.

─── src/verification/publication/github.rs:701-704 ───
[bug · medium] The double-observation equality compares hashes of *entire* provider JSON bodies
(`repository`, `current-run`, deployment listings/statuses, approvals). These include volatile
fields — repo `pushed_at`/`updated_at`, run-attempt `updated_at` (which changes as steps progress
during an `in_progress` run), status timestamps — so two consecutive snapshots taken seconds apart
on an active repo/run will frequently differ, producing spurious `publication-prerequisites-changed`
failures even when nothing security-relevant changed. The same applies to `observe_capabilities`.
Consider hashing a canonical projection of the security-relevant fields (ids, settings, states,
shas) rather than whole response bodies, keeping the strict equality on that projection.

─── tests/apply.rs:1-1 ───
[test · medium] This test file is not gated to Unix, but `armorer::apply::apply` unconditionally
returns `Err("durable transactions require a supported Unix host")` on non-Unix hosts
(src/apply.rs). Every test here that calls `apply(...).unwrap()` (or asserts a specific apply error)
will therefore fail on Windows/macOS-less CI targets — only the symlink test is `#[cfg(unix)]`. Gate
the whole file with `#![cfg(unix)]` (or gate each apply-dependent test) so the suite matches the
platform contract of the code under test.

+ #![cfg(unix)]
+
  use armorer::{

─── tests/apply.rs:85-91 ───
[test · medium] The mtime-equality check can pass spuriously: several filesystems have 1–2s mtime
granularity, so a non-idempotent rewrite of `rust-toolchain.toml` within the same timestamp tick
still compares equal. Unlike `state.json`, the toolchain file's bytes are never re-checked after
replay, so even a content-changing rewrite could slip through. Add a deterministic byte-level
assertion after the second apply (keep the mtime check as a secondary signal; note that an
identical-content rewrite in the same tick remains unprovable via mtime alone).

+     assert_eq!(
+         bytes(root.path(), "rust-toolchain.toml"),
+         plan.changes[0].proposed_content.as_bytes()
+     );
      assert_eq!(
          fs::metadata(root.path().join("rust-toolchain.toml"))
              .unwrap()
              .modified()
              .unwrap(),
          modified
      );

─── tests/apply.rs:279-279 ───
[test · medium] Both forged-state scenarios assert only `is_err()`, so an unrelated early failure
(digest validation, input drift, etc.) would mask a regression in the intent/ownership
re-verification this test is meant to protect — e.g. if the `same_inputs` re-check were removed but
the fresh-digest check still rejected the forged plan, this test would keep passing. Pin the
rejection reason (assert the error message/variant: "approved inputs changed" for the forged plan,
"committed plan receipt conflicts with managed files" for the emptied `managed` map), and after each
rejected apply assert `state.json` bytes are unchanged so a failed apply can't silently mutate
ownership state.

─── tests/apply.rs:241-241 ───
[test · medium] Despite the name, this test never verifies that approval is *required*: it only runs
the happy path with `--expect-plan-sha256` supplied. If the CLI regression made the flag optional
(silently auto-approving the plan file), this suite would still pass — the digest gate is only
tested at the `load_plan` library level, not at the CLI argument-wiring level. Add a negative
invocation (omit `--expect-plan-sha256`, and/or pass a wrong digest) asserting a non-zero exit and
no applied files. Also include stderr in the failure diagnostic below — CLI errors are usually
written there, so a failing run currently prints an empty/unhelpful message.

─── tests/bootstrap.rs:50-62 ───
[test · medium] The no-write oracle follows symlinks and opens special files: `path.is_dir()`
follows directory symlinks (walking outside the fixture and misattributing escaped mutations), and
`fs::read` on a FIFO blocks forever. This is why the symlink/FIFO tests can't use `snapshot` at all
and fall back to weaker checks (outside-dir count / `.armorer` absence), missing partial writes
inside the fixture root. Use `entry.file_type()` (does not follow links) and record symlinks/special
entries without opening them, so every test can use the full byte-equality oracle.

-             let path = entry.unwrap().path();
-             if path.is_dir() {
+             let entry = entry.unwrap();
+             let path = entry.path();
+             let kind = entry.file_type().unwrap();
+             let key = path.strip_prefix(root).unwrap().to_str().unwrap().to_owned();
+             if kind.is_dir() {
                  walk(root, &path, files);
+             } else if kind.is_symlink() {
+                 files.insert(key, fs::read_link(&path).unwrap().to_string_lossy().into_owned().into_bytes());
+             } else if kind.is_file() {
+                 files.insert(key, fs::read(&path).unwrap());
              } else {
-                 files.insert(
-                     path.strip_prefix(root)
-                         .unwrap()
-                         .to_str()
-                         .unwrap()
-                         .to_owned(),
-                     fs::read(path).unwrap(),
-                 );
+                 files.insert(key, b"<special>".to_vec());
              }

─── tests/bootstrap.rs:377-378 ───
[test · medium] This test unconditionally spawns the external GNU `patch` binary; on Windows or
minimal CI images without it, `spawn().unwrap()` panics with NotFound instead of the test being
skipped. The other external-tool tests (`symlink`, `mkfifo`) are `#[cfg(unix)]`-gated — gate this
one the same way (or skip when `patch` is unavailable / apply the diffs with an in-crate applier).

+ #[cfg(unix)]
  #[test]
  fn independently_applied_complete_diffs_reconstruct_every_target() {

─── src/verification/publication/tests.rs:281-285 ───
[test · medium] This test only exercises `CapabilityReport::require_ready` directly; it never drives
`c.stage(...)` with degraded gates, so the final `creates == 0` assertion is vacuous (no staging is
ever attempted). `Fake::check` always returns `GateState::Enforced` and offers no way to inject a
non-enforced or missing gate, so a controller regression that skips readiness enforcement before
creating/uploading would not be caught here. Consider adding a gate-state override field to `Fake`
and asserting that `c.stage(&approval)` fails with zero backend writes for each degraded/missing
gate.

─── src/verification/publication/tests.rs:783-785 ───
[test · medium] The listing-cap behavior is not actually verified: all 101 deployments bind the same
run/attempt (10/1) and only deployment 1 has a status entry, so `deployment_gate_states` returns
`(Unknown, Unknown)` due to duplicate attempt bindings / inactive deployments regardless of the >100
cap. The test would still pass if the cap were removed. Make the input discriminating: keep one
deployment bound to the expected run with an active status, give the other 100 distinct run IDs plus
active statuses, so the unbounded result would be `(Enforced, Unknown)` and only the cap can produce
`(Unknown, Unknown)`.

─── src/verification/publication/tests.rs:117-117 ───
[test · medium] The fake discards the maximum-byte bound that `NativeGithub::download` enforces, so
the bounded-download rejection path (served bytes exceeding the expected asset size) is never
exercised by this suite. Honor the bound in the double (e.g. fail when `bytes.len() as u64 >
max_bytes`) and add a test where the served bytes exceed the frozen asset size to confirm
verification fails closed before any publication approval.

-     fn download(&mut self, id: u64, _: u64) -> Result<Vec<u8>> {
+     fn download(&mut self, id: u64, max_bytes: u64) -> Result<Vec<u8>> {

─── src/verification/publication/tests.rs:432-434 ───
[test · medium] This test hardcodes an offset into the backend check counter (`checks + 2`),
coupling it to the current number of publish preflights, and then asserts only that publication
errored with zero publishes. If the preflight count changes, the injected failure lands on a
different check and the test can pass for an unrelated reason. Assert the injected error reason and
the retained journal phase, mirroring `rerun_reviewer_change_at_final_preflight_does_not_publish`.

      c.backend.fail_check_at = Some(c.backend.checks + 2);
-     assert!(c.publish(&approval).is_err());
+     assert!(matches!(
+         c.publish(&approval),
+         Err(Error::Invalid(reason)) if reason == "test-mutable-gate-changed"
+     ));
      assert_eq!(c.backend.publishes, 0);
+     assert_eq!(
+         c.state.load(&c.release.identity).unwrap().unwrap().phase,
+         Phase::DraftVerified
+     );

─── src/verification/publication/tests.rs:620-621 ───
[test · medium] The shared helper pins `actor` and `triggering_actor` to the same identity (1), so
the pure gate decision's exclusion of a rerun initiator distinct from the original actor is never
exercised — `self_approval_blocks_enforcement_only` only covers approver == actor. Parameterize
`triggering_actor` (or add a case with `triggering_actor: Some(3)` and an approval by user 3) and
assert enforcement stays `Unknown` while serialization remains `Enforced`.

─── tests/catalog.rs:105-105 ───
[test · medium] This hardening test only exercises the v1 catalog (`reviewed()` /
`WORKFLOW_COMMIT`). The v2 catalog (`reviewed_v2()`, pinned to `WORKFLOW_COMMIT_V2`) is what the CLI
test asserts against, yet its rendered callers never go through these permission/forbidden-key
checks — a regression in the v2 pin or a v2-specific renderer change would only be caught by the
byte-comparison fixture test, which can be "fixed" by updating the committed fixtures in the same
change. Run the same assertions for both catalogs, parameterized by their expected commit.

─── tests/publication_cli.rs:2-2 ───
[bug · medium] This unconditional `std::os::unix` import breaks compilation of the whole test crate
on non-Unix targets. Sibling tests gate this import (and Unix-only tests) with `#[cfg(unix)]` (see
`tests/integrated_cli.rs`). Since this test also writes `/bin/sh` scripts and sets Unix permission
bits, add `#[cfg(unix)]` to the test function as well.

- use std::{fs, os::unix::fs::PermissionsExt, process::Command};
+ use std::{fs, process::Command};
+ #[cfg(unix)]
+ use std::os::unix::fs::PermissionsExt;

─── tests/real_cyclonedx.rs:25-29 ───
[test · medium] This blanket `is_err()` accepts a failure from any gate, so the negative cases don't
prove what the test claims. In `validate`, errors can come from the byte snapshot, the `specVersion
== "1.5"` pre-check (`unsupported-sbom-document-version`), the duplicate-key JSON parser, or the
native tool (`sbom-schema-validation-failed`). Concretely: the `specVersion: "1.6"`, duplicate-key,
and `"{} {}"` inputs never reach the native tool at all, and the three metadata mutations
(licenses/timestamp/externalReferences) — the whole point of "distinguish malformed metadata from
graph consistency" — would still pass this assertion if a regression made the native step fail for
the wrong reason or be skipped. Pass an expected error code into the helper and match it, e.g.
`Err(armorer::Error::Invalid(code)) if code == expected_code`, using
`"sbom-schema-validation-failed"` for the three field mutations (as the positive test already does
for `verification-input-type-or-size`).

─── tests/integrated_cli.rs:260-260 ───
[test · medium] This fault injection is environment-dependent: mode `0o500` does not block writes
when the test runs as root (common in CI containers) or on filesystems that ignore Unix mode bits,
so the exit-status assertion below fails spuriously even though the CLI is correct (the assertion
message even anticipates this). Additionally, the later `"Permission denied" / "Operation not
permitted"` substring match depends on English-locale `strerror` output. Probe whether the mode
actually blocks writes and skip visibly when the environment bypasses permission checks, so the test
only runs where the intended failure path is real.

      fs::set_permissions(&workflows, fs::Permissions::from_mode(0o500)).unwrap();
+     let probe = workflows.join(".permission-probe");
+     if fs::write(&probe, b"probe").is_ok() {
+         fs::remove_file(&probe).unwrap();
+         drop(restore);
+         eprintln!("skipping: account/filesystem bypasses directory mode bits (root?)");
+         return;
+     }

─── tests/integrated_cli.rs:284-294 ───
[test · medium] Rollback completeness is verified against a hardcoded list of paths, so leftovers at
any unlisted path (temp files, renamed backups, artifacts added as the managed set evolves) would
pass these absence assertions. `before` already captures the full pre-apply byte snapshot —
comparing `snapshot(root.path())` against `before` proves complete restoration, subsumes this list,
and stays correct as managed artifacts change. (Note the snapshot still can't see empty directories;
see the `snapshot` helper comment.)

-     for path in [
-         ".armorer/ci-policy.toml",
-         ".armorer/bootstrap-state-v2.json",
-         ".armorer/bootstrap-journal-v2.json",
-         ".github/workflows/armorer-build.yml",
-         ".github/workflows/armorer-ci.yml",
-         "armorer.lock",
-         "rust-toolchain.toml",
-     ] {
-         assert!(!root.path().join(path).exists(), "{path}");
-     }
+     assert_eq!(snapshot(root.path()), before);

─── tests/integrated_cli.rs:31-37 ───
[bug · medium] `Path::is_dir()` follows symlinks. If the code under test ever leaves a symlink
behind (e.g., an interrupted atomic replacement — exactly the kind of side effect this harness
exists to detect), `snapshot` recurses outside the tempdir or loops forever on a symlink cycle,
hanging/aborting the harness instead of reporting the offending write. Dispatch on non-following
metadata (`fs::symlink_metadata` or `DirEntry::file_type`) and record symlinks as their own entries.
The same issue exists in `copy` (`entry.path().is_dir()` + `fs::copy` follow links). Also note the
files-only byte map cannot observe empty-directory creation/removal or permission changes; consider
recording directory entries too if those guarantees matter.

-         if path.is_dir() {
+         let metadata = fs::symlink_metadata(&path).unwrap();
+         if metadata.file_type().is_symlink() {
+             files.insert(
+                 relative,
+                 fs::read_link(&path)
+                     .unwrap()
+                     .to_string_lossy()
+                     .into_owned()
+                     .into_bytes(),
+             );
+         } else if metadata.is_dir() {
              for (name, bytes) in snapshot(&path) {
                  files.insert(format!("{relative}/{name}"), bytes);
              }
          } else {
              files.insert(relative, fs::read(path).unwrap());
          }

─── tests/preview.rs:32-42 ───
[test · medium] The snapshot only records file contents: `is_dir()` and `fs::read()` both follow
symlinks, and directories, file types, and permission bits are never captured. As a result, the
suite's read-only guarantees (`assert_eq!(snapshot(...), before)`) can pass even if preview/apply
creates an empty directory (e.g. `.armorer/`), changes file modes, or swaps a symlink for a regular
file with identical bytes. Following directory symlinks also risks unbounded recursion on cycles.
Record the entry kind via `entry.file_type()` (which does not follow links), include directory
entries and symlink targets (`fs::read_link`), and capture permission bits so "no mutation"
assertions are actually meaningful.

          for entry in fs::read_dir(path).unwrap() {
-             let path = entry.unwrap().path();
-             if path.is_dir() {
-                 walk(root, &path, output);
-             } else {
+             let entry = entry.unwrap();
+             let path = entry.path();
+             let file_type = entry.file_type().unwrap();
+             let key = path.strip_prefix(root).unwrap().to_string_lossy().into_owned();
+             if file_type.is_symlink() {
                  output.insert(
-                     path.strip_prefix(root).unwrap().to_string_lossy().into(),
-                     fs::read(path).unwrap(),
+                     key,
+                     fs::read_link(&path).unwrap().to_string_lossy().into_owned().into_bytes(),
                  );
+             } else if file_type.is_dir() {
+                 output.insert(format!("{key}/"), Vec::new());
+                 walk(root, &path, output);
+             } else {
+                 output.insert(key, fs::read(&path).unwrap());
              }
          }

─── tests/preview.rs:233-234 ───
[test · medium] The leak assertion scans only stdout, but error diagnostics are typically written to
stderr, so a `DO-NOT-ECHO` leak on stderr would pass. Also, unlike every other test in this file,
this one never snapshots the consumer before/after the failed preview, so a preview that replaced
the symlink with a regular file or created `.armorer/` would go undetected. Check both output
streams and verify the link itself is still a symlink (plus snapshot equality once the snapshot
helper records file types).

      assert_eq!(output.status.code(), Some(1));
      assert!(!String::from_utf8_lossy(&output.stdout).contains("DO-NOT-ECHO"));
+     assert!(!String::from_utf8_lossy(&output.stderr).contains("DO-NOT-ECHO"));
+     assert!(
+         fs::symlink_metadata(root.path().join("rust-toolchain.toml"))
+             .unwrap()
+             .file_type()
+             .is_symlink()
+     );

─── tests/preview.rs:186-189 ───
[test · medium] `unwrap_or_default()` silently converts any read error (not just absence) into empty
content, and for `rust-toolchain.toml` / `rust-toolchain` — which the fixture never creates — this
loop creates a new file rather than mutating an existing input, so the intended "changed saved-plan
input" scenario isn't what gets exercised for those cases. The bare `is_err()` also cannot
distinguish the intended staleness rejection from an unrelated parse/validation failure caused by
the appended text (e.g. an invalid legacy toolchain channel), so the test can pass for the wrong
reason. Handle `NotFound` explicitly and assert the error is specifically the staleness error (its
message contains "stale").

-         let old = fs::read_to_string(&path).unwrap_or_default();
-         fs::write(path, format!("{old}\n# changed\n")).unwrap();
+         let old = match fs::read_to_string(&path) {
+             Ok(text) => text,
+             Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
+             Err(error) => panic!("{input}: {error}"),
+         };
+         fs::write(&path, format!("{old}\n# changed\n")).unwrap();
          let before = snapshot(root.path());
-         assert!(preview_plan(root.path(), &plan).is_err(), "{input}");
+         let error = preview_plan(root.path(), &plan).unwrap_err();
+         assert!(error.to_string().contains("stale"), "{input}: {error}");

─── tests/cargo_graph_v2.rs:136-141 ───
[test · medium] These tamper-rejection loops assert only `is_err()`, so a mutation rejected for an
*unrelated* reason still passes and a regression in the targeted check goes unnoticed. Example:
`/nodes/0/features` is index-based — if fixture ordering drifts, the pointer hits a different node
and the error may come from `invalid-cargo-feature-inventory` or an identity check instead of the
intended `cargo-graph-requested-feature-missing`. The feature tests later in this file already pin
exact error strings; do the same here by pairing each pointer with its expected code
(`cargo-graph-input-mismatch`, `cargo-graph-selection-mismatch`, `cargo-graph-coverage-mismatch`,
etc.), and apply the same to the SBOM mutation loop and the completeness loop below.

─── tests/cargo_graph_v2.rs:613-617 ───
[test · medium] This test runs unconditionally (not `#[ignore]`d) but is Unix-only: `.env_clear()`
strips `SystemRoot`/`TEMP` that Windows processes need, `PATH` is hardcoded to `/usr/bin:/bin`, and
`cargo.with_file_name("rustc")` produces `rustc` without `.exe` on Windows. It will fail on any
non-Unix CI leg. Gate the test with `#[cfg(unix)]` (or set up the environment portably: preserve
required system vars and use the platform executable suffix for `RUSTC`).

─── tests/inspection.rs:316-320 ───
[test · medium] Vacuous assertion: `bytes` is an owned `Vec<u8>` captured from the earlier
`load_lock` call and never re-read, so comparing it to `lock.as_bytes()` after rewriting the file is
trivially true. It cannot detect a re-read/TOCTOU regression, so the stated 'loaded bytes remain
bound to the validated lock' guarantee is never exercised. To actually test binding, re-run the
pipeline (e.g. `inspect`) after the rewrite and assert it hashes/uses the originally validated
bytes, or drop this misleading assertion.

─── tests/inspection.rs:425-428 ───
[test · medium] Environment-sensitive panic: this `.unwrap()` on the PATH search aborts the test on
any host without rustup on PATH (and `var_os("PATH").unwrap()` panics if PATH is unset). Prefer
skipping gracefully (e.g. early `return` / `eprintln!` + return when rustup isn't found) so the
suite doesn't fail on rustup-less CI images.

─── tests/inspection.rs:578-584 ───
[test · medium] Portability/flakiness: hardcoding `/usr/bin/mkfifo` breaks on systems where mkfifo
lives elsewhere (e.g. `/bin/mkfifo`); resolve it via PATH or use a libc/nix mkfifo. Also the fixed
2s wall-clock kill deadline can false-positive under CI load. Consider a more generous/adaptive
timeout.

─── tests/inspection.rs:174-182 ───
[test · medium] On non-unix hosts the provider script is never made executable (the chmod is
`#[cfg(unix)]`) and a `#!/bin/sh` script isn't a runnable Windows executable, so `!marker.exists()`
passes trivially without proving rustc-wrapper/credential-provider exclusion. Gate the whole test
with `#[cfg(unix)]` (or make the provider genuinely executable cross-platform) so it doesn't give
false confidence on Windows.

─── tests/inspection.rs:397-400 ───
[test · medium] Silent no-op risk: if the replace target drifts (whitespace/ordering), `replace`
returns the manifest unchanged and `is_ok()` still passes, so the feature-cycle path is never
actually exercised. Assert the mutation took effect (e.g. `assert_ne!(manifest, mutated)` or assert
the plan reflects the cycle) before asserting success.

─── tests/trust_contracts.rs:601-602 ───
[test · medium] The test name promises unknown-field rejection, but no payload with an unknown field
is ever written — only duplicate keys, oversize, traversal and symlink cases are exercised. Add an
unknown-field case built from a valid record (so it can't fail for missing required fields instead),
e.g. serialize `policy()`, inject an extra key, and assert `load_json::<VerificationPolicy>` rejects
it. Otherwise the claimed `deny_unknown_fields` contract is untested.

      std::fs::write(&path, br#"{"outer":{"required":true,"required":false}}"#).unwrap();
      assert!(armorer::trust::load_json::<serde_json::Value>(temp.path(), "contract.json").is_err());
+     let mut value = serde_json::to_value(policy()).unwrap();
+     value["unexpected_field"] = true.into();
+     std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
+     assert!(armorer::trust::load_json::<VerificationPolicy>(temp.path(), "contract.json").is_err());

─── tests/trust_contracts.rs:171-173 ───
[test · medium] These mutations (traversal name, `INVENTORY_BUNDLE_NAME`, and the self-referencing
subject above) all alter asset[0]'s name/subjects, which already fails the expected-asset-set
comparison — the same rejection the `"extra.tar.gz"` rename case proves. So the bare `is_err()`
assertions pass even if the path-safety, reserved-name, or cycle checks were deleted; they don't
isolate the security property the test name claims. Pin the rejection reason via the error code, as
`producer_cannot_refresh_independently_approved_tool_observation_time` already does with
`unwrap_err().to_string()`.

─── tests/release_context.rs:381-381 ───
[test · medium] Same unsupported-host vacuity as the substitution test: this test is not
`cfg_attr`-gated and asserts only `is_err()`. On hosts where `qualified_native_validator()` fails,
`open` errors at the native-validator gate regardless of symlink handling, so a regression in
symlink rejection would go undetected there. Gate the test like the other semantic tests or assert
the specific symlink/type rejection error code for each case.

─── tests/release_context.rs:165-167 ───
[test · medium] The unsupported-host branch accepts any error, so a broken fixture (bad JSON,
missing file, wrong digest) would still pass here and silently mask loss of the intended coverage.
Assert that the error is specifically the unsupported-native-validator rejection rather than bare
`is_err()`.

─── tests/release_context.rs:1076-1083 ───
[test · medium] On unsupported hosts no assertion is made about the error message at all — any
exit-1 JSON error passes, including one produced by an unrelated early-exit path. Add an `else`
branch asserting the expected unsupported-host/native-validator rejection message so the check is
never silently skipped (the file's own doc comment on `supported()` promises "never skip a test
silently").

─── tests/release_context.rs:598-599 ───
[test · medium] These slices hardcode the exact layout of `synthetic-runtime.tar` (1024-byte stride,
128-byte members at +512). The container-mutation test's fixed offsets
(148/156/257/512/640/1536/3500) and the `len() == 128` assertion share the same coupling. If the
binary fixture drifts, this panics on slice bounds or — worse — the mutation test flips an
unintended byte and the archive-safety assertions silently test the wrong field. Add an explicit
layout sanity check before slicing (e.g., `assert!(archive.len() >= targets.len() * 1024 + 1024)`
plus a ustar-magic/member-size check), or derive offsets from the tar headers, so drift fails with a
clear diagnostic.

─── tests/historical.rs:252-260 ───
[test · medium] Vacuous size-cap assertion: the 1 MiB+1 file is checked against `"a".repeat(64)`,
which can never match its content. If the size gate in `io::read_bounded`/`regular` were removed,
this would still fail with `unapproved-historical-policy` and the test stays green. Use the file's
real digest so only the size cap can reject it, and assert the specific code (`regular()` emits
`verification-input-type-or-size` for `len > max`, before the digest comparison). Likewise, the
`../escape` assertion just above accepts any error — pin it to the asset-name rejection (e.g.
`invalid-historical-asset-name` from policy validation).

-     assert!(
-         HistoricalByteMatch::verify(
+     let digest = ByteIdentity::from_bytes(&fs::read(&path).unwrap()).sha256;
+     let error = HistoricalByteMatch::verify(
-             &directory.path().join("download"),
+         &directory.path().join("download"),
-             &path,
+         &path,
-             &"a".repeat(64),
+         &digest,
-             &source
+         &source
-         )
+     )
-         .is_err()
+     .err()
+     .unwrap();
+     assert!(
+         error.to_string().contains("verification-input-type-or-size"),
+         "{error}"
      );

─── tests/historical.rs:89-91 ───
[test · medium] These markers can never be observed here: `touch executed` and any cargo build
resolve against the *process* working directory (the crate root under `cargo test`), not the tempdir
— and `target/` legitimately exists at the crate root anyway. So actual execution/extraction would
go undetected while these assertions always pass. Exercise the scenario in a child process with an
explicit `.current_dir(directory.path())` (the CLI test already spawns one) so markers land where
they are checked, or assert against the real CWD for `executed`.

─── tests/historical.rs:149-149 ───
[test · medium] Security negatives accept any error: a broken source-pinning guard could hide behind
an unrelated earlier failure (fixture drift, review expiry, digest handling). The implementation
distinguishes `historical-release-not-allowed` (source mismatch) from
`historical-release-mode-required` (non-release ref) — assert those, as the attestation/total-byte
tests already do. Same applies to the bare `is_err()` checks for `modern`, `collision`, and
`rehearsal` below.

-         assert!(verify(directory.path(), &policy, &wrong).is_err());
+         let error = verify(directory.path(), &policy, &wrong).err().unwrap();
+         let message = error.to_string();
+         assert!(
+             message.contains("historical-release-not-allowed")
+                 || message.contains("historical-release-mode-required"),
+             "case {case}: {error}"
+         );

─── tests/historical.rs:195-197 ───
[test · medium] Bare `is_err()` lets a broken byte comparison hide behind set/IO errors. Pin each
case to its intended failure: tampered bytes → `historical-asset-byte-mismatch`; extra file →
`historical-download-file-set-mismatch`; extra directory →
`historical-download-entry-type-or-count`; missing file → a NotFound IO error. At minimum assert the
tamper case's byte-mismatch code — that is the core guarantee of this test.

─── tests/historical.rs:301-301 ───
[test · medium] Any error passes, so a broken symlink/special-file guard can hide behind an
unrelated rejection. Note case 3's socket is also an unapproved extra entry, so a plain file-set
mismatch would mask broken special-file handling. Assert per-case codes: linked policy/asset →
`verification-input-type-or-size` (`regular()` uses `symlink_metadata`), linked download dir →
`historical-download-directory-type`, socket → `historical-download-entry-type-or-count`.

─── tests/historical.rs:113-116 ───
[test · medium] These inputs also fail `VerificationPolicy` schema validation (`{}` is missing every
required field), so `is_err()` passes even with a lenient parser — serde_json accepts duplicate keys
by default. This does not prove trailing-data/duplicate-key rejection. Start from an otherwise-valid
approved policy (e.g. the fixture), append trailing data or duplicate an existing key, recompute the
digest, and assert failure — then only the strict-parse gate can reject it.

─── tests/historical.rs:334-334 ───
[test · medium] Two gaps: (1) the test name claims "never auto-selects policy", but every invocation
passes `--policy` and `--expect-policy-sha256` explicitly — add a run omitting them and assert
nonzero exit so the required-argument contract is actually exercised. (2) On the failure path this
unconditional stdout-JSON unwrap panics with no exit-code/stderr context if the CLI ever reports
errors differently; include status and stderr in the panic message.

-         let result: Value = serde_json::from_slice(&output.stdout).unwrap();
+         let result: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
+             panic!(
+                 "non-JSON stdout ({error}); status: {:?}; stderr: {}",
+                 output.status,
+                 String::from_utf8_lossy(&output.stderr)
+             )
+         });

─── tests/upgrade.rs:393-396 ───
[test · medium] This raw string mutation is fragile and can pass for the wrong reason. For a fresh
fixture the plan's `state_preimages` map may be empty, serializing as `"state_preimages":{}`; the
replacement then yields `{".armorer/state.json":null,}` — a trailing comma, i.e. malformed JSON.
`load_plan` fails at decode with a syntax error, so the final `is_err()` never exercises
null-preimage rejection at all. If the key already exists in the map, you instead get duplicate JSON
keys (serde_json silently keeps the last). Build the mutation via `serde_json::Value` so the
document stays well-formed, and assert the preimage map's prior shape so the test deterministically
exercises strict reconstruction/digest validation rather than an accidental parse failure.

-     let raw = serde_json::to_string(&p).unwrap().replace(
-         "\"state_preimages\":{",
-         "\"state_preimages\":{\".armorer/state.json\":null,",
-     );
+     let mut value = serde_json::to_value(&p).unwrap();
+     let preimages = value["state_preimages"].as_object_mut().unwrap();
+     assert!(!preimages.contains_key(".armorer/state.json"));
+     preimages.insert(".armorer/state.json".into(), serde_json::Value::Null);
+     let raw = serde_json::to_string(&value).unwrap();

─── tests/upgrade.rs:872-875 ───
[test · medium] The plan `p` is computed before the symlink is created, so these negative assertions
may pass for the wrong reason: `upgrade::apply` re-validates freshness against the current
inventory, and replacing `armorer.lock` / `armorer-ci.yml` with a symlink already invalidates the
pre-symlink plan — it would fail even with no symlink defense at all. Likewise `recover` can fail
simply because no valid journal exists. The outside-file and `is_symlink()` checks help, but a
staleness-rejected apply never writes anything anyway, so the symlink overwrite guard is not
actually proven here. Assert the failure cause (e.g. error text identifying the symlink rejection)
for `inspect`, `apply`, and `recover` so the test can't silently degrade into a generic staleness
check.

─── Cargo.toml:9-9 ───
[maintainability · medium] No `include`/`exclude` is set, so `cargo publish` packages everything not
git-ignored: the nested example projects under `examples/` (each with its own
`Cargo.toml`/`Cargo.lock`), all of `tests/fixtures/` (sigstore bundles, `trusted_root.json`,
attestation JSON), and `docs/`. This bloats the published crate and risks silently shipping future
fixtures/generated artifacts — a notable gap for a crate whose purpose is verifiable releases. Add
an explicit allowlist (or at minimum exclude `examples`, `tests`, `docs`) so the package contents
are intentional and reviewable via `cargo package --list`.

  readme = "README.md"
+ include = ["src/**/*", "Cargo.toml", "README.md", "LICENSE"]

─── .github/workflows/development.yml:13-13 ───
[bug · medium] `cancel-in-progress: true` applies to `push` on `main` as well as PRs. A new merge to
`main` will cancel the still-running validation of the previous main commit, so that commit never
produces its `runtime-candidate-*` artifact (which appears intended for downstream consumption,
given `overwrite: false` and per-run unique names). Limit cancellation to PRs so every main commit
completes and uploads its candidate.

-   cancel-in-progress: true
+   cancel-in-progress: ${{ github.event_name == 'pull_request' }}

