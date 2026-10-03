# Account for every guide

Review snapshot: 2026-10-02 America/Los_Angeles / 2026-10-03 UTC. This audit follows
[the complete documentation requirements](../IMPLEMENTATION_PLAN.md#documentation-and-recovery-deliverables)
and [ticket #14's local acceptance map](../IMPLEMENTATION_PLAN.md#14-open-source-onboarding-upgrades-and-recovery-runbooks).
It is not a tracker update or ticket closure.

Main is `96457ee418439dde097339cfcd374c08f2cc98ad`; the combined candidate is
`aff8c041c410c79674c484fc29b8adbd2ca727b9`. The embedded workflow foundation is
`brianluby/armorer-workflows@772ca83e386c883c88cc3b936d69f8cb3216c91e`.
Candidate guides remain separate from main's commands. Preserve source identities
when these boundaries change.

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
| Manual settings and required scopes | [Platform guide](platform-prerequisites.md#inspect-release-immutability); documented Administration-read endpoint and unknown/403 behavior | Accepted scoped identity adapter and fresh authenticated observations near publication |
| Exact credential names and local setup without collection | [Credential boundaries](credential-boundaries.md); pinned CI/build source needs no provisioned secrets, manual admin-read uses owner's configured identity | Exact names/provider setup for future signing, preflight and publishing adapters remain undefined |
| Apple approvals and rotation | [Credential boundaries](credential-boundaries.md#review-signing-approval-before-releasing-credentials); decision sequence and protected-environment duties prepared | Accepted production adapter, exact secret bindings, effective approvals and hosted rotation/handoff rehearsal |
| Check/plan/apply examples | [Main CLI](cli-reference.md), [onboarding](onboarding.md), [apply](apply.md); validated read-only discovery, exact preview, digest/ownership/recovery boundaries | No multi-file authority is implied by main's toolchain-only plan |
| Upgrade/migration examples | [Candidate upgrades](candidate-upgrades.md); all-profile integrated same-catalog migration and preserved customization/reversal tests | Accepted CLI integration and actual two-accepted-catalog migration evidence |
| Exact-byte verification and per-artifact claims | [Trust contracts](trust-contracts-v1.md), [offline verifier](offline-verifier.md), [candidate complete verifier](candidate-verification.md); independent authority, scope and no subset/fallback rules | Own complete genuine signed positive/negative producer/consumer qualification and production authority |
| Online/offline roots | [Evidence maintenance](evidence-maintenance.md); current primary root-export command, offline transport limits and independent policy/context renewal | Owner-approved production root export/import/renewal and applicable Apple online limits |
| Explicit historical policy | [Candidate verification](candidate-verification.md#compare-historical-bytes-only-by-explicit-decision); exact source allowlist, weaker result and no modern fallback | Accepted integrated command and independently approved real historical identities |
| Advisory outage behavior | [Evidence maintenance](evidence-maintenance.md#hold-the-gate-during-an-outage); pinned CI freshly fetches feeds and enforces offline checks/freshness | Qualified release evidence/controller resumption; local semantic checks alone cannot establish live CI behavior |
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

Main's unchanged Rust source has the earlier formatting, strict Clippy, locked
build and 95-test receipt. The source-repair candidate at `3a21085a5864` passes
205 ordinary tests/eight doctests; seven native integrations are ignored locally.
The later test-only `aff8c041c410` head passes both integrated CLI tests locally
and its distinct hosted Linux x64/Linux ARM/macOS jobs pass. The
[successor review](../reviews/2026-10-02-integration-pr21-successor.md) keeps those
receipts separate from complete producer/publication acceptance; the
[ordering continuation](../reviews/2026-10-02-pr21-context-ordering.md) records
the latest test-only diff and exact-head run.

The new own-source inspection exports documentation head
`b2c50c216830cc8f1c9e3627e7b292f09220354a`, whose runtime source matches main,
copies only explicit training intent into the temporary project, and validates
one package/binary, one feature case and three target selections. Plan/check have
the documented 0/2 statuses and four open findings. All 172 copied files match
the original archive plus supplied config; no build output or consumer mutation
was created. This is discovery evidence, not a signed release rehearsal.

The expanded guide set passes checks for 41 Markdown files, local file/heading
links, balanced fences, shell syntax and the parsed dogfood TOML. This validation
checks documentation structure and the described current exercise; it does not
exercise a missing protected release adapter.

Pinned workflow source was read through GitHub's API, including its separately
pinned nested CI workflow. The credential guide records the actual no-secret,
contents-read interfaces. No credential value, provider setting or pilot checkout
was inspected or changed. Armorer's release list is empty; private vulnerability
reporting still returns disabled. The pending channel question remains unresolved.

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
