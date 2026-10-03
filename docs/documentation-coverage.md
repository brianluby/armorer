# Account for every guide

Review snapshot: 2026-10-02 America/Los_Angeles / 2026-10-03 UTC. This audit follows
[the complete documentation requirements](../IMPLEMENTATION_PLAN.md#documentation-and-recovery-deliverables)
and [ticket #14's local acceptance map](../IMPLEMENTATION_PLAN.md#14-open-source-onboarding-upgrades-and-recovery-runbooks).
It is not a tracker update or ticket closure.

Main is `96457ee418439dde097339cfcd374c08f2cc98ad`; the combined candidate is
`56d7380a5534583b7973b5eedc55a1223b46b60e`. The embedded workflow foundation is
`brianluby/armorer-workflows@772ca83e386c883c88cc3b936d69f8cb3216c91e`.
Candidate guides remain separate from main's commands. Preserve source identities
when these boundaries change.
The separately reviewed workflow integration candidate is
`e88e25fc46c1bb5298579c5cd49f52d43215b9e1`; its observer and unsigned intake do
not change the embedded workflow pin or grant protected production authority.

Count what stands. Keep the missing proof visible. **This is the way.**

## Requirements and their evidence

“Validated” below identifies the stated local/source scope. “Prepared” identifies
reviewable design or candidate guidance with outstanding operational acceptance.
Neither word supplies production qualification.

| Explicit deliverable | Guide / present evidence | Remaining requirement |
| --- | --- | --- |
| MIT license and contribution policy | [LICENSE](../LICENSE), [contributor guide](../CONTRIBUTING.md); exact main checks and all 14 schema exporters | No new license decision pending; final release acceptance remains separate |
| Mandalorian voice and frequent creed | [Writing guide](writing-guide.md); resolute openings/checkpoints, exact technical identifiers and repeated “This is the way” throughout behavior guides | Retain this voice in future behavior changes; preserve historical receipts |
| Onboarding for library, CLI and service | [Onboarding](onboarding.md), [profiles](profiles.md), [workspace example](../examples/profile-workspace/README.md); validated metadata selections and temporary native fixture builds | Accepted full packaging/release adoption remains pending |
| New versus existing adoption | [Profiles](profiles.md#bring-an-existing-project), [candidate bootstrap](candidate-bootstrap.md), [candidate upgrades](candidate-upgrades.md); unowned/owned conflicts and preserved custom files tested | Promote candidate commands only after accepted integration; qualify adopter-specific prerequisites |
| Public/private account and capability matrix | [Platform guide](platform-prerequisites.md); primary GitHub sources and explicit adapter limitations | Authenticated preflight, private-positive qualification and repository-specific effective settings |
| Manual settings and required scopes | [Platform guide](platform-prerequisites.md#inspect-release-immutability), [candidate observer](candidate-workflow-prerequisites.md); Administration-read endpoint, exact prerequisite states and bounded native qualification | Accepted production scoped identity integration and fresh observations near publication; current-attempt approval remains unsupported |
| Exact credential names and local setup without collection | [Credential boundaries](credential-boundaries.md); current CI/build needs no provisioned secrets; candidate qualification uses ephemeral `ARMORER_READ_TOKEN` and isolated `GH_TOKEN` | Production administrative-read, signing and publishing credential names/provider setup remain undefined |
| Apple approvals and rotation | [Credential boundaries](credential-boundaries.md#review-signing-approval-before-releasing-credentials), [unsigned intake](candidate-workflow-prerequisites.md); complete frozen handoff and decision sequence prepared | Accepted protected signing adapter, exact secret bindings, effective approvals and hosted production rotation/handoff rehearsal |
| Check/plan/apply examples | [Main CLI](cli-reference.md), [onboarding](onboarding.md), [apply](apply.md); validated read-only discovery, exact preview, digest/ownership/recovery boundaries | No multi-file authority is implied by main's toolchain-only plan |
| Upgrade/migration examples | [Candidate upgrades](candidate-upgrades.md); all-profile integrated same-catalog migration and preserved customization/reversal tests | Accepted CLI integration and actual two-accepted-catalog migration evidence |
| Exact-byte verification and per-artifact claims | [Trust contracts](trust-contracts-v1.md), [candidate verifier](candidate-verification.md), [final payloads](candidate-final-payloads.md); independent authority, exact artifact/observation/root-feature scope, fixed measured transformations and detached inventory proof | Own complete genuine signed positive/negative producer/consumer qualification and production authority |
| Online/offline roots | [Evidence maintenance](evidence-maintenance.md); current primary root-export command, offline transport limits and independent policy/context renewal | Owner-approved production root export/import/renewal and applicable Apple online limits |
| Explicit historical policy | [Candidate verification](candidate-verification.md#compare-historical-bytes-only-by-explicit-decision); exact source allowlist, weaker result and no modern fallback | Accepted integrated command and independently approved real historical identities |
| Advisory outage behavior | [Evidence maintenance](evidence-maintenance.md#hold-the-gate-during-an-outage), [candidate producer evidence](candidate-producer-evidence.md); pinned CI fetch gate and candidate complete handoff/source/advisory freshness checks | Qualified authenticated release controller and resumption; unsigned consistency alone cannot establish protected production behavior |
| Partial draft retry/conflict handling | [Release recovery](release-recovery.md#choose-the-recovery-branch); current receipt identity guards plus prepared decisions | Authenticated owned receipts, remote backend, serialized preconditions and fault/race qualification |
| Post-publication incident/new-version recovery | [Release recovery](release-recovery.md#respond-to-a-published-failure), [incident record](release-incident-template.md); original evidence preserved, corrected version separate | Qualified backend, concrete owner-authorized incident actions and operational exercise |
| Alias update failure | [Release recovery](release-recovery.md#resume-only-the-authorized-alias-change); separate immutable-release and alias evidence | Accepted alias adapter, newest-stable decision, authorization and exact readback/race tests |
| Upgrade rollback | [Candidate upgrades](candidate-upgrades.md#recover-an-interruption-or-review-a-reversal); separate reverse approval and journal fault tests | Accepted integration; restored bytes remain distinct from historical authenticity/readiness |
| Version/support/deprecation policy | [Compatibility](compatibility.md); experimental v0.1, exact runtime/schema boundary, supported metadata targets versus verifier hosts | Maintainer-approved released support/deprecation commitments; no invented support window |
| Published security policy | [Security draft](security-reporting-draft.md); reporting scope, evidence and privacy prepared | Working private channel and owner acceptance before root SECURITY.md publication |
| Armorer dogfooding before public release | [Dogfooding](dogfooding.md), [own intent example](../examples/armorer-dogfood/README.md); own-source read-only inspection validated | Accepted hosted own producer/signing/consumer rehearsal and human acceptance; no published release exists at this snapshot |

Every named operational subject has a current guide or explicitly labelled
preparation. The outstanding column is part of the requirement, not an optional
appendix. The documentation goal and ticket #14 closeout are **not proven complete**.

Earn the last column. **This is the way.**

## Retained validation boundary

Documentation head `375298dfbf87d621c7d88306008fe93d58970746` passes formatting,
strict Clippy, locked build and 95 local tests, with one genuine Sigstore test
ignored locally. Its Linux x64, Linux ARM and macOS Development jobs all pass in
[run 37105227772](https://github.com/brianluby/armorer/actions/runs/37105227772),
including genuine-signature fixtures, schema checks and workflow audits. This
receipt belongs to [draft #22](https://github.com/brianluby/armorer/pull/22), whose
main runtime source is unchanged; it supplies no candidate-integration or complete
signed-release acceptance. The source-repair candidate at `3a21085a5864` passes
205 ordinary tests/eight doctests; seven native integrations are ignored locally.
The later test-only `aff8c041c410` head passes both integrated CLI tests locally
and its distinct hosted Linux x64/Linux ARM/macOS jobs pass. The
[successor review](../reviews/2026-10-02-integration-pr21-successor.md) keeps those
receipts separate from complete producer/publication acceptance; the
[ordering continuation](../reviews/2026-10-02-pr21-context-ordering.md) records
the latest test-only diff and exact-head run.

The later `0ef1145bf7d6fc8a92a276ace0fe13dc6dab0baa` successor adds explicit v3
resolved-feature authority. Its focused graph, release-context and integrated CLI
suites pass 30 tests; three native integrations remain ignored in those local
suites. All 23 earlier schema files are byte-identical and the added v3 exporter
matches the committed schema. The
[feature review](../reviews/2026-10-02-pr21-resolved-features.md) records that scope
and a contributor schema-loop typo separately from operational acceptance. Its
25 trust-contract tests also pass locally. That candidate's hosted ARM
bootstrap test fails with transaction contention; its earlier heads' green runs
do not qualify this successor.

The next successor `56d7380a5534583b7973b5eedc55a1223b46b60e` tightens exact
tool/database observation and coverage-exception binding and releases transaction
locks explicitly before descriptor closure. Its apply/bootstrap/upgrade/trust
suites pass 73 tests locally. All three hosted Rust jobs pass in
[run 37104573003](https://github.com/brianluby/armorer/actions/runs/37104573003).
The [evidence and lock continuation](../reviews/2026-10-03-pr21-evidence-and-locks.md)
retains that new receipt; it does not erase the earlier failure, fix the remaining
contributor typo or qualify a complete own signed release.

The new own-source inspection exports documentation head
`b2c50c216830cc8f1c9e3627e7b292f09220354a`, whose runtime source matches main,
copies only explicit training intent into the temporary project, and validates
one package/binary, one feature case and three target selections. Plan/check have
the documented 0/2 statuses and four open findings. All 172 copied files match
the original archive plus supplied config; no build output or consumer mutation
was created. This is discovery evidence, not a signed release rehearsal.

The expanded guide set passes checks for 49 Markdown files, 328 local links
(including 33 heading links), balanced fences and 29 shell blocks. The retained
parsed dogfood TOML validation is unchanged. This validation
checks documentation structure and the described current exercise; it does not
exercise a missing protected release adapter.

Pinned workflow source was read through GitHub's API, including its separately
pinned nested CI workflow. The credential guide records the actual no-secret,
contents-read interfaces. No credential value, provider setting or pilot checkout
was inspected or changed. Armorer's release list is empty; private vulnerability
reporting still returns disabled. The pending channel question remains unresolved.

The preceding workflow candidate's `853437cb1c9c` results and local import failure
remain in the [dated review](../reviews/2026-10-02-workflow-prerequisites.md).
The integration successor `e88e25fc46c1` preserves runtime/schema/pin/action bytes
and updates three fixture-validator references. In a new temporary environment
using hash-pinned development dependencies, 85 assembly/intake/collector/policy/
capability/build-step tests pass; one real scanner/public-feed control is skipped.
Its prerequisite/intake native runs and all 12 runtime-validation jobs pass at
that exact head. The [integration review](../reviews/2026-10-03-workflow-integration.md)
retains the distinct combined caller run, whose three native collection/writer-join
jobs pass for the fixed nine-selection/eighteen-archive fixture; no complete own signed
rehearsal or production approval is inferred from these scoped qualifications.

## Next actions require their own evidence

When a source adapter or accepted integration arrives, update its command/setup
guide from that exact commit and run the relevant copyable procedure in an
isolated consumer. Retain dated source/run/policy/root identities and fault results.
When the owner selects a working private reporting channel and accepts support
commitments, promote the corresponding policy with exact names and no promised
SLA/window beyond that decision.

Tracker access was unavailable in the agent environment during the initial
documentation review; no live ticket status or authorized readback is inferred
from this local map. No source PR was merged, release published, tag moved,
credential collected or pilot altered by this documentation work.

Keep the plan whole. Let evidence close it. **This is the way.**
