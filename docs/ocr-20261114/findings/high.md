# HIGH severity findings (9)

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

─── tests/release_context.rs:205-208 ───
[test · high] Vacuous on unsupported hosts: `TrustedReleaseContext::open` always fails at the
`qualified_native_validator()?` gate (src/verification/context.rs:323) when the host is unsupported,
and `fixture()` deliberately embeds a bogus validator identity in that case. Since this test only
asserts `is_err()` and is not `cfg_attr`-gated like the other semantic tests, it passes on
unsupported hosts even if every config/lock/catalog/policy substitution check were deleted. Add the
same `#[cfg_attr(..., ignore)]` gate used by the other semantic tests, and/or assert the specific
authority-mismatch error code so an unrelated rejection path cannot satisfy the assertion.

