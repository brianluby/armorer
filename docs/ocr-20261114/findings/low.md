# LOW severity findings (93)

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

─── src/filesystem.rs:77-77 ───
[test · low] The symlink is created pointing at an existing file, so `AlreadyExists` here does not
distinguish "no clobber" from "no symlink follow" — the doc promise covers symlinks generally. Add a
dangling-symlink case: if the implementation ever followed links, it would create the missing target
instead of failing.

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

─── src/config.rs:57-57 ───
[maintainability · low] Schema hardcodes `max = 3` while `validate_shape` enforces `d.targets.len()
<= TARGETS.len()`. The moment `TARGETS` gains a fourth triple, the published schema rejects
configurations the runtime accepts (and the enum in `items` below already tracks `TARGETS`
automatically, so only this bound drifts). Derive the bound from the constant instead.

-     #[schemars(length(min = 1, max = 3))]
+     #[schemars(extend("minItems" = 1, "maxItems" = TARGETS.len(), "uniqueItems" = true, "items" = serde_json::json!({"enum": crate::config::TARGETS})))]

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

─── src/plan.rs:124-124 ───
[maintainability · low] Unnecessary allocation to satisfy the comparison: `base.sha256.clone()`
builds a `String` just to compare against an `Option<String>`. Compare by reference instead.

-             .is_some_and(|base| Some(base.sha256.clone()) == change.before_sha256)
+             .is_some_and(|base| change.before_sha256.as_ref() == Some(&base.sha256))

─── src/plan.rs:221-221 ───
[maintainability · low] `inspect` is a read-only preview, but a size overflow is reported as
`Error::Transaction`, surfacing to CLI consumers as "transaction blocked" (code `transaction`) when
no transaction exists — misleading diagnostics. `Error::Invalid` fits better here. Also, `1_048_576`
is now hard-coded in `read_small`, `Plan::digest`, and here; a shared `pub(crate) const
MAX_INPUT_BYTES` would keep the bound consistent.

─── src/trust/capability.rs:245-245 ───
[maintainability · low] Adapter de-duplication keys on `format!("{:?}", a.id)`: it allocates a
String per adapter and makes identity depend on Debug output stability (a future custom Debug impl
could silently break dedup). `AdapterId` is a plain Copy enum — add `PartialOrd, Ord` to its derive
and use `BTreeSet<AdapterId>` directly; no allocation, typed identity.

-             require(ids.insert(format!("{:?}", a.id)), "duplicate-adapter")?;
+             require(ids.insert(a.id), "duplicate-adapter")?;

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

─── src/verification/io.rs:184-184 ───
[maintainability · low] This is a byte-for-byte duplicate of the size-gated hashing loop in
`snapshot()` (same 64 KiB buffer, `take(max + 1)`, per-block `require(size <= max)`,
finalize/validate). Two copies of a security check invites divergence: a future fix to the size gate
or hashing must be applied twice. Extract one private streaming helper taking an optional writer
(`Option<&mut File>`) and call it from both functions.

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

─── src/verification/runtime.rs:48-48 ───
[bug · low] The 20-minute stage budget is only polled inside `decode` (and once at its end). This
`io::snapshot` call copies up to `MAX_DISTRIBUTION` (~384 MiB) from the untrusted archive path with
no deadline checks, and the later `io::identity` rehash / `sync_all` are likewise uncovered.
`io::regular` rules out FIFOs/symlinks, but a slow or hung filesystem (e.g., network mount) can
still block `open()` indefinitely before the first `deadline(started)` ever runs, so the stage
timeout does not actually bound the whole stage. If the bound is a real requirement, make the copy
loop deadline-aware (pass `started`/a deadline into the snapshot); otherwise document the budget as
best-effort so callers don't rely on it for liveness.

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

─── tests/catalog.rs:107-107 ───
[test · low] The test name promises "only reviewed full pins", but this is a presence check: an
extra unpinned reference (e.g. a second `uses: actions/checkout@main` line) would still pass, since
the pinned line remains and none of the forbidden substrings match. Also, the forbidden list misses
`permissions: write-all`, which grants write access without matching `"contents: write"`. Assert
exactly one `uses:` occurrence per workflow and add `"write-all"` to the forbidden list.

          assert!(value.contains(&format!("@{WORKFLOW_COMMIT}\n")));
+         assert_eq!(value.matches("uses:").count(), 1);

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

─── tests/inspection.rs:603-603 ───
[test · low] Weak assertion: checking only exit code 1 lets any unrelated failure satisfy the
'non-blocking FIFO rejection' guarantee. Capture stdout and assert the specific error code (as the
other CLI tests do) so this proves the FIFO was rejected for the right reason.

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

─── tests/release_context.rs:47-47 ───
[test · low] `now()` is sampled repeatedly (twice here, once per tool in the `observed_at` map,
again in the expiry/observation mutations) and each sample is signed into the approved digest, so
related timestamps can straddle a second boundary and the fixtures are non-reproducible. Capture one
instant at the top of `fixture()` (`let now = now();`) and derive
`reviewed_at`/`expires_at`/`observed_at` from it; pass the same base into the mutation tests.
Margins are currently wide enough to avoid flakes, but a single clock read removes the risk and
makes digests deterministic per run.

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

