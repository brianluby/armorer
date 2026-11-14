# Review findings — group D_catalog_plan (22 findings)

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

