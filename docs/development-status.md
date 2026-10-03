# What stands. What is still being forged.

Reconciliation snapshot: 2026-10-03 UTC. The source baseline is Armorer
`5e8e2b7fd84a4b6a39650278ae14dbc7cae0afce`; the workflow baseline is `b94969c7030b8c1105882a1374ed4d7a3eab653a`.
Both retain the five human-approved integration merges below. Their latest
push validation passes. **This is the way.**

## Accepted implementation

| Repository / PR | Accepted merge | Scope |
| --- | --- | --- |
| Armorer [#21](https://github.com/brianluby/armorer/pull/21) | `e866a92ea8e4003f6f0d47222ad7341bdac839d7` | Bootstrap, upgrade, complete/historical consumer and exact evidence scope |
| Workflows [#21](https://github.com/brianluby/armorer-workflows/pull/21) | `0f94f5c2abe8136c22540d881f92d0ad85f9753a` | Integrated unprivileged build/policy/producer and unsigned intake foundations |
| Armorer [#24](https://github.com/brianluby/armorer/pull/24) | `d70ef3d977de4bfe0194fd843365a17036a1c811` | Owned-draft controller and fail-closed native capability observations |
| Workflows [#22](https://github.com/brianluby/armorer-workflows/pull/22) | `c2909ebc19e378c68265b49f9816fbff9a53003d` | Native Node signature qualification; production authority remains separate |
| Armorer [#25](https://github.com/brianluby/armorer/pull/25) | `7cb4a3ec76f7bbe8fb0274bca1b59e4d1215e391` | Readiness/onboarding and scoped acceptance evidence |

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

The new controller [#26](https://github.com/brianluby/armorer/pull/26) remains
separate at `deb69d20d35347c9bc2f7b21dd39c79cf0b3aac6`. It rejects rerun-initiator
self-review and explicitly releases completed state locks. Its 244 local tests,
three platform checks and CodeRabbit review pass; human review remains pending.
It is not included in this documentation acceptance.

## Hosted evidence and remaining gates

Source [Development run 37154673178](https://github.com/brianluby/armorer/actions/runs/37154673178)
passes Linux x64, Linux ARM64 and macOS. All eight workflow push runs pass,
including fresh [Development run 37154682802](https://github.com/brianluby/armorer-workflows/actions/runs/37154682802).
The earlier partial retry rejected retained prior-attempt artifacts. That failure
earns no validation credit; the fresh run preserves exact attempt/asset checks.
The external Momus review-workflow additions are retained in both baselines.

All nine full v0.1 outcomes remain open. Independently accepted production
runtime/Node/root/catalog authority, second-catalog migration, protected whole-job
startup and Apple finalization, current-attempt approval and serialization across
runners, a complete signed rehearsal of Armorer's own producer/consumer flow,
approved provider transition/race trials, Apple parity and both pilots remain
required. The native controller still blocks writes on unsupported required gates.
No SLSA level, publication or completed release rehearsal is inferred from CI.

Use the [acceptance ledger](v01-acceptance.md) for each ticket's exact gate and
[documentation coverage](documentation-coverage.md) for operational guide limits.
The [preceding source map](../reviews/2026-10-03-development-status-before-reconciliation.txt)
and existing review records remain historical snapshots.

Keep every earned protection. **This is the way.**
