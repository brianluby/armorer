# Review findings — group E_publication (21 findings)

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

