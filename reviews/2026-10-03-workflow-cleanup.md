# Reap the reader. Require the reports.

Snapshot: 2026-10-03 UTC. Workflow integration candidate
`9d857a80f0943ecd06870f89d6bbde968a207625` is one commit after
`e88e25fc46c1bb5298579c5cd49f52d43215b9e1`. Accepted workflow main remains
`772ca83e386c883c88cc3b936d69f8cb3216c91e`; Armorer candidate remains
`56d7380a5534583b7973b5eedc55a1223b46b60e`.

Keep each source. Keep each failure. **This is the way.**

The complete seven-file successor patch changes three fixed Python native readers
(source, controller and mapped worker), one development schema-check statement,
two regression files and the v3 caller-pin guide. Readers now attempt SIGKILL on
their owned process group even after the leader exits, then wait for the leader
and close pipes. The previous leader-status guard left pipe-holding descendants
running after timeout. Process groups are a cleanup boundary, not containment of
children that escape the group. The inherited macOS PermissionError branch waits
for the leader; it is not independent proof of descendant termination.

The schema statement requires at least one `*/policy-v1.json` match and validates
every match. Its regression executes the actual workflow statement, rejecting
empty/misplaced reports, accepting a valid synthetic report and rejecting a
second malformed report. This is structural validation, not complete selection
or authentic observation acceptance. The caller guide now names the actual
`59a2e782150abff3f995682013d514e6865d8dfb` pin used by the lock and v1/v2/v3
callers; no caller pin changed.

Exact Git comparisons confirm schemas, tool pins, action wrappers, fixture lock
and the four inspected rehearsal callers are unchanged. The source/runtime
changes above mean the new tree is not runtime-byte-identical to its predecessor.
The [earlier integration review](2026-10-03-workflow-integration.md) and
[identity review](2026-10-03-identity-prerequisites.md) keep their original scope.

## Local controls

An immutable export, temporary hash-pinned CPython 3.14 environment and sterile
subprocess environment pass **48 tests**: four actual process cleanup tests, two
policy-schema tests, eleven source tests, twenty-one controller tests and ten
worker tests. Native-child tests use only owned inert executables and synthetic
read tokens. They do not authenticate real protected producers.

The four new exited-leader tests also run against the immutable predecessor:
all four fail because the descendant writes its delayed marker. All four pass at
the successor. Test finally blocks clean up the deliberately exposed children.
This negative control confirms the tested cleanup repair on this local macOS host.
No repository build script, credential value or consuming repository is involved.

## Reject startup transport overrides

Successor `3fac9a1cef3d9a5c4ac6f5c79c99d9255f63200a` changes only the issuer
helper and its issuer/mapped Node tests. The complete three-file patch rejects
any presence, including empty strings, of `NODE_OPTIONS`, `NODE_EXTRA_CA_CERTS`,
`NODE_TLS_REJECT_UNAUTHORIZED` and `NODE_USE_ENV_PROXY` before independent intent,
OIDC credential reads or HTTP. Removing variables after Node startup cannot undo
startup transport changes; a separately trusted clean launcher is required.
This guard does not restore trust in a compromised in-process runtime.

Local Node v22.22.3 tests use a sterile subprocess environment and ephemeral
synthetic RSA issuer keys. The new issuer test detects premature credential reads
with a throwing getter and confirms zero HTTP requests. The mapped test confirms
that each forbidden variable prevents the issuer join and both endpoint requests.
The fixed native mapping may already occur before the issuer rejection; the
mapped test does not establish zero native child work. All **12 issuer tests and 13 mapped tests pass**, with no skips. Workflows,
schemas, pins, lock and the three cleanup readers are byte-identical to
`9d857a80f094`.

## Hosted receipts at cleanup predecessor `9d857a80f094`

| Exact-head run | Terminal outcome |
| --- | --- |
| [37106718592: prerequisites](https://github.com/brianluby/armorer-workflows/actions/runs/37106718592) | All three hosts pass |
| [37106718578: unsigned Apple intake](https://github.com/brianluby/armorer-workflows/actions/runs/37106718578) | All three hosts pass |
| [37106718602: runtime validation](https://github.com/brianluby/armorer-workflows/actions/runs/37106718602) | All 12 test/build/policy/transport jobs pass |
| [37106718935: combined caller](https://github.com/brianluby/armorer-workflows/actions/runs/37106718935) | Failure; Linux x64 and ARM collection fail, macOS collection passes |

The combined run's nine build jobs, nine policy jobs and nine prerequisite CI
jobs pass. Both failed jobs stop at “Qualify actual complete build-policy archives
and uploader bindings.” Job/step results establish failure. Both Linux hosts report the fixed diagnostic
`archive-pair-reader:invariant-run-state`; it identifies a rejected run-state
invariant, not its underlying provider/race cause. No archive-by-archive audit or causal attribution is claimed here. The
preceding combined success retains its own source/run identity and supplies no
all-host positive for this attempt. Do not weaken complete-set or freshness checks
to make collection pass.

At successor `3fac9a1cef3d`, all three prerequisite jobs pass in
[37107472089](https://github.com/brianluby/armorer-workflows/actions/runs/37107472089)
and all three unsigned-intake jobs pass in
[37107472088](https://github.com/brianluby/armorer-workflows/actions/runs/37107472088).
Its runtime-validation run `37107472105` and combined run `37107472496` were
queued at this observation. No terminal positive is inferred from queue state.

No new exhaustive audit of inherited modules, private OIDC positive, protected
Apple transformation, final-byte cryptographic consumer acceptance, immutable
publication/recovery or pilot qualification is supplied. Those production gates
remain open. No source workflow, setting, tag, pilot or hosted review comment was
changed by this documentation continuation.

Hold the gate. Earn the joined handoff. **This is the way.**
