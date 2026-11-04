# Ticket #4 CLI acceptance evidence

Date: 2026-11-04 UTC. Scope: Armorer project 16 ticket **#4** (database ID
**1307**) against source `ab526d7a0d3b547cc39041b52262e60173223b49` on branch
`evidence/acceptance-4`. This is a local, disposable-consuming-repository
receipt, not ticket completion, production-catalog approval, hosted CI evidence,
or release evidence.

Veans task readback was attempted with `rtk veans show 1307`. This isolated
worktree has no `.veans.yml`, so Veans returned `NOT_CONFIGURED`; no tracker
write, comment, or status change was made. The ticket requirements are also
recorded in `IMPLEMENTATION_PLAN.md` and the current acceptance ledger.

## Fixture, executable, and notation

All observations below were freshly replayed using the checked-out debug CLI and
only `/tmp/armorer-4-cli-evidence-replay` disposable consumers. No command runs
a consuming build or build script. Exit/result comparisons use shell zero as
success: `*_unchanged=0` means identical SHA-256 output before and after.

```sh
rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo build --locked
rtk rm -rf /tmp/armorer-4-cli-evidence-replay
rtk mkdir -p /tmp/armorer-4-cli-evidence-replay
rtk cp -R examples/bootstrap-v2/library /tmp/armorer-4-cli-evidence-replay/create
rtk printf 'unrelated dirty bytes\r\n' > /tmp/armorer-4-cli-evidence-replay/create/notes.txt
```

The observed build result was `Finished dev profile [unoptimized + debuginfo]`.
Every plan file is outside its consuming fixture root. For concise copyable
commands below, set these shell variables first:

```sh
rtk sh -c 'ARMORER=/Users/bluby/repos/armorer-evidence-4/target/debug/armorer; ROOT=/tmp/armorer-4-cli-evidence-replay; export ARMORER ROOT; exec "$SHELL"'
```

The commands use their literal paths so they remain reproducible without an
interactive exported shell.

## C1 — read-only inspection, bounded preview, and independent approval digest

The following checks every regular fixture file before and after `check`:

```sh
rtk sh -c 'before=$(find /tmp/armorer-4-cli-evidence-replay/create -type f -exec shasum -a 256 {} \; | sort); /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/create bootstrap check --policy /tmp/armorer-4-cli-evidence-replay/create/ci-policy.reviewed.toml > /tmp/armorer-4-cli-evidence-replay/check.json; status=$?; after=$(find /tmp/armorer-4-cli-evidence-replay/create -type f -exec shasum -a 256 {} \; | sort); test "$before" = "$after"; printf "check_status=%s check_read_only=%s\n" "$status" "$?"'
```

Observed output: `check_status=2 check_read_only=0`. The JSON listed exactly
these create dispositions: `.armorer/ci-policy.toml`,
`.github/workflows/armorer-build.yml`, `.github/workflows/armorer-ci.yml`,
`armorer.lock`, and `rust-toolchain.toml`. Its remaining findings were
`github-capabilities-unchecked` and `release-runtime-unavailable`; check's
nonzero status is an intentional secure-gate result, not a write.

This separately checks a normal successful plan for read-only behavior and then
prints each exact preview summary:

```sh
rtk sh -c 'before=$(find /tmp/armorer-4-cli-evidence-replay/create -type f -exec shasum -a 256 {} \; | sort); /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/create bootstrap plan --policy /tmp/armorer-4-cli-evidence-replay/create/ci-policy.reviewed.toml > /tmp/armorer-4-cli-evidence-replay/create-plan.json; status=$?; after=$(find /tmp/armorer-4-cli-evidence-replay/create -type f -exec shasum -a 256 {} \; | sort); test "$before" = "$after"; printf "plan_status=%s plan_read_only=%s\n" "$status" "$?"'
rtk python3 -c 'import json; v=json.load(open("/tmp/armorer-4-cli-evidence-replay/create-plan.json")); print("create_digest="+v["plan_sha256"]); [print("{} {} before={} proposed_bytes={} diff_header={}".format(c["path"],c["disposition"],c["before_content"] is not None,len(c["proposed_content"].encode()),c["unified_diff"].splitlines()[0])) for c in v["changes"]]'
```

Observed: `plan_status=0 plan_read_only=0`; digest
`7b51e848ce50bf4b5280ff82098d1870e9795da413654915953a5006e7e4af4b`.
Every create change had `before=False`, complete proposed content, and a
`--- /dev/null` unified-diff header. Proposed byte sizes were respectively 275,
320, 326, 2747, and 86 in the path order above. This records together the
absence/preimage, proposed bytes, disposition, and complete replacement diff.

A deliberately oversized escaped conflict is bounded rather than truncated and
its input is unchanged:

```sh
rtk sh -c 'rm -rf /tmp/armorer-4-cli-evidence-replay/bounded; cp -R examples/bootstrap-v2/library /tmp/armorer-4-cli-evidence-replay/bounded; python3 -c '\''open("/tmp/armorer-4-cli-evidence-replay/bounded/rust-toolchain.toml", "wb").write(b"\\t" * 300000)'\''; before=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/bounded/rust-toolchain.toml); /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/bounded bootstrap plan --policy /tmp/armorer-4-cli-evidence-replay/bounded/ci-policy.reviewed.toml > /tmp/armorer-4-cli-evidence-replay/bounded-plan.json; status=$?; after=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/bounded/rust-toolchain.toml); test "$before" = "$after"; printf "bounded_plan_status=%s bounded_input_unchanged=%s\n" "$status" "$?"; cat /tmp/armorer-4-cli-evidence-replay/bounded-plan.json'
```

Observed: `bounded_plan_status=1 bounded_input_unchanged=0`, with
`transaction blocked: bootstrap plan exceeds 1 MiB`. The CLI emitted only the
error JSON, not a partial preview.

Applying the saved plan with an independently supplied wrong digest failed
before mutation while the unrelated dirty file retained its hash:

```sh
rtk sh -c 'before=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/create/notes.txt); /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/create bootstrap apply --plan /tmp/armorer-4-cli-evidence-replay/create-plan.json --expect-plan-sha256 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa > /tmp/armorer-4-cli-evidence-replay/wrong-digest.json; status=$?; after=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/create/notes.txt); test "$before" = "$after"; printf "wrong_digest_status=%s unrelated_after_wrong_digest=%s\n" "$status" "$?"; cat /tmp/armorer-4-cli-evidence-replay/wrong-digest.json'
```

Observed status `1`, `unrelated_after_wrong_digest=0`, and `bootstrap plan does
not match approved digest, catalog or runtime`.

## C2 — replay, unrelated bytes, stale preimages, and rollback boundary

The approved digest was applied and replayed without changing unrelated dirty
bytes:

```sh
rtk sh -c 'd=$(python3 -c "import json; print(json.load(open(\"/tmp/armorer-4-cli-evidence-replay/create-plan.json\"))[\"plan_sha256\"])"); before=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/create/notes.txt); /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/create bootstrap apply --plan /tmp/armorer-4-cli-evidence-replay/create-plan.json --expect-plan-sha256 "$d" > /tmp/armorer-4-cli-evidence-replay/applied.json; apply_status=$?; after=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/create/notes.txt); test "$before" = "$after"; printf "apply_status=%s unrelated_after_apply=%s\n" "$apply_status" "$?"; /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/create bootstrap apply --plan /tmp/armorer-4-cli-evidence-replay/create-plan.json --expect-plan-sha256 "$d" > /tmp/armorer-4-cli-evidence-replay/reapplied.json; printf "reapply_status=%s\n" "$?"; cat /tmp/armorer-4-cli-evidence-replay/applied.json; cat /tmp/armorer-4-cli-evidence-replay/reapplied.json'
```

Observed `apply_status=0 unrelated_after_apply=0 reapply_status=0`; receipts
were `outcome: "applied"` then `outcome: "unchanged"`, both `configured: true`
and with `ci_verified`, `release_rehearsed`, `published`, and
`provenance_verified` all `false`.

This is the exact stale-preimage mutation and old-plan submission, including the
unrelated-byte comparison:

```sh
rtk sh -c 'd=$(python3 -c "import json; print(json.load(open(\"/tmp/armorer-4-cli-evidence-replay/create-plan.json\"))[\"plan_sha256\"])"); printf "tampered managed custom bytes\n" > /tmp/armorer-4-cli-evidence-replay/create/rust-toolchain.toml; before=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/create/notes.txt); /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/create bootstrap apply --plan /tmp/armorer-4-cli-evidence-replay/create-plan.json --expect-plan-sha256 "$d" > /tmp/armorer-4-cli-evidence-replay/stale-preimage.json; status=$?; after=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/create/notes.txt); test "$before" = "$after"; printf "stale_preimage_status=%s unrelated_after_stale=%s\n" "$status" "$?"; cat /tmp/armorer-4-cli-evidence-replay/stale-preimage.json'
```

Observed `stale_preimage_status=1 unrelated_after_stale=0` and `committed
bootstrap receipt conflicts with files or ownership`. The stale plan therefore
did not overwrite the modified managed target.

**Rollback limitation:** no production CLI fault-injection control exists for a
reproducible partial-write failure. This receipt therefore marks CLI
partial-failure rollback **UNPROVEN**, rather than treating private tests as CLI
acceptance evidence. `src/bootstrap/transaction.rs` has private
transaction-boundary rollback/recovery coverage, including all target write
boundaries and commit-marker failures, but that is separately scoped source test
evidence. The smallest future owner-reviewed evidence method is a purpose-built,
non-production test harness or an owner-approved deterministic filesystem-failure
environment that drives the released CLI; do not add a public apply
fault-injection flag merely to create this receipt.

## C3 and C4 — conflict, update, unsafe paths, and concurrent apply

This creates the conflict fixture, records exact custom workflow bytes, plans,
and verifies that applying the conflict leaves the custom workflow unchanged:

```sh
rtk sh -c 'cp -R examples/bootstrap-v2/library /tmp/armorer-4-cli-evidence-replay/conflict; mkdir -p /tmp/armorer-4-cli-evidence-replay/conflict/.github/workflows; printf "bespoke workflow\r\ncustom job no final newline" > /tmp/armorer-4-cli-evidence-replay/conflict/.github/workflows/armorer-ci.yml; /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/conflict bootstrap plan --policy /tmp/armorer-4-cli-evidence-replay/conflict/ci-policy.reviewed.toml > /tmp/armorer-4-cli-evidence-replay/conflict-plan.json; d=$(python3 -c "import json; print(json.load(open(\"/tmp/armorer-4-cli-evidence-replay/conflict-plan.json\"))[\"plan_sha256\"])"); before=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/conflict/.github/workflows/armorer-ci.yml); /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/conflict bootstrap apply --plan /tmp/armorer-4-cli-evidence-replay/conflict-plan.json --expect-plan-sha256 "$d" > /tmp/armorer-4-cli-evidence-replay/conflict-apply.json; status=$?; after=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/conflict/.github/workflows/armorer-ci.yml); test "$before" = "$after"; printf "conflict_apply_status=%s custom_workflow_unchanged=%s\n" "$status" "$?"; cat /tmp/armorer-4-cli-evidence-replay/conflict-apply.json'
rtk python3 -c 'import json; v=json.load(open("/tmp/armorer-4-cli-evidence-replay/conflict-plan.json")); c=next(c for c in v["changes"] if c["path"]==".github/workflows/armorer-ci.yml"); print("conflict={} before={!r} proposed_bytes={} diff_has_before={} diff_has_proposed={}".format(c["disposition"],c["before_content"],len(c["proposed_content"].encode()),"-bespoke workflow" in c["unified_diff"],"+name: Armorer CI" in c["unified_diff"]))'
```

Observed `conflict_apply_status=1 custom_workflow_unchanged=0`; the error was
`bootstrap conflict or unfinished transaction requires explicit resolution`.
The plan recorded `conflict`, exact `bespoke workflow\r\ncustom job no final
newline` before content, 326 proposed bytes, and both old/proposed workflow
lines in the unified diff.

For the update disposition, first restore the generated toolchain after the
stale-preimage exercise, then change only explicit policy bytes, plan, and apply:

```sh
rtk sh -c 'python3 -c '\''import json; root="/tmp/armorer-4-cli-evidence-replay/create"; p=json.load(open(root+"-plan.json")); open(root+"/rust-toolchain.toml", "w").write(next(c["proposed_content"] for c in p["changes"] if c["path"] == "rust-toolchain.toml"))'\''; printf "# revised owner policy\n" >> /tmp/armorer-4-cli-evidence-replay/create/ci-policy.reviewed.toml; /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/create bootstrap plan --policy /tmp/armorer-4-cli-evidence-replay/create/ci-policy.reviewed.toml > /tmp/armorer-4-cli-evidence-replay/update-plan.json; python3 -c '\''import json; v=json.load(open("/tmp/armorer-4-cli-evidence-replay/update-plan.json")); [print(c["path"]+":"+c["disposition"]+":before="+str(c["before_content"] is not None)) for c in v["changes"]]; print(v["plan_sha256"])'\''; d=$(python3 -c "import json; print(json.load(open(\"/tmp/armorer-4-cli-evidence-replay/update-plan.json\"))[\"plan_sha256\"])"); /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/create bootstrap apply --plan /tmp/armorer-4-cli-evidence-replay/update-plan.json --expect-plan-sha256 "$d" > /tmp/armorer-4-cli-evidence-replay/update-applied.json; cat /tmp/armorer-4-cli-evidence-replay/update-applied.json'
```

Observed `.armorer/ci-policy.toml:update:before=True`; the other four fixed
targets were `unchanged:before=True`. Digest
`50c6c185545aea62213a67fe07cbcb0cd76de3c29044556b08d517c68272a7e0` applied
with `outcome: "applied"`. Along with create and conflict above, every requested
disposition has CLI evidence carrying before and proposed content together.

C4 coverage was inspected in `tests/bootstrap.rs`, `tests/preview.rs`, and
`tests/integrated_cli.rs`. Existing coverage exercises complete replacement
hunks for create/update/conflict, CRLF/Unicode/no-final-newline cases, bounded
views, and independent patch reconstruction. No genuinely uncovered preview
case was found, so no regression was added. The CLI commands above are the
acceptance evidence; test names are not relied on as proof.

This creates a symlinked target parent and outside sentinel, then checks its
hash after plan rejection:

```sh
rtk sh -c 'cp -R examples/bootstrap-v2/library /tmp/armorer-4-cli-evidence-replay/symlink; mkdir /tmp/armorer-4-cli-evidence-replay/outside; printf "outside bytes" > /tmp/armorer-4-cli-evidence-replay/outside/preserve; ln -s /tmp/armorer-4-cli-evidence-replay/outside /tmp/armorer-4-cli-evidence-replay/symlink/.github; before=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/outside/preserve); /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/symlink bootstrap plan --policy /tmp/armorer-4-cli-evidence-replay/symlink/ci-policy.reviewed.toml > /tmp/armorer-4-cli-evidence-replay/symlink-plan.json; status=$?; after=$(shasum -a 256 /tmp/armorer-4-cli-evidence-replay/outside/preserve); test "$before" = "$after"; printf "symlink_plan_status=%s external_sentinel_unchanged=%s\n" "$status" "$?"; cat /tmp/armorer-4-cli-evidence-replay/symlink-plan.json'
```

Observed `symlink_plan_status=1 external_sentinel_unchanged=0`, with `symlinks
require migration before discovery`.

This writes the traversal configuration exactly before planning:

```sh
rtk sh -c 'cp -R examples/bootstrap-v2/library /tmp/armorer-4-cli-evidence-replay/traversal; cat > /tmp/armorer-4-cli-evidence-replay/traversal/armorer.toml <<'\''TOML'\''
schema_version = 1
repository = "example/pilot"
toolchain = "1.95.0"
[[deliverables]]
id = "primary"
profile = "library"
package = "bootstrap-library"
targets = ["x86_64-unknown-linux-gnu"]
feature_set = "standard"
[feature_sets.standard]
default_features = true
features = []
[policy]
license_file = "../outside"
attestations = "required"
TOML
/Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/traversal bootstrap plan --policy /tmp/armorer-4-cli-evidence-replay/traversal/ci-policy.reviewed.toml > /tmp/armorer-4-cli-evidence-replay/traversal-plan.json; printf "traversal_plan_status=%s\n" "$?"; cat /tmp/armorer-4-cli-evidence-replay/traversal-plan.json'
```

Observed `traversal_plan_status=1`, with `expected a plain relative path`.

This creates and plans a separate concurrent fixture, starts a lock holder,
submits apply while that holder is live, waits for it, and therefore tears it
down before reporting no managed write:

```sh
rtk sh -c 'cp -R examples/bootstrap-v2/library /tmp/armorer-4-cli-evidence-replay/concurrent; /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/concurrent bootstrap plan --policy /tmp/armorer-4-cli-evidence-replay/concurrent/ci-policy.reviewed.toml > /tmp/armorer-4-cli-evidence-replay/concurrent-plan.json; d=$(python3 -c "import json; print(json.load(open(\"/tmp/armorer-4-cli-evidence-replay/concurrent-plan.json\"))[\"plan_sha256\"])"); mkdir /tmp/armorer-4-cli-evidence-replay/concurrent/.armorer; python3 -c '\''import fcntl, time; f=open("/tmp/armorer-4-cli-evidence-replay/concurrent/.armorer/apply.lock", "w"); fcntl.flock(f, fcntl.LOCK_EX); time.sleep(8)'\'' & holder=$!; sleep 1; /Users/bluby/repos/armorer-evidence-4/target/debug/armorer --repository /tmp/armorer-4-cli-evidence-replay/concurrent bootstrap apply --plan /tmp/armorer-4-cli-evidence-replay/concurrent-plan.json --expect-plan-sha256 "$d" > /tmp/armorer-4-cli-evidence-replay/concurrent-apply.json; status=$?; wait "$holder"; test ! -e /tmp/armorer-4-cli-evidence-replay/concurrent/rust-toolchain.toml; printf "concurrent_apply_status=%s no_managed_write=%s\n" "$status" "$?"; cat /tmp/armorer-4-cli-evidence-replay/concurrent-apply.json'
```

Observed `concurrent_apply_status=1 no_managed_write=0`, with `another Armorer
transaction is active`.

## C5 and C6 — reviewed design slice, not expanded apply scope

The design slice is `docs/adr/0009-versioned-bootstrap-provisioning.md` and
`docs/bootstrap-v2.md`. The observed create plan implements only its fixed five
reviewable targets: explicit policy, catalog-backed `armorer.lock`, pinned CI and
manual unsigned-build caller templates, and `rust-toolchain.toml`. The observed
lock proposal contains workflow revision
`772ca83e386c883c88cc3b936d69f8cb3216c91e` and 18 target-qualified tool
SHA-256 pins. The policy proposal is the exact owner-supplied reviewed policy
bytes; its fixture-only MIT allowance is explicitly not a production policy.

The custom-workflow conflict demonstrates no silent replacement. The transaction
plans only its fixed target list; the unrelated `notes.txt` and bespoke workflow
are preserved. This evidence does not extend apply, adopt custom workflows/jobs/
policies, approve an operational caller, authenticate an upstream catalog, or
claim CI/build/release readiness.

## Still absent and requiring independent owner approval

The following are intentionally not self-approved by this receipt:

1. An independently owner-approved production catalog, workflow revision, tool
distribution identities/digests, and runtime choice. The embedded bootstrap
catalog is source authority under review, not that approval.
2. An owner-approved production project dependency policy (license allowlist,
advisory exceptions, source and bans decisions), rather than fixture policy.
3. Authenticated GitHub capability/policy provisioning and an operational
onboarding rehearsal in an actual consuming repository, including hosted caller
execution and all later CI/release gates.
4. Reproducible production-CLI partial-failure rollback evidence as called out
above.

No pilot repository, root checkout, catalog, pin, schema, trust module, tracker,
settings, release, merge, push, or tag was changed.

## Proposed single tracker comment

`Request In Review delivery for #4 / DB1307: commit <COMMIT> adds reviews/2026-11-04-ticket-4-cli-acceptance-evidence.md with disposable-consumer CLI receipts for read-only check/plan, bounded preview rejection, exact create/update/conflict previews, digest and stale-preimage rejection, replay no-op, unrelated-byte preservation, symlink/traversal rejection, and concurrent-apply rejection. CLI partial-failure rollback remains explicitly UNPROVEN; private tests are not claimed as acceptance proof. Before this delivery, required validations passed: rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo fmt --check — passed; rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo clippy --locked --all-targets --all-features -- -D warnings — passed; rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo test --locked — passed; rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo build --locked — passed. Production catalog/runtime approval, production policy, authenticated capability provisioning, operational onboarding, and CLI rollback evidence remain absent; no completion claim.`
