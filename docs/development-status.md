# What stands. What is still being forged.

Source snapshot: **2026-10-02, America/Los_Angeles**, main
`96457ee418439dde097339cfcd374c08f2cc98ad`. PR API observations were made
on 2026-10-03 UTC. Seven implementation PRs (#15–21) are open. The separate
[documentation draft #22](https://github.com/brianluby/armorer/pull/22) supplies
these field guides and review records. Inspect source ancestry before promoting
a feature into the current command guide.

**This is the way.**

## The current main implementation

Main exposes `check`, `catalog`, `plan`, `preview`, `apply`, `recover` and
`schema`. Follow [the command reference](cli-reference.md). Apply manages only
the toolchain. Catalog rendering is a library operation; it does not adopt files.
The offline verifier authenticates individual evidence slots through pinned
native `gh`. It does not provide the complete-release CLI. No SLSA level is claimed.

PRs [#6](https://github.com/brianluby/armorer/pull/6),
[#7](https://github.com/brianluby/armorer/pull/7) and
[#10](https://github.com/brianluby/armorer/pull/10) are incorporated in this main
source. Earlier contract and local transaction slices are also present.

## Merged into a stack is a different boundary

GitHub reports #9 and #11–14 as merged, but their implementation commits are
**not ancestors of this main source**. Those PRs targeted parent branches.
The provisioning PR #8 is closed without a merge record; its original
implementation is included in the #9 stack. Recheck final integration source
before calling these features available on main.

| Stack work | Exact reviewed implementation / guide |
| --- | --- |
| #8 provisioning, #9 upgrades and rollback | [`e184062d61a7`](https://github.com/brianluby/armorer/blob/e184062d61a73a72e7d7fefdedb65523bc138820/docs/upgrades.md), [bootstrap v2](https://github.com/brianluby/armorer/blob/e184062d61a73a72e7d7fefdedb65523bc138820/docs/bootstrap-v2.md) |
| #11 Cargo graph v2 | [`919a94892531`](https://github.com/brianluby/armorer/blob/919a9489253176e15b8bf62b91176fe230c749c2/docs/cargo-graph-v2.md) |
| #12 complete SBOM validation | [`178e9d39b2f3`](https://github.com/brianluby/armorer/blob/178e9d39b2f323042c50e30dd476f9c515112f36/docs/complete-sbom-validation.md) |
| #13 authenticated inventory consumer | [`b133a9288589`](https://github.com/brianluby/armorer/blob/b133a9288589e8e24fcb2f23be3a6fda05f2d0e3/docs/authenticated-inventory.md) |
| #14 explicit historical byte comparison | [`30d26139b4c7`](https://github.com/brianluby/armorer/blob/30d26139b4c7424f62a3fa0f461321bd7c9560d1/docs/historical-verification.md) |

These heads identify original reviewed implementations. Parent branches may have
advanced through later merges; the links are not mutable-branch status claims.

## Work still open

Branch guides are linked by full commit. Passing checks establish evidence for
that candidate; they do not accept the entire stack or complete a release.

| PR | Base | What it proposes | Exact head / guide |
| --- | --- | --- | --- |
| [#15: Modern builder identity](https://github.com/brianluby/armorer/pull/15) | #14 branch | Exact reusable-workflow builder identity and independently expected hosted runner | [`005f08099a2b`](https://github.com/brianluby/armorer/blob/005f08099a2b08681080a4e532b3615f3232d449/docs/offline-verifier.md) |
| [#16: Native catalog and runtime v2](https://github.com/brianluby/armorer/pull/16) | #15 | Archive/member identities, explicit context v2 and bounded common runtime loading | [`c649051a04e4`](https://github.com/brianluby/armorer/blob/c649051a04e49c25885502db65b1aeb6f14a6bba/docs/runtime-native-v2.md) |
| [#17: Current provenance build type](https://github.com/brianluby/armorer/pull/17) | #16 | Exact qualified current build-type URI with strict legacy behavior | [`cd47288b3b17`](https://github.com/brianluby/armorer/blob/cd47288b3b17268af079e882614431f7520caec3/docs/offline-verifier.md) |
| [#18: Complete-release CLI](https://github.com/brianluby/armorer/pull/18) | #17 | Explicit-context verification over the strict offline consumer | [`a5d19efa10b3`](https://github.com/brianluby/armorer/blob/a5d19efa10b3b5596930ad5eedb75ce7c18598a0/docs/verify-release-cli.md) |
| [#19: Native Apple consumer](https://github.com/brianluby/armorer/pull/19) | #18 | Native Developer ID, team, certificate, hardened runtime, timestamp and ticket checks | [`6c0c3d192360`](https://github.com/brianluby/armorer/blob/6c0c3d192360b9dc983533f427babe323eb0baeb/docs/apple-native-verification-v1.md) |
| [#20: Preview helper documentation](https://github.com/brianluby/armorer/pull/20) | main | Comments documenting existing preview helpers and regression tests | [`29c8e6fadf7f`](https://github.com/brianluby/armorer/blob/29c8e6fadf7f2c4e98eb67cbae42938e4e8b2b16/tests/preview.rs) |
| [#21: Combined source](https://github.com/brianluby/armorer/pull/21) | main | Bootstrap, upgrades, preview and strict verification; repaired artifact scope, exact evidence observations and independently approved resolved root features | [`56d7380a5534`](https://github.com/brianluby/armorer/blob/56d7380a5534583b7973b5eedc55a1223b46b60e/docs/source-integration-v1.md) |

Use the separately labelled candidate guides for [bootstrap](candidate-bootstrap.md),
[upgrades and reversal](candidate-upgrades.md), and
[complete/historical verification](candidate-verification.md). #21's new head
repairs the artifact-scope defect identified at `c815a065fb08`; open #15–19 retain
their old heads and missing check. Complete own producer/consumer qualification
and production acceptance remain separate from the repaired source.

## The separate workflow candidate

Workflow main remains `772ca83e386c883c88cc3b936d69f8cb3216c91e`, matching the
embedded catalog pin. The preceding workflow integration PR #21 at
`e88e25fc46c1bb5298579c5cd49f52d43215b9e1` targets that main and preserves the
stack through #20 at `853437cb1c9cbb9a60b2186ebb637ce56d3f6b51`. Its only changes
relative to that stack are three fixture-validator references to Armorer
`56d7380a5534` and seven test-class docstrings. Runtime/schema/pin/action bytes
remain identical. The
[candidate workflow guide](candidate-workflow-prerequisites.md) records fixed
read-only configuration observations, ephemeral read-token isolation and complete
unsigned Apple intake. The [producer evidence guide](candidate-producer-evidence.md)
also covers complete build/policy collection, freshness and private proof boundaries.
The [final payload guide](candidate-final-payloads.md) covers Linux/library byte
transformations and the required unverified inventory/bundle order. The integrated
source's prerequisite/intake runs and all 12 runtime validation jobs pass across
the three supported hosts. Local affected-scope suites pass 85 tests, with one
real scanner/public-feed control skipped. Operational authority stays false.

The [preceding scoped review](../reviews/2026-10-02-workflow-prerequisites.md)
retains its original source and dependency limitation. The
[integration continuation](../reviews/2026-10-03-workflow-integration.md) records
the fresh environment and exact-head hosted evidence. The separate combined
caller rehearsal also passes all three native collection/writer-join jobs for
its fixed nine-selection/eighteen-archive fixture. These helpers are not integrated into
main's CLI/callers; unsigned intake does not implement protected Developer ID
signing, notarization or immutable publication. The production administrative-read
credential binding and current-attempt environment approval remain open.

The reviewed successor `9d857a80f0943ecd06870f89d6bbde968a207625` changes seven
files: source/controller/worker process-group cleanup, a nonempty policy-schema
CI check, two regression suites and one caller-pin documentation correction.
Schemas, tool pins, action wrappers, lock and rehearsal callers are unchanged.
Its affected local suites pass 48 tests; four actual descendant regressions fail
against the predecessor and pass at the successor. All prerequisite/intake jobs
and all 12 runtime-validation jobs pass at the new head. Its combined caller run
fails Linux x64/ARM collection while macOS passes, despite successful build/policy
siblings. The [cleanup review](../reviews/2026-10-03-workflow-cleanup.md) retains
these exact outcomes; the prior combined success does not qualify this run.
The next reviewed successor `3fac9a1cef3d9a5c4ac6f5c79c99d9255f63200a` changes
only the issuer helper and its two Node test files. It rejects four inherited
startup transport variables before OIDC credential reads/HTTP. Review and local
compatibility evidence are in the same continuation; each new hosted run retains
its own result. All three prerequisite and unsigned-intake jobs pass at
`3fac9a1cef3d`; the formerly queued runtime-validation run now passes all 12
jobs and the same combined run passes all 33 jobs, including all three native
collection/writer joins. The [transport continuation](../reviews/2026-10-03-workflow-transport-cleanup.md)
retains that terminal receipt and a reproduced shared-adapter cleanup fault at
`3fac9a1cef3d`: its leader-status guard leaves a pipe-holding descendant running
in an owned timeout control. Successor `b2b9fbc9846d5aa4bf1dbdb8e8618bddad25e23c`
repairs that shared guard and adds the base-adapter regression in a two-file patch.
Its 32 affected local tests, separate prerequisite/intake jobs and all 12 runtime
jobs pass. Its distinct combined run `37108079402` now finishes successfully
with all 33 jobs and all three native writer joins; preserve the earlier live
observation separately.

## Integrate with care

The [original review packet](../reviews/2026-10-02-open-prs.md) retains the first
13-head review and its exact conflict comparisons. The
[continuation review](../reviews/2026-10-02-pr-review-continuation.md) covers #19,
#20 and the changed merge boundary. The
[combined-source review](../reviews/2026-10-02-integration-pr21.md) covers #21.
The [successor review](../reviews/2026-10-02-integration-pr21-successor.md) records
its later repair without rewriting that first snapshot.
The [test-only continuation](../reviews/2026-10-02-pr21-context-ordering.md)
records the latest exact context-error ordering regression.
The [resolved-feature continuation](../reviews/2026-10-02-pr21-resolved-features.md)
records the explicit native-v3 successor, exact root-feature comparison and a
confirmed contributor schema-loop typo at the newer head. The candidate exposes
24 schemas; main retains 14. Older contexts reject default/expanded feature sets
instead of choosing an expectation from offered graphs.
The combined candidate reconciles the original structural conflicts and assigns
distinct ADRs 0009, 0010 and 0011. Main still lacks those integrated interfaces.

Retain preview, catalog, bootstrap/upgrade and verification interfaces together.
Retain each stack's schema, workflow-audit and genuine native integration gates.
Resolve open source findings before accepting the integrated set.

Keep every earned protection. **This is the way.**

## Earn the release

Documentation head `8f6982006d0a55833b85c54da01964e9834248fa` passed all three
Development jobs in
[run 37108554559](https://github.com/brianluby/armorer/actions/runs/37108554559),
including genuine-signature fixtures, schema checks and workflow audits. That
receipt covers the documented main runtime and training additions at that exact
head. It does not accept #21 or qualify a complete signed release.

Open #15–20 retain passing hosted Rust checks. #21's source-repair local suite at
`3a21085a5864` passes 205 ordinary tests plus eight doctests; seven native
integrations remain ignored locally. Its later test-only head `aff8c041c410`
passes both integrated CLI tests locally. All three native hosted Rust jobs in
[run 37101218059](https://github.com/brianluby/armorer/actions/runs/37101218059)
report success at that head.
That receipt belongs to `aff8c041c410`; the later resolved-feature head
`0ef1145bf7d6` passes 30 focused graph/context/CLI tests and 25 trust-contract
tests locally, but its ARM hosted bootstrap suite fails with transaction
contention in [run 37102828787](https://github.com/brianluby/armorer/actions/runs/37102828787).
The [feature review](../reviews/2026-10-02-pr21-resolved-features.md) records the
failure and contributor schema-loop finding. Its successor `56d7380a5534` binds
independently expected observation times and coverage exception use, and explicitly
unlocks completed transactions even when duplicate descriptors remain open.
It passes 73 affected-scope local tests and all three hosted Rust jobs in
[run 37104573003](https://github.com/brianluby/armorer/actions/runs/37104573003).
The [latest continuation](../reviews/2026-10-03-pr21-evidence-and-locks.md) retains
that fresh receipt alongside the earlier failure; the contributor typo remains.
Several
automated reviews failed or skipped work; they supply no approval. The new Apple
consumer's hosted reference checks do not implement protected Apple production.
Its online ticket request does not guarantee a fresh service response or
authenticate a producer submission UUID. Linux cannot verify Apple executables;
macOS library source packages do not need Developer ID signing.

Production context/catalog/root approval, the final-byte producer, a complete own
signed non-publishing rehearsal, protected Apple finalization, immutable
publication, both pilots and human acceptance remain separate gates.
The repaired candidate binds platform evidence to the selected final artifact.
That regression does not establish the missing genuine complete own signed
producer/consumer acceptance. Historical byte matching cannot substitute for
authenticated provenance.
v0.1 targets SLSA Build L2; L3 stays on the future-version backlog.

Hold each gate until its evidence stands. **This is the way.**
