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


─── schemas/publish-set-v1.json:37-42 ───
[security · high] `name` and `version` are bare strings (empty, `../`, whitespace, arbitrary text
all pass), unlike every other identity field in this schema. These values are used for crate
identity matching and downstream path/URL construction. Every other schema in `schemas/` constrains
crate names with `^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$`; reuse it here and pin `version` to the same
semver pattern already used by `runtime_version`.

          "name": {
+           "pattern": "^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$(?![\\s\\S])",
            "type": "string"
          },
          "version": {
+           "pattern": "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$(?![\\s\\S])",
            "type": "string"
          }


─── schemas/publish-set-v1.json:155-155 ───
[bug · medium] GitHub Actions accepts both `.yml` and `.yaml` workflow files, so a legitimate
`.yaml` workflow identity is rejected here. (The same restriction is repeated in the sibling schemas
— fix the pattern consistently rather than only in this file.)

-           "pattern": "^\\.github/workflows/[A-Za-z0-9][A-Za-z0-9_.-]*\\.yml$(?![\\s\\S])",
+           "pattern": "^\\.github/workflows/[A-Za-z0-9][A-Za-z0-9_.-]*\\.ya?ml$(?![\\s\\S])",


─── schemas/publish-set-v1.json:101-106 ───
[bug · low] `format: uint64` is annotation-only in 2020-12 (not an assertion), and the full u64
maximum 18446744073709551615 — on `attempt` and `id` here, and on the same fields in
`schemas/release-inventory-v1.json` — exceeds JSON's interoperable safe-integer range (2^53−1 =
9007199254740991). Validators/consumers that parse numbers as IEEE-754 doubles (e.g. JavaScript
`JSON.parse`) round the bound up to 2^64 and accept out-of-range ids/attempts, and silently lose
precision on any value above 2^53, breaking exact identity and byte-exact comparisons in
cross-language verifiers of this trust contract. Either cap the schema at 9007199254740991 or
explicitly require/document lossless big-integer-aware parsing in consumers. Note
`release-inventory-v1.json` is generated and the Rust producer/consumer (`RunIdentity`, u64 via
serde_json) is already exact: apply the cap in the schemars range / `schema_bounds::unsigned`
transform and regenerate, since a hand-edit to that file would be overwritten.



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



─── schemas/native-catalog-v2.json:397-402 ───
[bug · low] `adapters` (and `NativeAdapter.pins`) have no `minItems`, so an empty catalog / an
adapter with zero pins validates structurally while `NativeCatalogV2::validate` rejects it
(`!self.adapters.is_empty()`, `!adapter.pins.is_empty()`). Schema-only consumers therefore accept a
pin-lock authority that locks nothing. Add `minItems: 1` (and the enforced upper bounds 32 / 128 if
you want the schema to match) via the schemars derive on the Rust fields, then regenerate — do not
hand-edit this generated file. Skip `uniqueItems` here: uniqueness is enforced on adapter `id` and
pin `name`, which whole-object `uniqueItems` would not catch.



─── schemas/verification-context-v3.json:651-656 ───
[maintainability · low] The description promises "sorted unique features", but the schema enforces
neither. Consumer-side validation (src/verification/context.rs, resolved_root_features) does enforce
sorting/uniqueness/key linkage, so this is not a correctness hole in the pipeline — but any
schema-only validation of this file accepts duplicate entries that contradict the stated contract.
Uniqueness is cheaply expressible here; add `uniqueItems: true`. (Sortedness and linkage to
release.selections cannot be expressed in draft 2020-12 and correctly remain consumer-enforced.)

        "additionalProperties": {
          "items": {
            "type": "string"
          },
-         "type": "array"
+         "type": "array",
+         "uniqueItems": true
        },


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


─── src/ci_policy.rs:184-184 ───
[security · low] The control-character filter only rejects code points < 32, so DEL (U+007F) and C1
controls (U+0080–U+009F, e.g. CSI U+009B) pass through into `owner`/`reason`, which are presumably
rendered in CI output/review tooling. Since the clear intent here is to sanitize these fields, use
`char::is_control()`, which covers both ranges.

-                     && !value.chars().any(|c| u32::from(c) < 32),
+                     && !value.chars().any(char::is_control),


─── src/ci_policy.rs:172-179 ───
[maintainability · low] `seen.insert(&e.id)` is a side effect hidden as the last conjunct of a
short-circuiting `&&` chain: it only runs if every prior format check passes. It happens to be
correct today (any failure aborts the whole parse), but reordering the conjuncts or making `require`
non-eager would silently break duplicate detection. Split the mutation into its own statement so the
validation condition stays pure.

          require(
              b.len() == 17
                  && b.starts_with(b"RUSTSEC-")
                  && b[12] == b'-'
                  && b[8..12].iter().all(u8::is_ascii_digit)
-                 && b[13..].iter().all(u8::is_ascii_digit)
-                 && seen.insert(&e.id),
+                 && b[13..].iter().all(u8::is_ascii_digit),
          )?;
+         require(seen.insert(&e.id))?;


─── src/catalog.rs:149-151 ───
[bug · low] Latent silent data loss: pins are keyed by `format!("{name}--{target}")` and collected
into a `BTreeMap`, so any duplicate key silently overwrites an earlier platform pin. The currently
authenticated catalog has no `--` in tool names or targets, but a future reviewed catalog update
could introduce a collision and drop a pin with no error on this security-relevant path. A one-line
count check after collect makes it fail loudly instead.

                  })
              })
-             .collect();
+             .collect::<BTreeMap<_, _>>();
+         if tools.len() != self.tools.values().map(|t| t.platforms.len()).sum::<usize>() {
+             return Err(Error::Invalid("duplicate tool pin id in catalog".into()));
+         }


─── src/catalog.rs:175-175 ───
[security · low] `toolchain` is interpolated raw into a quoted TOML string. All current callers
happen to validate it as an exact semver (config validation, `decode_state`, channel parse), so no
injection exists today — but this shared `pub(crate)` boundary enforces nothing itself, and one
future caller skipping validation would corrupt/inject `rust-toolchain.toml`. One guard here (same
semver check the callers already use) is a smaller, root-cause fix than relying on every call site.
`policy` is fine: it is validated TOML inserted verbatim as file content by design.

+         if !semver::Version::parse(toolchain).is_ok_and(|v| v.pre.is_empty() && v.build.is_empty()) {
+             return Err(Error::Invalid("toolchain must be an exact stable x.y.z version".into()));
+         }
          files.insert("rust-toolchain.toml".into(), format!("[toolchain]\nchannel = \"{}\"\nprofile = \"minimal\"\ncomponents = [\"clippy\", \"rustfmt\"]\n", toolchain));


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



─── src/preview.rs:83-83 ───
[bug · low] Unified-diff convention deviations for empty sides: when `after` is empty (a deletion),
the header stays `+++ b/{path}`, so patch readers (`git apply`, `patch`) truncate the file to empty
instead of deleting it — the convention is `+++ /dev/null`. Likewise, `before == None` with empty
`after` emits an empty `@@ -0,0 +0,0 @@` hunk that `git apply` rejects (git emits header-only "new
file mode" for empty files). Not reachable from today's `inspect` output (proposed content is always
non-empty TOML), but the deletion case is explicitly exercised by
`complete_hunks_preserve_empty_crlf_unicode_and_final_newline` and `unified_diff` is documented for
patch readers, so a future delete disposition would silently render as a truncation. Mirror the
old-side handling: use `/dev/null` for the new side when `after.is_empty()` (and update the test
expectation).

      let old_name = before.map_or_else(|| "/dev/null".into(), |_| format!("a/{path}"));
+     let new_name = if after.is_empty() { "/dev/null".into() } else { format!("b/{path}") };


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



─── src/schema_bounds.rs:38-38 ───
[performance · low] Unnecessary deep clone (and `unwrap()`): `visit` only mutates contents and never
replaces the top-level `Value::Object` variant, so the whole schema map is deep-copied just to move
it back into place. Destructure the value instead — this moves the map with zero copying and removes
the `unwrap()` entirely.

-         *object = value.as_object().unwrap().clone();
+         if let Value::Object(map) = value {
+             *object = map;
+         }


─── src/schema_bounds.rs:16-19 ───
[bug · low] `as_u64()` returns `None` both when `maximum` is absent and when it is present but not a
u64 (e.g. a fractional `maximum: 10.5` or a negative value from a custom schema override).
`is_none_or` then treats that stricter bound as missing and overwrites it with
`u32::MAX`/`u64::MAX`, loosening validation instead of only tightening it. Distinguish "absent" from
"present but not a u64" and compare numerically, leaving uninterpretable bounds untouched.

-                     && object
-                         .get("maximum")
-                         .and_then(Value::as_u64)
-                         .is_none_or(|old| old > maximum)
+                     && object.get("maximum").map_or(true, |old| {
+                         old.as_f64().is_some_and(|old| old > maximum as f64)
+                     })


─── src/schema_bounds.rs:23-25 ───
[bug · low] The traversal recurses into every object value, including non-schema annotation payloads
(`default`, `examples`, `const`, `enum`). A user-supplied default/example object that happens to
contain `"format": "uint32"` (or `"uint64"`) gets a spurious `maximum` key injected, silently
corrupting the annotation data. Skip these data-bearing keywords when recursing.

-                 for child in object.values_mut() {
+                 for (key, child) in object.iter_mut() {
+                     if !matches!(key.as_str(), "default" | "examples" | "const" | "enum") {
-                     visit(child);
+                         visit(child);
+                     }
                  }


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


─── src/plan.rs:120-120 ───
[maintainability · medium] `toolchain_change` already probed `rust-toolchain` existence, but it
collapses "legacy file present" and "content differs" into the same `"conflict"` string, forcing
this guard to re-probe the filesystem (a third probe happens again in the `input_preimages` loop).
Besides the duplicated I/O, the two probes can disagree under concurrent mutation: if the legacy
file is removed between them, a legacy-caused conflict whose base hash matches gets promoted to
`"update"`. Return the legacy flag from `toolchain_change` (e.g. `(Change, bool)`) or distinguish
the conflict cause, and reuse it here instead of re-checking.



─── src/plan.rs:124-124 ───
[maintainability · low] Unnecessary allocation to satisfy the comparison: `base.sha256.clone()`
builds a `String` just to compare against an `Option<String>`. Compare by reference instead.

-             .is_some_and(|base| Some(base.sha256.clone()) == change.before_sha256)
+             .is_some_and(|base| change.before_sha256.as_ref() == Some(&base.sha256))


─── src/plan.rs:45-45 ───
[maintainability · medium] `disposition` is a free-form `String` compared against magic literals
(`"conflict"`, `"update"`, `"unchanged"`, `"create"`) here and across apply/bootstrap, and the
generated schema enforces no enum — a typo silently changes plan semantics (e.g. the
conflict-to-update promotion or the `toolchain-conflict` finding would just stop firing). Model it
as an enum (`Create`/`Unchanged`/`Update`/`Conflict`) with `#[serde(rename_all = "lowercase")]` so
the wire format stays identical while the compiler and JSON schema reject invalid states. The same
applies to `mode`, `state`, and `capability_state` on `Plan`.



─── src/plan.rs:221-221 ───
[maintainability · low] `inspect` is a read-only preview, but a size overflow is reported as
`Error::Transaction`, surfacing to CLI consumers as "transaction blocked" (code `transaction`) when
no transaction exists — misleading diagnostics. `Error::Invalid` fits better here. Also, `1_048_576`
is now hard-coded in `read_small`, `Plan::digest`, and here; a shared `pub(crate) const
MAX_INPUT_BYTES` would keep the bound consistent.



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


─── src/trust/capability.rs:245-245 ───
[maintainability · low] Adapter de-duplication keys on `format!("{:?}", a.id)`: it allocates a
String per adapter and makes identity depend on Debug output stability (a future custom Debug impl
could silently break dedup). `AdapterId` is a plain Copy enum — add `PartialOrd, Ord` to its derive
and use `BTreeSet<AdapterId>` directly; no allocation, typed identity.

-             require(ids.insert(format!("{:?}", a.id)), "duplicate-adapter")?;
+             require(ids.insert(a.id), "duplicate-adapter")?;


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


─── src/trust/policy.rs:40-40 ───
[documentation · low] `workflow.repository` is deliberately not tied to `self.repository`
(reusable-workflow signers live in other repositories; `ExpectedAttestation` binds the *caller*
workflow to the source repo separately), while `sources` and `historical` entries are explicitly
pinned to `self.repository`. That asymmetry makes this look like a missing check to a future reader
or hardening pass. Document the intent here so it is not "fixed" by wrongly pinning signer repos —
or, if cross-repository signers are not actually intended, add `rule.workflow.repository ==
self.repository` to the signer validation instead.

+     /// May name a reusable workflow in another repository; authorized by exact
+     /// repository, path and commit, not by the policy repository.
      pub workflow: WorkflowIdentity,


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


─── src/trust/mod.rs:200-201 ───
[maintainability · low] Byte/char bound mismatch: JSON Schema `minLength`/`maxLength` count Unicode
code points, but `validate_at` enforces `rationale.len() <= 4096` bytes and rejects whitespace-only
text. A schema-valid 4096-character multibyte rationale (up to 16 KiB of UTF-8) or a whitespace-only
string like `" "` passes schema validation yet fails at runtime. The direction is fail-closed, but
the published contract misleads consumers; exact byte bounds and non-whitespace aren't expressible
in JSON Schema, so document the runtime semantics on the field.

      #[schemars(length(min = 1, max = 4096))]
+     #[schemars(description = "Runtime bound is 1-4096 UTF-8 bytes (not code points) and must contain non-whitespace text")]
      pub rationale: String,


─── src/trust/mod.rs:152-155 ───
[maintainability · low] Unbounded version components in the schema: `[0-9]*` accepts arbitrarily
long digit strings, while `stable_version` (semver) rejects any component exceeding `u64`. Cap each
component at 20 digits so the schema stays close to the runtime contract and schema-only consumers
can't be fed unbounded numeric text.

      #[schemars(regex(
-         pattern = "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$(?![\\s\\S])"
+         pattern = "^(0|[1-9][0-9]{0,19})\\.(0|[1-9][0-9]{0,19})\\.(0|[1-9][0-9]{0,19})$(?![\\s\\S])"
      ))]
      pub runtime_version: String,


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


─── src/trust/native.rs:248-248 ───
[maintainability · low] Duplicate-adapter detection keys on `Debug` output rather than the typed id.
This is only correct as long as `AdapterId`'s derived `Debug` stays injective and stable — if a
variant ever gains fields or a manual `Debug` impl is added, distinct adapters could collide (or
duplicates escape detection) silently in this trust-validation path. It also allocates a `String`
per adapter. `AdapterId` already derives `Copy + Eq`; add `PartialOrd, Ord` to its derive list in
`trust/capability.rs` and use a typed `BTreeSet<AdapterId>` here (the same pattern in the v1 catalog
can then be fixed identically).

-             require(ids.insert(format!("{:?}", adapter.id)), "duplicate-adapter")?;
+             require(ids.insert(adapter.id), "duplicate-adapter")?;


─── src/trust/native.rs:176-176 ───
[bug · low] Pin version validation accepts whitespace-padded or otherwise non-canonical strings
(e.g. `" 1.2.3"`, `"1.2.3\n"`). Since `matches()` and the lock-coverage check compare versions by
exact equality against the canonically-formatted lock/evidence versions, such a pin passes
validation but can never apply — a silently inert approval that surfaces later as a confusing
`catalog-lock-pin-mismatch`/coverage failure instead of a clear error here. Requiring
`stable_version` for all pins would be too strict (pins also cover databases/schemas/policies with
non-semver versions), but at minimum reject leading/trailing whitespace so approved pins are never
dead on arrival.

-             identifier(&self.name) && !self.version.trim().is_empty() && self.version.len() <= 100,
+             identifier(&self.name)
+                 && !self.version.is_empty()
+                 && self.version == self.version.trim()
+                 && self.version.len() <= 100,


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


─── src/trust/evidence.rs:485-485 ───
[maintainability · low] Unlike the producer-side lists (which reject duplicate tool names,
capabilities and exception ids), the requirements-side collections are not checked for uniqueness.
Duplicate names in `required.tools` (and duplicate ids in `allowed_exceptions` below) are silently
absorbed by this first-match lookup, leaving later entries unreachable and the policy ambiguous.
Reject duplicate names/ids in `EvidenceRequirements` before matching, mirroring the producer-side
checks.



─── src/trust/evidence.rs:361-361 ───
[performance · low] Unnecessary clone: `p.output` is cloned on every chained step solely to build a
one-element array for comparison. Compare by borrow instead, e.g. `s.inputs.as_slice() ==
std::slice::from_ref(&p.output)` (or `matches!(s.inputs.as_slice(), [x] if *x == p.output)`), which
avoids allocating a `ByteIdentity` copy.

-                     s.inputs == [p.output.clone()] && p.finished_at <= s.started_at,
+                     s.inputs.as_slice() == std::slice::from_ref(&p.output)
+                         && p.finished_at <= s.started_at,


─── src/trust/evidence.rs:377-378 ───
[performance · low] The expected asset name is rebuilt for every inventory asset inside the `find`
closure (a `Selection::key()` allocation plus a `format!` per iteration). Hoist it into a `let
expected_name = format!("{}{}", self.selection.key(), ...);` before the search and compare `a.name
== expected_name` in the closure.



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



─── src/trust/publication.rs:254-261 ───
[bug · high] The identity comparison omits the remote draft byte identity: two
DraftBytesVerified/Approved receipts for the same release_id with different `draft_download_receipt`
digests pass, so a retry can resume against replaced draft bytes — contradicting the documented
"remote draft identity" / "Never authorizes replacement" contract. Compare `draft_download_receipt`
(None == None keeps OwnedDraft/Uploaded retries working). Also decide whether
`approval`/`immutable_setting` substitution between Approved retries must be rejected or explicitly
documented as renewable; as written, an Approved draft can resume with swapped approval or setting
evidence.

          require(
              self.inputs == next.inputs
                  && self.tag == next.tag
                  && self.release_id == next.release_id
                  && self.inventory == next.inventory
-                 && self.policy == next.policy,
+                 && self.policy == next.policy
+                 && self.draft_download_receipt == next.draft_download_receipt,
              "conflicting-retry-identity",
          )


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


─── src/upgrade/rollback.rs:63-64 ───
[bug · low] Deletion of an empty file yields an empty `unified_diff`. When `w.before == Some("")`
and `w.after == None` (rolling back a created empty file), `bootstrap::plan::diff` returns `""` via
its `before == Some(after)` short-circuit, so this `replacen` is a no-op and the packet shows no
diff for a real deletion — asymmetric with the forward creation diff, which does emit headers.
Synthesize the deletion header when the diff comes back empty so the reviewed image stays faithful.

                      if w.after.is_none() {
+                         if diff.is_empty() {
+                             format!("--- a/{}\n+++ /dev/null\n@@ -0,0 +0,0 @@\n", w.path)
+                         } else {
-                         diff.replacen(&format!("+++ b/{}\n", w.path), "+++ /dev/null\n", 1)
+                             diff.replacen(&format!("+++ b/{}\n", w.path), "+++ /dev/null\n", 1)
+                         }


─── src/verification/apple.rs:502-503 ───
[bug · medium] These in-process `check_validity` calls run with `kSecCSAllowNetworkAccess` (bit 16)
and have no deadline. Only the `codesign` subprocess is timeout-bounded (60s via `run_process`); the
Security.framework validation here can block on network ticket/cert lookup indefinitely. The
20-minute stage budget (`started.elapsed() <= ...`) is checked only *after* `native()` returns, so a
stalled network validation hangs the verifier rather than being cut off. Consider enforcing a
deadline around the whole native stage (e.g. run it on a bounded worker / watchdog) so network
stalls fail closed within the budget.



─── src/verification/apple.rs:103-103 ───
[performance · low] `archive` (up to ~1 GiB) is unused after `unpack` but, being a `Vec` with a
destructor, stays allocated until the end of the loop body — i.e. throughout the expensive
`native()` call, which itself re-reads up to 1 GiB from disk several times. That needlessly inflates
peak memory (archive + extracted binary + transient re-read buffers). Drop it right after
extraction.

              let binary = unpack(&archive, binary_name)?;
+             drop(archive);


─── src/verification/apple.rs:178-183 ───
[maintainability · low] This is byte-for-byte identical to the existing `ByteIdentity::from_bytes`
(src/trust/mod.rs), which is already `pub` and in scope via the `ByteIdentity` import. Reuse it
(`ByteIdentity::from_bytes(bytes)`) and delete this helper instead of maintaining a second copy of
the same digest+size construction.



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



─── src/verification/context.rs:441-442 ───
[security · low] macOS signing/notarization gating is hardcoded to one exact triple.
`config::TARGETS` is the authority for supported targets; if another Darwin triple (e.g.
`x86_64-apple-darwin`) is ever added there, this comparison silently classifies it as non-mac and
bypasses apple_team, Sign/Notarize steps, and AppleSigning coverage — a fail-open security gate.
Match the platform suffix (or use a shared `is_macos_target` helper next to `TARGETS`) so new Apple
targets fail closed.

-             let mac =
-                 selection.profile != Profile::Library && selection.target == "aarch64-apple-darwin";
+             let mac = selection.profile != Profile::Library
+                 && selection.target.ends_with("-apple-darwin");


─── src/verification/context.rs:158-159 ───
[maintainability · low] Doc says "rejecting adapters with no pins", but the Legacy branch's
`pins.iter().all(...)` is vacuously true for a pin-less adapter, unlike the NativeV2 branch's
explicit `!pins.is_empty()` guard. This is only safe today because `Catalog::validate`
(capability.rs) separately rejects empty pins and `open_version` requires the baseline adapters — an
invariant enforced in another module. Mirror the explicit non-empty guard here so this security
check doesn't silently degrade if that distant validation changes.

                      .all(|adapter| {
-                         adapter.pins.iter().all(|pin| {
+                         !adapter.pins.is_empty() && adapter.pins.iter().all(|pin| {


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



─── src/upgrade/state.rs:42-50 ───
[maintainability · low] This single gate collapses six distinct invariant failures (schema version,
runtime version, authority identity, plan/config digest format, managed-count bound, toolchain
validity) into one opaque message. Since the design deliberately offers no fallback, an operator
hitting this error has no way to tell which check failed or how to remediate. Consider splitting the
checks (or building a reason string) so each failure reports the specific violated invariant while
still returning the same hard `Error::Transaction` with no fallback.



─── src/upgrade/state.rs:91-91 ───
[performance · low] `next` only consumes `source.bases`, but this deep-clones the entire preimage
map (the raw text of up to three ownership state files) just to satisfy `source_bytes`'s by-value
parameter, which moves the map into `Ownership.preimages` that is then dropped here. `next_text`
runs this on every commit/rollback/recover call. A borrowed variant of `source_bytes` (or a
bases-only extraction) would avoid copying the state texts entirely; the fresh-read caller in
`plan::source` can keep the by-value path.



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


─── src/verification/graph.rs:144-147 ───
[maintainability · low] `sbom-string-field-missing` is reported both when the key is absent and when
it is present but not a string (e.g. a numeric `version`), which misdirects producer debugging.
Distinguish the two cases with a second opaque code — this preserves the no-echo policy while making
the diagnostic actionable.

-     value
-         .get(key)
-         .and_then(Value::as_str)
-         .ok_or_else(|| crate::Error::Invalid("sbom-string-field-missing".into()))
+     match value.get(key) {
+         None => Err(crate::Error::Invalid("sbom-string-field-missing".into())),
+         Some(v) => v
+             .as_str()
+             .ok_or_else(|| crate::Error::Invalid("sbom-string-field-invalid".into())),
+     }


─── src/upgrade/plan.rs:681-681 ───
[security · medium] Offline reconstruction gap: `inspect_bytes` rejects `intent.policy.license_file`
when it overlaps bootstrap targets, state/journal, or control files (`armorer.toml`,
`.armorer/apply.lock`, `rust-toolchain`, ...), but `validate()` only calls `validate_shape()`, which
does not cover that blocklist (verified in src/config.rs). A hand-built packet with a
self-consistent digest and e.g. `license_file = "armorer.lock"` or a state path passes offline
validation, and the rollback consumer relies on offline validation without freshness re-inspection.
Extract the overlap check into a shared helper and call it from both `inspect_bytes` and
`validate()` so the invariant is fixed once at the shared boundary.



─── src/upgrade/plan.rs:680-680 ───
[bug · low] Canonicality gap: `inspect_bytes` sorts `import_files` before storing them, but
`validate()` only checks uniqueness/membership via `imports()`, not ordering. A semantically
identical packet with unsorted `import_files` still validates (digest covers the unsorted order),
yielding two distinct valid approval digests for the same approval — and `validate_fresh` will then
always fail because re-inspection sorts them. Enforce the sorted invariant here.

      imports(&plan.import_files)?;
+     if !plan.import_files.is_sorted() {
+         return Err(Error::Transaction("import files must be canonically sorted"));
+     }


─── src/upgrade/plan.rs:184-184 ───
[bug · low] Panic-prone indexing in a library path: `files["rust-toolchain.toml"]` panics if the key
is absent. Today `catalog::bootstrap_files` always inserts it, so this is unreachable, but the
safety of this line rests on an undocumented cross-module invariant; a fallible lookup keeps the
failure a typed `false`/`Error` instead of a panic if the generated set ever changes.

-                             .is_ok_and(|files| files["rust-toolchain.toml"] == *base)
+                             .is_ok_and(|files| files.get("rust-toolchain.toml") == Some(base))


─── src/upgrade/plan.rs:712-712 ───
[bug · low] `zip` silently truncates to the shorter iterator: `plan.changes.len()` is checked
against `bootstrap::TARGETS.len()`, but the generated map's size is not. If `bootstrap_files` ever
returns fewer entries than TARGETS, trailing changes skip candidate/base/diff reconstruction
entirely. Currently unreachable (fixed 5-entry set), but one defensive length check removes the
fragile coupling.

+     if generated.len() != plan.changes.len() {
+         return Err(Error::Transaction("generated target set differs from fixed contract"));
+     }
      for (c, (path, candidate)) in plan.changes.iter().zip(generated) {


─── src/upgrade/plan.rs:803-803 ───
[bug · low] Misleading freshness failure on calendar rollover: `inspect_bytes` recomputes
`policy_review_day = ci_policy::today()`, so once the UTC day advances, `fresh.digest()` differs
from the saved plan even when every on-disk input, base, and preimage is byte-identical — yet the
error claims inputs/bases/preimages changed. This also contradicts `validate()`'s deliberate
historical-day replay support. Either compare freshness at `plan.policy_review_day` (threading it
into the re-inspection) or make the error text acknowledge review-day expiry as a distinct cause.



─── src/verification/bundle.rs:39-42 ───
[documentation · low] The documented contract ("exact unverified bare transport bytes") does not
hold for the `load_download` path: there, `bytes` is the re-serialized `serde_json::to_vec(&value)`
normalization, not the bytes actually received on the line (whitespace/key order are rewritten,
wrapper metadata stripped). Current downstream consumers are self-consistent (snapshots are written
from these same retained bytes), so nothing breaks today — but in a security-sensitive crate, a
future caller relying on this doc for byte-identity against the received transport would be misled.
Either retain the original line bytes in `load_download`, or reword the doc to state that download
records are stored in canonicalized form.



─── src/verification/bundle.rs:71-73 ───
[maintainability · low] A base64 decode failure is mapped to `Error::Json`, whose display is "JSON
serialization failed" — an actively misleading diagnostic for an encoding fault in the DSSE payload
(it is neither JSON nor serialization). The crate's opaque fixed-code convention is fine, but this
case deserves its own static code, e.g. via the same mechanism `require` uses.

      let payload = STANDARD
          .decode(&bundle.dsse_envelope.payload)
-         .map_err(|_| Error::Json)?;
+         .map_err(|_| Error::Transaction("attestation-payload-encoding"))?;


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


─── src/upgrade/transaction.rs:553-554 ───
[maintainability · low] `.is_ok()` silently collapses genuine configuration drift, transient I/O
failures (e.g. momentarily unreadable config), and policy parse/expiry errors into `configured:
false`, discarding the error entirely. A committed recovery on a healthy repo with a transient read
failure is reported identically to real drift, which can mislead operators. Consider propagating
unexpected I/O errors (or at least distinguishing drift from failure/documenting the conservative
default) instead of swallowing all error classes into a boolean.



─── src/upgrade/transaction.rs:143-153 ───
[performance · low] `proposed_content` is cloned even when `before_content == after` and the write
is skipped. Borrow first and clone only when actually pushing the write, avoiding a potentially
large content copy for every unchanged target.

          let after = c
              .proposed_content
-             .clone()
+             .as_ref()
              .ok_or(Error::Transaction("upgrade after image missing"))?;
-         if c.before_content.as_deref() != Some(&after) {
+         if c.before_content.as_deref() != Some(after.as_str()) {
              writes.push(Write {
                  path: c.path.clone(),
                  before: c.before_content.clone(),
-                 after: Some(after),
+                 after: Some(after.clone()),
              });
          }


─── src/verification/bundle.rs:73-73 ───
[maintainability · low] Correction to my previous suggestion on this line: use the same mechanism as
`trust::require`, i.e. `Error::Invalid` with a fixed static code (`Transaction` is reserved for
transaction-blocking failures). The core finding stands: mapping a base64 decode failure to
`Error::Json` ("JSON serialization failed") is a misleading diagnostic for a payload-encoding fault.

-         .map_err(|_| Error::Json)?;
+         .map_err(|_| Error::Invalid("attestation-payload-encoding".into()))?;


─── src/verification/historical.rs:75-75 ───
[bug · low] An empty `historical.assets` map passes every gate here: `policy.validate()`'s
`assets.values().all(...)` is vacuously true, `total == 0 <= MAX_TOTAL`, and `compare_directory`
succeeds against an empty directory. `verify` then returns a successful `HistoricalByteMatch` that
matched zero bytes. Given this module's fail-closed design, require a nonempty approved asset set so
an empty allowlist can never produce a vacuous "byte match" result.

-             historical.assets.len() <= MAX_FILES
+             !historical.assets.is_empty()
+                 && historical.assets.len() <= MAX_FILES


─── src/verification/io.rs:34-36 ───
[security · medium] The dev/ino identity binding only exists on Unix. On other platforms `regular()`
silently degrades to a type+size check, so a same-length file (or symlink) swapped between
`symlink_metadata` and `File::open` passes as "unchanged". The rest of this module fails closed on
non-Unix (`readonly()`, `filesystem::private_file`); do the same here (or add an equivalent identity
check) instead of weakening the guarantee per-platform. Also note that even on Unix, an in-place
mutation before open that preserves inode and size is undetectable — document that content is
trusted as of the opened handle, since all later reads are pinned to it.



─── src/verification/io.rs:160-160 ───
[bug · high] The destination is created before streaming, and any mid-stream failure (read/write
error, source growing past `max`, `sync_all` failure) leaves a truncated file behind. Because
`private_file` uses `create_new`, every later snapshot attempt at this path then fails with
`AlreadyExists`, and the partial file can be consumed downstream as if it were a complete snapshot
(its recorded identity never exists, but the bytes are there). Remove the destination on any error —
e.g. run the copy loop in a closure returning `Result`, and on `Err` do `let _ =
fs::remove_file(destination);` before propagating — or stream into a temp leaf and publish
atomically.



─── src/verification/io.rs:224-227 ───
[security · high] `fs::set_permissions` is path-based: it follows symlinks and re-resolves the path
at call time. If the snapshot path has been replaced with a symlink (or swapped) since creation,
this chmods an unrelated file and can grant the execute bit (0o500) outside the private staging
directory. Open the leaf first — rejecting symlinks (ideally `O_NOFOLLOW` to close the open race
itself) — and use handle-based `File::set_permissions`, the pattern `write_readonly` already uses in
this file.



─── src/verification/io.rs:151-154 ───
[maintainability · medium] Every distinct failure mode — duplicate keys (a security-relevant
ambiguity/tampering signal), node/depth budget exhaustion, trailing data, and typed conversion —
collapses into `Error::Json`, whose Display is "JSON serialization failed" (misleading: this is
parsing). Callers and logs cannot distinguish an attack signal from a malformed document. Follow the
`require()` pattern used everywhere else in this file: map each stage to a distinct stable code,
e.g. `Error::Invalid("verification-json-structure".into())` / `"verification-json-trailing"` /
`"verification-json-shape"`, without echoing untrusted input.



─── src/verification/io.rs:184-184 ───
[maintainability · low] This is a byte-for-byte duplicate of the size-gated hashing loop in
`snapshot()` (same 64 KiB buffer, `take(max + 1)`, per-block `require(size <= max)`,
finalize/validate). Two copies of a security check invites divergence: a future fix to the size gate
or hashing must be applied twice. Extract one private streaming helper taking an optional writer
(`Option<&mut File>`) and call it from both functions.



─── src/verification/publication/mod.rs:569-572 ───
[bug · high] Crash/failure windows permanently wedge staging. The journal persists `Phase::Creating`
before `backend.create(...)` and `Phase::Uploading` before `backend.upload(...)`, but this retry
gate only accepts `OwnedDraft | Uploaded | DraftVerified`. If the process dies (or the call errors
ambiguously) after the remote write succeeded but before the follow-up `save`, the on-disk phase is
`Creating`/`Uploading` and every retry fails here. Worse, after a lost `create` response the tag is
now occupied (so the fresh-start path also fails), and after a lost `upload` response the remote
asset is absent from `journal.uploaded`, so `validate_recorded_assets` would reject it even if the
phase were accepted. Since the design forbids overwrite/delete, there is no reconciliation path: add
recovery for `Creating` (adopt the owned draft by marker via `releases_for_tag`) and `Uploading`
(re-list and reconcile exact-plan asset IDs into `journal.uploaded`) instead of hard-failing.



─── src/verification/publication/mod.rs:721-724 ───
[bug · high] A definitively failed publish wedges the journal with no rollback. `Phase::Publishing`
is persisted before `backend.publish(id)?`; if the write fails without taking effect (e.g.,
403/permission error), the error propagates and the journal stays `Publishing`. `publish()` only
accepts `DraftVerified`, and `recover_published()` -> `finish_published()` -> `verify_bytes(id,
false)` requires `draft == false`, which fails while the release is still a draft. No code path ever
transitions back to `DraftVerified`, so a single failed publish attempt permanently blocks both
publishing and recovery for this plan. On a confirmed non-mutation failure, restore
`Phase::DraftVerified` (or let recovery detect the still-draft state and reset) while keeping the
never-repeat-write guarantee for ambiguous failures.



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


─── src/verification/publication/mod.rs:424-425 ───
[maintainability · low] The underlying `PersistError` (which carries the original I/O error and the
temp file handle) is discarded, leaving operators without the actual cause (permissions,
cross-device rename, target-exists on Windows, etc.) when journal persistence fails. Preserve the
cause in the message.

          file.persist(&self.journal)
-             .map_err(|_| Error::Invalid("publication-journal-persist-failed".into()))?;
+             .map_err(|e| Error::Invalid(format!("publication-journal-persist-failed: {e}")))?;


─── src/verification/release.rs:285-285 ───
[bug · low] The final full-rehash loop never samples `deadline(started)`, unlike the snapshot,
bundle, and selection loops. It re-reads and re-hashes up to `MAX_TOTAL` (4 GiB) across up to 8194
files, so on slow storage this loop alone can run far past the 20-minute `DEADLINE` after the last
check, breaking the stated "refuse partial work after the deadline" invariant. Add a per-iteration
deadline check here (it's cheap relative to the hashing).

          for (name, identity) in &identities {
+             deadline(started)?;


─── src/verification/release.rs:161-164 ───
[performance · low] O(n²) name lookups at maximum release scale: this per-bundle linear `.find` over
`inventory.assets`, and the per-identity `.find` for role caps in the final recheck loop, each scan
up to ~8192 assets — up to ~67M string comparisons per loop, eating into the fixed 20-minute
verification budget. Since names are unique after `inventory.validate_against`, build a name index
once (e.g. `let by_name: BTreeMap<&str, &Asset> = inventory.assets.iter().map(|a| (a.name.as_str(),
a)).collect();`) and use it for both the subject lookup here and the role-cap lookups in the final
loop (and optionally `asset_bytes`).



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



─── src/verification/runtime.rs:48-48 ───
[bug · low] The 20-minute stage budget is only polled inside `decode` (and once at its end). This
`io::snapshot` call copies up to `MAX_DISTRIBUTION` (~384 MiB) from the untrusted archive path with
no deadline checks, and the later `io::identity` rehash / `sync_all` are likewise uncovered.
`io::regular` rules out FIFOs/symlinks, but a slow or hung filesystem (e.g., network mount) can
still block `open()` indefinitely before the first `deadline(started)` ever runs, so the stage
timeout does not actually bound the whole stage. If the bound is a real requirement, make the copy
loop deadline-aware (pass `started`/a deadline into the snapshot); otherwise document the budget as
best-effort so callers don't rely on it for liveness.



─── src/verification/publication/github.rs:962-965 ───
[security · high] Fail-open serialization gate: a competing deployment whose status list is empty
(HTTP 200 `[]`) gets `latest = Null`, so `active` is false and it is excluded from `active_count`.
Status *fetch failures* are downgraded via `status_unavailable`, and full pages via
`status_truncated`, but the empty-list case slips through both. A freshly created competing
deployment (e.g., created via API before its first status is written) is therefore invisible,
letting `active_count == 1 && bound_active` reach `Enforced` while two runners concurrently hold the
environment — contradicting the `terminal_deployment_status` doc ("anything else is still active").
Fail closed here: count a deployment with no observed status as active. Keep the `bound_active`
check below strict so enforcement is never upgraded by a missing status.

-         let latest = latest_status_map.get(&id).cloned().unwrap_or(Value::Null);
-         let active = latest["state"]
-             .as_str()
-             .is_some_and(|state| !terminal_deployment_status(state));
+         let latest = latest_status_map.get(&id);
+         // Fail closed: a deployment with no observed status may still hold the environment.
+         let active = latest
+             .and_then(|status| status["state"].as_str())
+             .map_or(true, |state| !terminal_deployment_status(state));


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


─── tests/apply.rs:202-203 ───
[test · low] Two robustness gaps: (1) Contention is simulated with two fds in the *same* process.
That works for Linux `flock`, but on filesystems that emulate it with per-process POSIX locks (e.g.
some NFS mounts) the second lock succeeds and this test fails/misbehaves; holding the lock from a
spawned child process would exercise real cross-process exclusion. (2) Matching the prose substring
"another Armorer" couples the test to error wording — a reworded message breaks the test even though
behavior is correct. Prefer a typed busy/lock-contention error variant (or at least a stable error
code) over string matching.



─── tests/apply.rs:236-237 ───
[test · low] Symlink-escape coverage is limited to final-component symlinks (`.armorer` and
`rust-toolchain.toml`). A regression that follows symlinks in *intermediate* path components (e.g. a
symlinked parent of a managed file) or that opens a symlinked approved-plan file would not be caught
here. Add cases with a symlinked intermediate directory and a symlinked plan file passed to
`load_plan`/the CLI, asserting the outside target is neither read-adopted nor written.



─── tests/apply.rs:279-279 ───
[test · medium] Both forged-state scenarios assert only `is_err()`, so an unrelated early failure
(digest validation, input drift, etc.) would mask a regression in the intent/ownership
re-verification this test is meant to protect — e.g. if the `same_inputs` re-check were removed but
the fresh-digest check still rejected the forged plan, this test would keep passing. Pin the
rejection reason (assert the error message/variant: "approved inputs changed" for the forged plan,
"committed plan receipt conflicts with managed files" for the emptied `managed` map), and after each
rejected apply assert `state.json` bytes are unchanged so a failed apply can't silently mutate
ownership state.



─── tests/apply.rs:295-298 ───
[test · low] The injection only produces a genuine duplicate key because the fixture's `BTreeMap`
happens to serialize `"LICENSE"` first with a non-null value; nothing asserts the replacement
occurred or that a doubled key exists. If the key set or serialization changes (empty map →
trailing-comma invalid JSON; absent `LICENSE` → merely an extra field), `load_plan` would reject for
parse/unknown-field reasons and this test would silently stop exercising duplicate-key detection.
Pin the injected duplicate, e.g. `assert!(raw.contains("\"LICENSE\":null,\"LICENSE\":"));` before
writing the file.

      let raw = serde_json::to_string(&plan).unwrap().replace(
          "\"input_preimages\":{",
          "\"input_preimages\":{\"LICENSE\":null,",
      );
+     assert!(raw.contains("\"LICENSE\":null,\"LICENSE\":"));


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


─── tests/bootstrap.rs:159-159 ───
[maintainability · low] Security assertions are positionally coupled to catalog/TARGETS ordering:
`changes[index]` here, `changes[1]` in the forgery test, `changes[4]` in the replay test, plus
hardcoded counts (5 files, 18 tools). A catalog reorder silently retargets the
conflict/ownership/forgery checks at different files while the tests still pass. Select changes by
path (as `new_bootstrap_defaults_to_the_v2_successor_authority` already does with `.find(|c| c.path
== ...)`) or assert the expected path at each index so a reorder fails loudly.



─── tests/bootstrap.rs:135-135 ───
[test · low] mtime equality does not prove no rewrite: on filesystems with coarse timestamp
granularity (1–2 s ticks, e.g. FAT/some network FS), a rewrite within the same tick leaves
`modified()` unchanged, so this assertion can pass even if the second `apply` rewrote the files. The
byte snapshot above is the only real evidence here. For a stronger idempotence proof, compare file
identity (inode on unix) or mark the targets read-only so any rewrite attempt errors.



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



─── src/verification/publication/tests.rs:83-84 ───
[test · low] The fake unconditionally replaces any existing remote release on create, whereas the
real provider rejects a duplicate tag (422). Today only the counter assertions guard against
accidental replacement; a controller bug that replaces a foreign release and then errors would
silently succeed at the provider layer in this double. Consider failing when a release with the same
tag already exists (e.g. `require(self.remote.is_none(), "test-duplicate-tag")?`) to match the
provider contract.



─── src/verification/publication/tests.rs:311-312 ───
[test · low] The "foreign" release is constructed through `Fake::create`, which assigns author 2 —
an ID inside the policy's `allowed_publisher_ids`. Rejection here therefore only proves
pre-existing-without-journal handling; genuinely foreign authorship is untested at the controller
level even though remote validation checks the author. Add a variant that sets `foreign.author` to
an unlisted ID (e.g. 99) to exercise the author-mismatch rejection path.



─── tests/ci_policy.rs:27-28 ───
[test · low] When a case in this loop fails, neither `unwrap_err()` nor the bare `assert!`
identifies which mutated policy caused the failure, making CI failures hard to diagnose. Attach the
input to the failure messages (e.g. `expect_err(&format!(...))` and an assertion message). The same
applies to the `bad`-input loops in the other tests, which currently use a bare
`assert!(parse_at(bad.as_bytes(), today).is_err());` with no context.

-         let error = parse_at(invalid.as_bytes(), today).unwrap_err();
-         assert!(!error.to_string().contains("DO-NOT-ECHO"));
+         let error = parse_at(invalid.as_bytes(), today)
+             .expect_err(&format!("policy should be rejected: {invalid}"));
+         assert!(
+             !error.to_string().contains("DO-NOT-ECHO"),
+             "error echoed untrusted input: {error}"
+         );


─── tests/ci_policy.rs:44-44 ───
[test · low] This case is intended to verify rejection of non-zero-padded dates, but `2026-2-03`
(Feb 3, 2026) is also before `today` (2026-10-02), so it would be rejected as expired even if format
validation were entirely missing — the assertion does not isolate the intended check. Use an
unpadded date that is otherwise valid and inside the accepted expiry window (the test accepts
"2026-12-31"), e.g. "2026-12-3", so only the missing zero-padding can cause rejection.

-         "2026-2-03",
+         "2026-12-3",


─── tests/ci_policy.rs:102-104 ───
[test · low] This loop injects `custom_hook = "DO-NOT-ECHO"` into `[bans]` but, unlike the first
test, never asserts that the resulting error message avoids echoing the untrusted input — the
non-echo guarantee is only checked for the top-level unknown-key path. Add the same
`!error.to_string().contains("DO-NOT-ECHO")` assertion here, and include the failing input in the
message so a failure identifies which of the five mutations broke.

          good.replace("[bans]", "[bans]\ncustom_hook = \"DO-NOT-ECHO\""),
      ] {
-         assert!(parse_at(bad.as_bytes(), today).is_err());
+         let error = parse_at(bad.as_bytes(), today)
+             .expect_err(&format!("policy should be rejected: {bad}"));
+         assert!(!error.to_string().contains("DO-NOT-ECHO"), "{error}");


─── tests/catalog.rs:105-105 ───
[test · medium] This hardening test only exercises the v1 catalog (`reviewed()` /
`WORKFLOW_COMMIT`). The v2 catalog (`reviewed_v2()`, pinned to `WORKFLOW_COMMIT_V2`) is what the CLI
test asserts against, yet its rendered callers never go through these permission/forbidden-key
checks — a regression in the v2 pin or a v2-specific renderer change would only be caught by the
byte-comparison fixture test, which can be "fixed" by updating the committed fixtures in the same
change. Run the same assertions for both catalogs, parameterized by their expected commit.



─── tests/catalog.rs:107-107 ───
[test · low] The test name promises "only reviewed full pins", but this is a presence check: an
extra unpinned reference (e.g. a second `uses: actions/checkout@main` line) would still pass, since
the pinned line remains and none of the forbidden substrings match. Also, the forbidden list misses
`permissions: write-all`, which grants write access without matching `"contents: write"`. Assert
exactly one `uses:` occurrence per workflow and add `"write-all"` to the forbidden list.

          assert!(value.contains(&format!("@{WORKFLOW_COMMIT}\n")));
+         assert_eq!(value.matches("uses:").count(), 1);


─── tests/publication_cli.rs:2-2 ───
[bug · medium] This unconditional `std::os::unix` import breaks compilation of the whole test crate
on non-Unix targets. Sibling tests gate this import (and Unix-only tests) with `#[cfg(unix)]` (see
`tests/integrated_cli.rs`). Since this test also writes `/bin/sh` scripts and sets Unix permission
bits, add `#[cfg(unix)]` to the test function as well.

- use std::{fs, os::unix::fs::PermissionsExt, process::Command};
+ use std::{fs, process::Command};
+ #[cfg(unix)]
+ use std::os::unix::fs::PermissionsExt;


─── tests/publication_cli.rs:16-16 ───
[test · low] Hardcoding `/usr/bin/touch` weakens the execution detector on Unix systems where that
path doesn't exist (NixOS, some minimal images): the marker can never appear, so
`assert!(!marker.exists())` passes vacuously even if a regression runs the tool before context
authentication (the context would still fail afterwards with the expected message). Use
PATH-resolved `touch` like the sibling tests (`tests/release_context.rs`, `tests/historical.rs`).

-             format!("#!/bin/sh\n/usr/bin/touch '{}'\n", marker.display()),
+             format!("#!/bin/sh\ntouch '{}'\n", marker.display()),


─── tests/publication_cli.rs:60-60 ───
[test · low] A bare `unwrap()` here panics with only a serde error when the CLI emits non-JSON
output, hiding which of the five actions failed and what the CLI actually printed. Include the
action and captured output in the failure message (the `cli()` helper in `tests/integrated_cli.rs`
does this) so regressions in this security-ordering guarantee are diagnosable.

-         let diagnostic: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
+         let diagnostic: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
+             panic!(
+                 "{action}: invalid diagnostic JSON ({error}); stdout: {}; stderr: {}",
+                 String::from_utf8_lossy(&output.stdout),
+                 String::from_utf8_lossy(&output.stderr)
+             )
+         });


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



─── tests/integrated_cli.rs:50-56 ───
[test · low] The `cli` helper makes contract failures hard to diagnose: (1) the status assertion
prints only stderr, so when the child is killed by a signal (`status.code()` is `None`, stderr often
empty) the failure message is blank; (2) `serde_json::from_slice(...).unwrap()` panics with only a
parse error and never shows the raw stdout that failed to parse. Include `output.status` (its
`Display` reports signals) and the raw stdout in both failure paths.

      assert_eq!(
          output.status.code(),
          Some(status),
-         "{}",
-         String::from_utf8_lossy(&output.stderr)
+         "status: {}\nstderr: {}\nstdout: {}",
+         output.status,
+         String::from_utf8_lossy(&output.stderr),
+         String::from_utf8_lossy(&output.stdout)
      );
-     serde_json::from_slice(&output.stdout).unwrap()
+     serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
+         panic!(
+             "CLI stdout was not valid JSON ({error}): {}",
+             String::from_utf8_lossy(&output.stdout)
+         )
+     })


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


─── tests/integrated_cli.rs:247-251 ───
[test · low] Pinning the blocked workflow to exactly `changes[1]` couples this test to the plan's
precise ordering. The scenario only requires that the blocked write happens *after* at least one
successful managed write; a benign reordering (e.g., inserting another writable change before the
workflow) breaks the test even though rollback behavior is unchanged — and if these assertions were
ever relaxed, the failure could occur on the very first write, silently losing the partial-apply
rollback coverage. Keep the guarantee that the first change is writable, but locate the blocked
workflow's position dynamically instead of hardcoding index 1.

-     assert_eq!(plan["changes"][0]["path"], ".armorer/ci-policy.toml");
-     assert_eq!(
-         plan["changes"][1]["path"],
-         ".github/workflows/armorer-build.yml"
+     let changes = plan["changes"].as_array().unwrap();
+     assert_eq!(changes[0]["path"], ".armorer/ci-policy.toml");
+     let blocked = changes
+         .iter()
+         .position(|change| change["path"] == ".github/workflows/armorer-build.yml")
+         .expect("plan must contain the managed build workflow");
+     assert!(
+         blocked > 0,
+         "the blocked workflow write must follow a successful managed write"
      );


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


─── tests/integrated_cli.rs:217-220 ───
[test · low] `.ok()` collapses every read error into `None`, conflating "file absent" with "file
exists but is unreadable" (permissions, I/O errors). If `Cargo.lock` was absent in `before` but is
later created unreadable, this assertion passes and masks an unintended side effect. Distinguish
`NotFound` from other errors so real I/O failures fail the test.

-         assert_eq!(
-             fs::read(root.path().join("Cargo.lock")).ok(),
-             before.get("Cargo.lock").cloned()
-         );
+         let lock = match fs::read(root.path().join("Cargo.lock")) {
+             Ok(bytes) => Some(bytes),
+             Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
+             Err(error) => panic!("reading Cargo.lock failed: {error}"),
+         };
+         assert_eq!(lock, before.get("Cargo.lock").cloned());


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


─── tests/preview.rs:294-299 ───
[test · low] This test hard-depends on an external GNU-compatible `patch` being on PATH. If the
binary is missing, `.output().unwrap()` panics with an opaque I/O error, and BSD/macOS `patch`
handles CRLF and "\ No newline at end of file" hunks differently, producing environment failures
rather than a signal about the product's diffs. Probe for GNU patch first (e.g. run `patch
--version` and check for "GNU") and skip with an explicit message when unavailable, or otherwise
document/gate the toolchain requirement.



─── tests/preview.rs:61-62 ───
[test · low] The exactness assertions index `files[0]` / `changes[0]` without asserting the
collection length or selecting the entry by path (this pattern repeats in the conflict, ownership,
legacy-toolchain, and independent-patch tests). `Preview.files` is a `Vec` derived from
`plan.changes`, so if another managed file is ever added or ordering changes, these assertions
silently target the wrong entry — or panic out of bounds — instead of failing with a meaningful
message. Assert the expected length (e.g. `assert_eq!(result.files.len(), 1)`) or look entries up
via `result.files.iter().find(|f| f.path == "rust-toolchain.toml")`.



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



─── tests/cargo_graph_v2.rs:313-313 ───
[test · low] The suite's theme is that the producer cannot choose its own consumer context, yet here
the expected `source_commit` (and `native_target`) are taken verbatim from the producer-supplied
receipt with no independent cross-check — a coherently forged receipt+graph pair passes.
`runtime_commit` is at least cross-checked against `lock.workflows.commit`; give `source_commit` the
same treatment (pin it as a fixture constant like `context()` does, or verify it against another
retained artifact). Also note this interop path is `#[ignore]`d, so it never runs in default CI —
make sure some pipeline actually sets `ARMORER_TEST_GRAPH_RECEIPTS`.



─── tests/cargo_graph_v2.rs:252-252 ───
[maintainability · low] `remove(0)` assumes the root's first dependency edge is the runtime `dep`
edge (true for today's fixture). The same test already resolves nodes and SBOM entries by id/ref —
resolve this edge by `package == "dep"` too, so a fixture reorder can't silently retarget the
scenario. Same pattern applies to `graph.nodes[0].features = actual;` in
`real_cargo_resolution_distinguishes_requested_flags_from_resolved_features`, which should look up
the root node by id like the other feature tests do.

-     let normal = graph.nodes[root].dependencies.remove(0);
+     let dep_edge = graph.nodes[root]
+         .dependencies
+         .iter()
+         .position(|e| e.package == "dep")
+         .unwrap();
+     let normal = graph.nodes[root].dependencies.remove(dep_edge);


─── tests/cargo_graph_v2.rs:128-128 ───
[test · low] The substitution matrix exercises every `Selection` field except `package` and
`feature_set`. Production binds them via whole-struct equality (`self.selection == *selection`), but
that binding is only regression-tested field-by-field here — add the two missing mutations so a
future switch to per-field comparison can't silently drop them.

          ("/selection/toolchain", json!("1.94.0")),
+         ("/selection/package", json!("other")),
+         ("/selection/feature_set", json!("minimal")),


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



─── tests/inspection.rs:603-603 ───
[test · low] Weak assertion: checking only exit code 1 lets any unrelated failure satisfy the
'non-blocking FIFO rejection' guarantee. Capture stdout and assert the specific error code (as the
other CLI tests do) so this proves the FIFO was rejected for the right reason.



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



─── tests/inspection.rs:493-493 ───
[test · low] Same silent no-op risk: `is_ok()` alone passes even if the `[package]` replace fails to
match, leaving anchored-pattern/opaque-metadata coverage untested. Assert the replacement actually
changed the manifest before asserting success.



─── tests/inspection.rs:411-411 ───
[test · low] Environment-dependent: the `cargo-discovery` code here relies on rustup-managed
resolution of `99.99.99` (with auto-install disabled). On a host without rustup, `trusted_rustup`
fails first and returns the same `cargo-discovery` code for an unrelated reason (`rustup-path`), so
the test passes without actually exercising the missing-toolchain path. Consider asserting the error
message identifies the missing toolchain (not `rustup-path`) to make the guarantee meaningful.



─── tests/real_sigstore.rs:263-268 ───
[test · high] This security gate asserts only `is_err()` for every identity-mismatch case (and the
same pattern is used for all other negative checks: the tamper loop, artifact flips, `GithubPrivate`
backend, approval-digest mismatch, wrong root, expired review, verifier substitution, and the mixed
download). Any unrelated failure — e.g. a native `gh` exec/runtime error or an
`OfflineVerifier::open` failure for a different reason — would satisfy the assertion without proving
the intended enforcement path fired, silently weakening this required CI gate. Match the specific
`Error::Invalid(code)` per case, as the first check already does with
`"verification-input-type-or-size"`, and include the actual error in the panic message. At minimum,
assert the error class:

-         assert!(
-             wrong_verifier
+         let err = wrong_verifier
-                 .verify(&artifact, &expected_bytes, &bundle, &wrong)
+             .verify(&artifact, &expected_bytes, &bundle, &wrong)
-                 .is_err(),
-             "{case}"
+             .expect_err(case);
+         assert!(
+             matches!(err, armorer::Error::Invalid(_)),
+             "{case}: unexpected error class: {err:?}"
          );


─── tests/real_sigstore.rs:174-174 ───
[test · low] Replacing the first base64 character with 'A' is not guaranteed to mutate anything: if
the fixture signature already starts with 'A', the bundle stays byte-identical and this "tamper"
case becomes an unmutated positive verification (the test then fails spuriously on any fixture
rotation). Also `&sig[1..]` panics on an empty string. The same pattern is repeated for the
certificate `rawBytes` below. Decode, flip a bit, and re-encode so the mutation is guaranteed
regardless of fixture content:

-     signature["dsseEnvelope"]["signatures"][0]["sig"] = format!("A{}", &sig[1..]).into();
+     let mut sig_bytes = STANDARD.decode(sig).unwrap();
+     sig_bytes[0] ^= 1;
+     signature["dsseEnvelope"]["signatures"][0]["sig"] = STANDARD.encode(sig_bytes).into();


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



─── tests/trust_contracts.rs:587-587 ───
[maintainability · low] On subprocess failure the assertion discards `output.stderr`, so a
schema-generation crash reports only "schema {kind}" with no diagnostic. Include stderr in the
failure message.

-         assert!(output.status.success(), "schema {kind}");
+         assert!(
+             output.status.success(),
+             "schema {kind}: {}",
+             String::from_utf8_lossy(&output.stderr)
+         );


─── tests/release_context.rs:205-208 ───
[test · high] Vacuous on unsupported hosts: `TrustedReleaseContext::open` always fails at the
`qualified_native_validator()?` gate (src/verification/context.rs:323) when the host is unsupported,
and `fixture()` deliberately embeds a bogus validator identity in that case. Since this test only
asserts `is_err()` and is not `cfg_attr`-gated like the other semantic tests, it passes on
unsupported hosts even if every config/lock/catalog/policy substitution check were deleted. Add the
same `#[cfg_attr(..., ignore)]` gate used by the other semantic tests, and/or assert the specific
authority-mismatch error code so an unrelated rejection path cannot satisfy the assertion.



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



─── tests/release_context.rs:47-47 ───
[test · low] `now()` is sampled repeatedly (twice here, once per tool in the `observed_at` map,
again in the expiry/observation mutations) and each sample is signed into the approved digest, so
related timestamps can straddle a second boundary and the fixtures are non-reproducible. Capture one
instant at the top of `fixture()` (`let now = now();`) and derive
`reviewed_at`/`expires_at`/`observed_at` from it; pass the same base into the mutation tests.
Margins are currently wide enough to avoid flakes, but a single clock read removes the risk and
makes digests deterministic per run.



─── tests/release_context.rs:598-599 ───
[test · medium] These slices hardcode the exact layout of `synthetic-runtime.tar` (1024-byte stride,
128-byte members at +512). The container-mutation test's fixed offsets
(148/156/257/512/640/1536/3500) and the `len() == 128` assertion share the same coupling. If the
binary fixture drifts, this panics on slice bounds or — worse — the mutation test flips an
unintended byte and the archive-safety assertions silently test the wrong field. Add an explicit
layout sanity check before slicing (e.g., `assert!(archive.len() >= targets.len() * 1024 + 1024)`
plus a ustar-magic/member-size check), or derive offsets from the tar headers, so drift fails with a
clear diagnostic.



─── tests/release_context.rs:305-305 ───
[maintainability · low] The wildcard arm silently absorbs any index: bumping `0..14` without adding
an arm re-applies the `apple_team` mutation instead of failing, and adding arm `13` without bumping
the range never runs it. Make the last case explicit and end with `_ => unreachable!()` (the v3 test
in this file already uses that pattern) so range/arm drift fails loudly.

-             _ => selected["evidence_requirements"]["apple_team"] = "ABCDE12345".into(),
+             13 => selected["evidence_requirements"]["apple_team"] = "ABCDE12345".into(),
+             _ => unreachable!(),


─── tests/release_context.rs:761-761 ───
[maintainability · low] Same wildcard-arm hazard as the baseline loop: an out-of-sync `0..15` range
silently re-runs the `build_workflow` mutation or drops a new case. Use an explicit `14 =>` arm plus
`_ => unreachable!()`.

-             _ => catalog["runtime"]["build_workflow"]["commit"] = "e".repeat(40).into(),
+             14 => catalog["runtime"]["build_workflow"]["commit"] = "e".repeat(40).into(),
+             _ => unreachable!(),


─── tests/release_context.rs:860-860 ───
[maintainability · low] Same wildcard-arm hazard in the `0..12` archive-mutation loop: range/arm
drift silently re-applies the `archive[1536] ^= 1` mutation. Use an explicit `11 =>` arm plus `_ =>
unreachable!()`.

-             _ => archive[1536] ^= 1,
+             11 => archive[1536] ^= 1,
+             _ => unreachable!(),


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



─── tests/historical.rs:369-372 ───
[test · low] Pin the rejection reason: the scan loop maps non-UTF-8 names to
`historical-download-name-invalid`; a bare `is_err()` would also pass if the file were rejected
merely as an extra set member.

          b"invalid name",
      )
      .unwrap();
-     assert!(verify(directory.path(), &policy, &source).is_err());
+     let error = verify(directory.path(), &policy, &source).err().unwrap();
+     assert!(
+         error.to_string().contains("historical-download-name-invalid"),
+         "{error}"
+     );


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



─── tests/upgrade.rs:592-592 ───
[test · low] mtime-equality is a weak no-op signal: on filesystems with coarse timestamp granularity
(1–2s ticks, common on CI volumes), an implementation that rewrites identical bytes during the
"unchanged" replay within the same tick passes this assertion, giving false confidence that no
rewrite occurred. Also, `Metadata::modified()` is unsupported on some platforms and will panic via
`unwrap()`. Ensure at least one filesystem timestamp tick elapses between the first apply and the
replay (or compare a stronger signal such as ctime/inode where available) so a spurious rewrite is
reliably detected.



─── tests/upgrade.rs:965-969 ───
[test · low] This test hard-depends on an external GNU `patch` (`--batch` and `-E` are GNU-specific;
BSD patch differs, Windows/minimal containers usually lack it). `spawn().unwrap()` panics with an
opaque "No such file or directory" when the binary is missing — use `.expect("GNU patch is required
for independent diff reconstruction")` or skip gracefully. Additionally, `patch` reports failures on
stderr, but the assertion message prints only stdout, making real diff-reconstruction failures hard
to diagnose; include `result.stderr` in the message.

              assert!(
                  result.status.success(),
-                 "{path}: {}",
-                 String::from_utf8_lossy(&result.stdout)
+                 "{path}: {}{}",
+                 String::from_utf8_lossy(&result.stdout),
+                 String::from_utf8_lossy(&result.stderr)
              );


─── examples/bootstrap-v2/library/Cargo.toml:5-5 ───
[maintainability · low] This library crate omits `rust-version` even though the repository has an
MSRV policy: the root `Cargo.toml` declares `rust-version = "1.95"` and the `profile-workspace`
example follows it. Edition 2024 requires Rust >= 1.85, so without an explicit MSRV an older
toolchain fails with a cryptic edition error instead of a clear "requires rustc X or newer" message.
Declare the repo MSRV here for consistency.

  edition = "2024"
+ rust-version = "1.95"


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


─── .github/workflows/momus.yml:3-5 ───
[performance · low] No `concurrency` group is defined. This workflow triggers on every `synchronize`
event, so rapid successive pushes to a PR will queue overlapping review runs, wasting runner time
and potentially posting stale or out-of-order review comments. Add a concurrency group keyed on the
PR number with `cancel-in-progress: true` so only the latest commit is reviewed.

  on:
    pull_request:
      types: [opened, synchronize, reopened]
+
+ concurrency:
+   group: ${{ github.workflow }}-${{ github.event.pull_request.number }}
+   cancel-in-progress: true


─── .github/workflows/momus.yml:12-13 ───
[performance · low] The review job has no explicit `timeout-minutes`, so a hung or slow review can
occupy a runner for up to the default 360 minutes. Note that `timeout-minutes` cannot be added to
this calling job: because it invokes a reusable workflow via `uses:`, GitHub Actions only permits
`name`, `uses`, `with`, `secrets`, `needs`, `if`, `permissions`, `concurrency`, and `strategy` on
the calling job — adding `timeout-minutes: 15` here would make the workflow invalid. To bound
runtime (e.g., 15 minutes), `timeout-minutes` must be set on the jobs inside
`brianluby/momus-review/.github/workflows/review.yml` (pinned at v0.9.0); consider
verifying/requesting that upstream.

    review:
      if: github.event.pull_request.head.repo.full_name == github.repository
+     timeout-minutes: 15


─── .github/workflows/development.yml:145-145 ───
[bug · low] `archive` is not a recognized input for `actions/upload-artifact` (valid inputs: `name`,
`path`, `if-no-files-found`, `retention-days`, `compression-level`, `overwrite`,
`include-hidden-files`). Unknown inputs are silently ignored, so this line is dead config. Remove
it, or use `compression-level` if the intent was to control archiving/compression.

-           archive: true
+           compression-level: 6


─── .github/workflows/development.yml:31-31 ───
[performance · low] No dependency caching anywhere: every run on all 3 matrix legs reinstalls the
exact rustup toolchain and rebuilds all crates from scratch for clippy, test, debug build, and
release build (within a 25-minute timeout). Consider `actions/cache` (or Swatinem/rust-cache) keyed
on `rust-toolchain.toml` + `Cargo.lock` for `~/.cargo` and `target/`. If caching is deliberately
omitted to avoid cache-poisoning of the uploaded runtime candidate, restrict any cache to read-only
on PR runs (`restore-keys` without save) — but as written this is a large, repeated cost.



─── .github/workflows/development.yml:13-13 ───
[bug · medium] `cancel-in-progress: true` applies to `push` on `main` as well as PRs. A new merge to
`main` will cancel the still-running validation of the previous main commit, so that commit never
produces its `runtime-candidate-*` artifact (which appears intended for downstream consumption,
given `overwrite: false` and per-run unique names). Limit cancellation to PRs so every main commit
completes and uploads its candidate.

-   cancel-in-progress: true
+   cancel-in-progress: ${{ github.event_name == 'pull_request' }}


─── .gitignore:1-1 ───
[maintainability · low] The leading slash anchors this to the repo root only. This repo contains
standalone Cargo packages under `examples/` (e.g. `examples/bootstrap-v2/cli/Cargo.toml`,
`examples/profile-workspace/`), and building them creates nested `target/` directories that will
show up as untracked files. Drop the anchor so build output is ignored at any depth.

- /target/
+ target/



──────── Project Summary ────────

### Top Issues

1. **Fail-open acceptance gates defeat the tool's core security promises.** `src/verification/publication/github.rs` treats an empty deployment-status list (`200 []`) as "no active deployment", excluding a competing deployment from `active_count` (fetch failures/truncation are likewise downgraded). `src/verification/historical.rs` returns a successful `HistoricalByteMatch` for an empty asset map. `src/trust/publication.rs` accepts any single `ByteIdentity` as "per-stage" evidence, leaves optional evidence unvalidated outside its owning state, and omits the remote draft digest from receipt identity — a retry can resume against replaced draft bytes. `src/trust/policy.rs` exposes acceptance helpers on a freely constructible policy with no enforcement that `validate(now)` ran. The schemas compound this: `schemas/publish-set-v1.json` accepts empty/duplicate crate sets and unconstrained identity strings; `schemas/native-catalog-v2.json` accepts pin-less adapters the runtime rejects.

2. **Check-then-act symlink races at every filesystem trust boundary.** The stat-then-open pattern repeats in `src/apply.rs` (`apply.lock`), `src/lib.rs` (read guard), `src/config.rs` (`try_exists` treats a dangling lock symlink as "no lock"), `src/bootstrap/transaction.rs` (`is_dir()` after `safe_path`), `src/verification/publication/mod.rs` (lock acquisition + directory trust), and `src/verification/io.rs` (path-based `set_permissions`; dev/ino binding is Unix-only). Worst case: `src/discovery.rs` gives `rustc` only an `is_file()` check while `cargo` is canonicalized and root-checked, so a workspace-pointing symlink is executed via `RUSTC`.

3. **Crash/failure windows permanently wedge publication; committed transactions are misreported as failures.** In `src/verification/publication/mod.rs`, `Creating`/`Uploading`/`Publishing` are persisted before the backend call with no retry or rollback path, and the journal is never rotated after `Complete` while `load` hard-fails on run/attempt-bound plan mismatch — one crash or 403 blocks all future releases for that repository. The same root cause appears in the apply trio: post-commit cleanup failures propagate as bare I/O errors indistinguishable from failed transactions (`src/apply.rs`, `src/bootstrap/transaction.rs`, `src/upgrade/transaction.rs`), and `src/apply.rs` ignores `unlock()` despite its own documented flock invariant.

4. **Non-injective identity keys and Debug-based dedup weaken pin/evidence binding.** `--`-joined composite keys collide because identifiers permit consecutive hyphens (`src/trust/evidence.rs`, `src/verification/context.rs`); `src/catalog.rs` silently overwrites duplicate pin keys in a `BTreeMap`; `src/trust/capability.rs` matches lock tools against pins across *all* adapters while uniqueness is enforced only per-adapter; `src/trust/native.rs` and `capability.rs` dedup adapters by `format!("{:?}")`; `src/verification/graph.rs` (`native_linkage`) and `src/trust/evidence.rs` (requirement lists) lack uniqueness checks.

5. **The security test suite frequently passes without exercising the guard it names.** Roughly 40% of all comments are in tests. Bare `is_err()` negatives accept unrelated failures (`tests/real_sigstore.rs`, `tests/historical.rs`, `tests/trust_contracts.rs`, `tests/release_context.rs`); mutations can be silent no-ops (`tests/real_sigstore.rs` base64 `'A'` flip, `tests/upgrade.rs` string replace yielding malformed JSON, `tests/inspection.rs` unmatched `replace`); assertions are vacuous (`tests/inspection.rs` compares stale bytes, `tests/historical.rs` size-cap checked against wrong content and markers written to the wrong cwd, `src/verification/publication/tests.rs` asserts `creates == 0` without ever attempting staging, fakes discard byte bounds and assign "foreign" authors inside the allow-list). Named contracts are untested: approval-required (`tests/apply.rs`), unknown-field rejection (`tests/trust_contracts.rs`), v2 catalog hardening (`tests/catalog.rs`).

6. **Published schemas drift from runtime validation, and the bounds generator corrupts schemas.** `src/config.rs` hardcodes `max = 3` against `TARGETS`, omits feature-name/`license_file` constraints, and pins `runtime_version` to `x.y.z` while requiring exact `CARGO_PKG_VERSION` equality (prerelease builds become unlockable). `src/trust/mod.rs` leaves `git_ref` unconstrained, mixes code-point and byte lengths, and allows unbounded version digits. `schemas/publish-set-v1.json` relies on annotation-only `format: uint64` with maxima beyond the 2^53−1 interop range and rejects legitimate `.yaml` workflows (repeated across sibling schemas). `src/schema_bounds.rs` overwrites stricter non-u64 `maximum`s with `u32::MAX`/`u64::MAX` and injects `maximum` into annotation payloads (`default`/`examples`/`const`/`enum`).

7. **Advertised time/memory bounds are not enforced end-to-end.** `src/verification/release.rs` never samples the deadline in its final rehash (up to 4 GiB / 8194 files) and does O(n²) asset lookups; `src/verification/runtime.rs` snapshots ~384 MiB outside the 20-minute budget; `src/verification/apple.rs` runs Security.framework validity checks with network access allowed and no deadline; `src/verification/sigstore.rs`'s "timeout" only breaks a poll loop (blocking `wait()` after `kill()`); `src/discovery.rs`'s `rustup which` is unbounded. Memory amplification: `github.rs` buffers 1 GiB assets at ~2× RSS, `apple.rs` holds the archive `Vec` through `native()`, `src/trust/inventory.rs` allocates all names before applying the 8192 cap.

8. **Error-category collapse hides security signals and misdirects operators.** `Error::Json` ("JSON serialization failed") covers parsing, duplicate-key anti-smuggling rejections, base64 faults, state-machine invariants, and even missing HOME (`src/trust/mod.rs`, `src/verification/io.rs`, `src/verification/bundle.rs`, `src/verification/publication/mod.rs`, `src/main.rs`). `Error::Io` renders write/delete failures as "cannot read local input" (`src/lib.rs`). `src/config.rs` discards TOML spans; `src/upgrade/state.rs` collapses six invariant failures into one message; `src/upgrade/transaction.rs`'s `.is_ok()` turns transient I/O into `configured: false`; `src/main.rs` exits 1 with no diagnostic at all on stdout/serialization failure.

9. **Security-critical logic is duplicated or compared via serialization.** `src/main.rs` re-implements the `freeze_publication_inputs` verification sequence verbatim; `src/verification/io.rs` duplicates the size-gated hashing loop; `src/verification/apple.rs` re-implements `ByteIdentity::from_bytes`; `src/upgrade/authority.rs` is a third copy of the catalog registry. Equality-by-`serde_json::to_value` in `src/trust/evidence.rs`/`inventory.rs` assumes serde injectivity for a security check; `src/verification/publication/github.rs` double-observation hashes entire provider bodies including volatile `pushed_at`/`updated_at`, guaranteeing spurious mismatches.

10. **Safety rests on undocumented cross-module coincidences; library paths can panic.** Unchecked `zip` cardinality (`src/bootstrap/plan.rs`, `src/upgrade/plan.rs`), direct indexing `files["rust-toolchain.toml"]` and `nodes[id]` (`src/upgrade/plan.rs`, `src/verification/graph.rs`), `unreachable!()` in CLI matches (`src/main.rs`), and Apple signing gates hardcoded to `aarch64-apple-darwin` (`src/trust/capability.rs`, `src/verification/context.rs`) — adding any Darwin triple to `TARGETS` silently bypasses notarization gating. `src/upgrade/plan.rs` also enforces overlap/sorting checks in `inspect_bytes` but not `validate()`, and recomputes `policy_review_day` so UTC rollover breaks freshness of byte-identical inputs.

### Module Hotspots

- **`src/verification/publication/`** — 16 comments, the highest severity concentration: fail-open gate (`github.rs`), three journal-wedging paths plus lock TOCTOU (`mod.rs`), and 7 comments in `tests.rs` about doubles that don't model the real backend.
- **`src/verification/` core** (`io.rs`, `release.rs`, `runtime.rs`, `sigstore.rs`, `apple.rs`, `context.rs`, `bundle.rs`, `graph.rs`, `historical.rs`) — ~25 comments: deadline gaps, symlink handling, duplicated hashing, vacuous historical success, byte-identity contract violations.
- **`src/trust/`** — 24 comments across `capability.rs`, `evidence.rs`, `publication.rs`, `mod.rs`, `inventory.rs`, `native.rs`, `policy.rs`: identity keys, serialization equality, per-state validation gaps.
- **Transaction core** — `src/apply.rs`, `src/bootstrap/transaction.rs`, `src/upgrade/transaction.rs` (plus `upgrade/state.rs`, `upgrade/plan.rs`): the identical post-commit error-handling defect in three places.
- **`tests/`** — ~78 comments (≈38% of the scan). Densest: `tests/release_context.rs` (9), `tests/historical.rs` (8), `tests/inspection.rs` (8), `tests/apply.rs` (7), `tests/integrated_cli.rs` (7).
- **Schema surface** — `schemas/`, `src/config.rs`, `src/trust/mod.rs`, `src/schema_bounds.rs` (~17 comments): contract drift plus active generator bugs.
- **`src/discovery.rs`** — 4 comments but includes the only direct code-execution boundary gap (`rustc` symlink).

### Cross-Cutting Concerns

- **Swallowed results.** `.is_ok()` / `.is_ok_and()` / `.ok()` / `unwrap_or_default()` / ignored `unlock()` erase the distinction between "absent", "denied", and "tampered": `src/apply.rs`, `src/upgrade/transaction.rs`, `tests/integrated_cli.rs`, `tests/preview.rs`.
- **Stringly-typed security invariants.** Magic disposition literals (`src/plan.rs`), `--` composite keys, Debug-format identity (`src/trust/native.rs`), raw TOML interpolation of `toolchain` (`src/catalog.rs`), weak `pin.version` charset (`src/trust/capability.rs`, `native.rs`), DEL/C1 controls passing the filter (`src/ci_policy.rs`).
- **Producer/consumer validation asymmetry.** `src/upgrade/plan.rs` (`validate()` vs `inspect_bytes`), `src/verification/bundle.rs` (`load_download` re-serializes "exact transport bytes"), `src/verification/context.rs` (Legacy pin-less adapter vacuously passes; authenticated `Cargo.lock` bytes dropped), `src/trust/publication.rs` (state-conditional evidence validation).
- **Test oracles that follow symlinks or ignore metadata.** `tests/bootstrap.rs`, `tests/integrated_cli.rs`, `tests/preview.rs` snapshots use `is_dir()`/`fs::read` (FIFO reads hang, symlink cycles escape the fixture) and never capture file types/permissions — the harnesses meant to detect escapes can themselves escape.
- **Environment-dependent tests.** Unconditional Unix imports (`tests/publication_cli.rs` breaks non-Unix compilation), external GNU `patch` (`tests/bootstrap.rs`, `tests/preview.rs`, `tests/upgrade.rs`), hardcoded `/usr/bin` paths (`tests/publication_cli.rs`, `tests/inspection.rs`), root/`0o500` assumptions (`tests/integrated_cli.rs`), rustup-on-PATH (`tests/inspection.rs`), coarse-mtime no-rewrite oracles (`tests/apply.rs`, `tests/bootstrap.rs`, `tests/upgrade.rs`).
- **Positional coupling to fixture ordering.** `changes[1]`/`files[0]` indexing (`tests/bootstrap.rs`, `tests/preview.rs`, `tests/integrated_cli.rs`), wildcard match arms with drifting ranges and fixed tar offsets (`tests/release_context.rs`), `remove(0)` (`tests/cargo_graph_v2.rs`), `checks + 2` (`src/verification/publication/tests.rs`).
- **Half-hearted non-Unix support.** `src/filesystem.rs` unused import fails `-D warnings`, `src/verification/io.rs` identity binding degrades silently, `tests/apply.rs` and `tests/cargo_graph_v2.rs` run Unix-only logic ungated.
- **Needless clones in hot paths.** `src/schema_bounds.rs` (deep schema clone + `unwrap`), `src/upgrade/state.rs` (preimage map), `src/upgrade/transaction.rs` (`proposed_content` on skipped writes), `src/trust/evidence.rs` (per-step/per-asset allocations), `src/plan.rs` (`sha256.clone()` for a comparison).

### Quick Wins

- **Workflows:** remove the dead `archive` input and scope `cancel-in-progress` to PRs only (main-commit artifacts are currently cancelled) in `.github/workflows/development.yml`; add a concurrency group to `.github/workflows/momus.yml`; add rustup/cargo caching.
- **Packaging:** add `include`/`exclude` to `Cargo.toml` (fixtures, docs, nested examples currently ship); unanchor `target/` in `.gitignore`; add `rust-version` to `examples/bootstrap-v2/library/Cargo.toml`.
- **Unbreak non-Unix builds:** cfg-gate the `OpenOptions` import in `src/filesystem.rs` and the `std::os::unix` import in `tests/publication_cli.rs`.
- **Close the exec gap:** apply cargo's canonicalize+root check to `rustc` in `src/discovery.rs`; skip broken PATH candidates with `let Ok(..) else { continue }`.
- **Sanitization:** reject U+007F and C1 controls in `src/ci_policy.rs`; refuse pre-existing destinations in `scripts/qualify-test-verifier.py` instead of `exist_ok=True`.
- **Schemas:** add `minItems`/`uniqueItems` and the shared crate-name pattern to `schemas/publish-set-v1.json`; accept `.yaml` in all sibling workflow patterns; make `previous_catalog` required (nullable) in `schemas/native-catalog-v2.json`.
- **Dedup:** reuse `ByteIdentity::from_bytes` and `drop(archive)` before `native()` in `src/verification/apple.rs`; derive `Ord` on `AdapterId` and dedup by typed id in `src/trust/capability.rs`/`native.rs`; destructure instead of clone+unwrap and skip annotation subtrees in `src/schema_bounds.rs`.
- **Diagnostics:** emit a stderr message before the silent exit 1 and use a config-error variant for missing HOME in `src/main.rs`; sample `deadline()` in the final rehash loop of `src/verification/release.rs`.
- **Diff correctness:** use `/dev/null` headers for empty sides in `src/preview.rs` and `src/upgrade/rollback.rs` (deletions currently render as truncations).
- **Mechanical test hardening (highest leverage per effort):** replace bare `is_err()` with expected error codes; `assert_ne!` after every fixture mutation; compare bytes instead of mtimes; include action/stderr in CLI failure messages; gate or PATH-resolve GNU `patch`/`mkfifo`/`touch` dependencies.
- **Allocation hygiene:** borrow instead of clone in `src/plan.rs` and `src/trust/evidence.rs` comparisons; hoist the expected asset name out of the `find` closure in `src/trust/evidence.rs`.
