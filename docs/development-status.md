# What stands. What is still being forged.

Reconciliation snapshot: 2026-11-12 UTC. The source baseline is Armorer
`ab526d7a0d3b547cc39041b52262e60173223b49` (PR #27); the workflow baseline is `b94969c7030b8c1105882a1374ed4d7a3eab653a`.
Both retain the human-approved integration merges below plus PRs #26/#27.
Their latest push validation passes. **This is the way.**

## Accepted implementation

| Repository / PR | Accepted merge | Scope |
| --- | --- | --- |
| Armorer [#21](https://github.com/brianluby/armorer/pull/21) | `e866a92ea8e4003f6f0d47222ad7341bdac839d7` | Bootstrap, upgrade, complete/historical consumer and exact evidence scope |
| Workflows [#21](https://github.com/brianluby/armorer-workflows/pull/21) | `0f94f5c2abe8136c22540d881f92d0ad85f9753a` | Integrated unprivileged build/policy/producer and unsigned intake foundations |
| Armorer [#24](https://github.com/brianluby/armorer/pull/24) | `d70ef3d977de4bfe0194fd843365a17036a1c811` | Owned-draft controller and fail-closed native capability observations |
| Workflows [#22](https://github.com/brianluby/armorer-workflows/pull/22) | `c2909ebc19e378c68265b49f9816fbff9a53003d` | Native Node signature qualification; production authority remains separate |
| Armorer [#25](https://github.com/brianluby/armorer/pull/25) | `7cb4a3ec76f7bbe8fb0274bca1b59e4d1215e391` | Readiness/onboarding and scoped acceptance evidence |
| Armorer [#26](https://github.com/brianluby/armorer/pull/26) | `7b0cca9df2b5f3744ce2af109ab937d3097917c3` | Rerun-initiator self-review rejection and explicit completed state-lock release |
| Armorer [#27](https://github.com/brianluby/armorer/pull/27) | `ab526d7a0d3b547cc39041b52262e60173223b49` | OCR remediation: private staging hardening and tightened verification contracts |

Main exposes `check`, `catalog`, `bootstrap`, `upgrade`, `plan`, `preview`,
`apply`, `recover`, `verify-release`, `verify-historical-bytes`, `publication`
and `schema`. There are 27 schema exporters. Follow the
[command reference](cli-reference.md), [bootstrap](bootstrap-v2.md),
[upgrades](upgrades.md), [complete verifier](verify-release-cli.md),
[historical policy](historical-verification.md) and
[owned-draft controller](owned-draft-controller-v1.md).
Version-one apply remains toolchain-only; bootstrap has its own approval contract.
No `init` command is provided.

## Reconcile the earlier PRs

PRs #15–#19 target old parent branches, but their exact heads are ancestors of
the accepted source baseline. They are closed as superseded by #21. PR #20's
two preview files are byte-identical to main, so that docstring PR is also closed.
The schema repair from #23 is incorporated in the current 27-schema contributor
procedure; its old stacked PR is closed. #21 is already merged. This documentation
reconciliation is delivered through #22. Original branches and dated receipts
retain their evidence; closing a duplicate PR does not add release qualification.

Controller [#26](https://github.com/brianluby/armorer/pull/26) merged
2026-10-04 as `7b0cca9df2b5f3744ce2af109ab937d3097917c3`; its rerun-initiator
self-review rejection and explicit completed state-lock release are accepted
main behavior. [#27](https://github.com/brianluby/armorer/pull/27) then landed
the OCR remediation at `ab526d7`. Residual items U01–U14 are Veans #27;
three of the four Veans #26 qualification gates executed and passed
2026-11-12 (`reviews/2026-11-12-ticket-26-gate-execution.md`). The sequenced
finish plan is `reviews/2026-11-12-completion-plan.md`.

## Hosted evidence and remaining gates

Source [Development run 37218646841](https://github.com/brianluby/armorer/actions/runs/37218646841)
passes Linux x64, Linux ARM64 and macOS at `ab526d7`, including the native
Apple/Sigstore/CycloneDX/gh qualification steps. Workflow push validation at
`b94969c` passes ([Development run 37154682802](https://github.com/brianluby/armorer-workflows/actions/runs/37154682802)).
The earlier partial retry rejected retained prior-attempt artifacts. That failure
earns no validation credit; the fresh run preserves exact attempt/asset checks.
The external Momus review-workflow additions are retained in both baselines.

All nine full v0.1 outcomes remain open. Independently accepted production
runtime/Node/root/catalog authority, second-catalog migration, protected whole-job
startup and Apple finalization, current-attempt approval and serialization across
runners, a complete signed rehearsal of Armorer's own producer/consumer flow,
approved provider transition/race trials, Apple parity and both pilots remain
required. The native controller still blocks writes until every required gate is
Enforced and a separate publication approval is supplied; the two current-attempt
gates are now observable through the pinned controller's attempt-bound deployment
(PR32), while a settings-only inspect still reports them unsupported.
No SLSA level, publication or completed release rehearsal is inferred from CI.

Use the [acceptance ledger](v01-acceptance.md) for each ticket's exact gate and
[documentation coverage](documentation-coverage.md) for operational guide limits.
The [preceding source map](../reviews/2026-10-03-development-status-before-reconciliation.txt)
and existing review records remain historical snapshots.

Keep every earned protection. **This is the way.**
