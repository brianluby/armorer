# Account for every guide

Review snapshot: 2026-10-03 America/Los_Angeles / UTC. This audit follows
[the complete documentation requirements](../IMPLEMENTATION_PLAN.md#documentation-and-recovery-deliverables)
and [ticket #14's local acceptance map](../IMPLEMENTATION_PLAN.md#14-open-source-onboarding-upgrades-and-recovery-runbooks).
It is not a tracker update or ticket closure.

Source baseline `5e8e2b7fd84a4b6a39650278ae14dbc7cae0afce` and workflow baseline
`b94969c7030b8c1105882a1374ed4d7a3eab653a` retain the accepted integrations.
The current command guide covers bootstrap, upgrade, complete/historical
verification and publication inspection; native publication remains gated.

Count what stands. Keep the missing proof visible. **This is the way.**

## Requirements and their evidence

“Validated” below identifies the stated local/source scope. “Prepared” identifies
reviewable design or candidate guidance with outstanding operational acceptance.
Neither word supplies production qualification.

| Explicit deliverable | Guide / present evidence | Remaining requirement |
| --- | --- | --- |
| MIT license and contribution policy | [LICENSE](../LICENSE), [contributor guide](../CONTRIBUTING.md); exact main checks and all 27 schema exporters | No new license decision pending; final release acceptance remains separate |
| Mandalorian voice and frequent creed | [Writing guide](writing-guide.md); resolute openings/checkpoints, exact technical identifiers and repeated “This is the way” throughout behavior guides | Retain this voice in future behavior changes; preserve historical receipts |
| Onboarding for library, CLI and service | [Onboarding](onboarding.md), [profiles](profiles.md), [workspace example](../examples/profile-workspace/README.md); validated metadata selections and temporary native fixture builds | Accepted full packaging/release adoption remains pending |
| New versus existing adoption | [Profiles](profiles.md#bring-an-existing-project), [candidate bootstrap](candidate-bootstrap.md), [candidate upgrades](candidate-upgrades.md); unowned/owned conflicts and preserved custom files tested | Implementation is accepted; qualify adopter-specific prerequisites |
| Public/private account and capability matrix | [Platform guide](platform-prerequisites.md); primary GitHub sources and explicit adapter limitations | Authenticated preflight, private-positive qualification and repository-specific effective settings |
| Manual settings and required scopes | [Platform guide](platform-prerequisites.md#inspect-release-immutability), [candidate observer](candidate-workflow-prerequisites.md); Administration-read endpoint, exact prerequisite states and bounded native qualification | Accepted production scoped identity integration and fresh observations near publication; current-attempt approval remains unsupported |
| Exact credential names and local setup without collection | [Credential boundaries](credential-boundaries.md); current no-secret CI/build; exact candidate Python/Node read variables, separate platform runtime/OIDC service names and private-proof expiry | Production administrative-read, signing and publishing credential names/provider setup remain undefined |
| Apple approvals and rotation | [Credential boundaries](credential-boundaries.md#review-signing-approval-before-releasing-credentials), [unsigned intake](candidate-workflow-prerequisites.md); complete frozen handoff and decision sequence prepared | Accepted protected signing adapter, exact secret bindings, effective approvals and hosted production rotation/handoff rehearsal |
| Check/plan/apply examples | [Main CLI](cli-reference.md), [onboarding](onboarding.md), [apply](apply.md); validated read-only discovery, exact preview, digest/ownership/recovery boundaries | Version-one apply is toolchain-only; bootstrap requires its separately approved five-file packet |
| Upgrade/migration examples | [Candidate upgrades](candidate-upgrades.md); all-profile integrated same-catalog migration and preserved customization/reversal tests | Actual two-accepted-catalog migration evidence |
| Exact-byte verification and per-artifact claims | [Trust contracts](trust-contracts-v1.md), [candidate verifier](candidate-verification.md), [final payloads](candidate-final-payloads.md); independent authority, exact artifact/observation/root-feature scope, fixed measured transformations and detached inventory proof | Own complete genuine signed positive/negative producer/consumer qualification and production authority |
| Online/offline roots | [Evidence maintenance](evidence-maintenance.md); current primary root-export command, offline transport limits and independent policy/context renewal | Owner-approved production root export/import/renewal and applicable Apple online limits |
| Explicit historical policy | [Candidate verification](candidate-verification.md#compare-historical-bytes-only-by-explicit-decision); exact source allowlist, weaker result and no modern fallback | Independently approved real historical identities |
| Advisory outage behavior | [Evidence maintenance](evidence-maintenance.md#hold-the-gate-during-an-outage), [candidate producer evidence](candidate-producer-evidence.md); pinned CI fetch gate and candidate complete handoff/source/advisory freshness checks | Qualified authenticated release controller and resumption; unsigned consistency alone cannot establish protected production behavior |
| Partial draft retry/conflict handling | [Release recovery](release-recovery.md#choose-the-recovery-branch); current receipt identity guards plus prepared decisions | Authenticated owned receipts, remote backend, serialized preconditions and fault/race qualification |
| Post-publication incident/new-version recovery | [Release recovery](release-recovery.md#respond-to-a-published-failure), [incident record](release-incident-template.md); original evidence preserved, corrected version separate | Qualified backend, concrete owner-authorized incident actions and operational exercise |
| Alias update failure | [Release recovery](release-recovery.md#resume-only-the-authorized-alias-change); separate immutable-release and alias evidence | Accepted alias adapter, newest-stable decision, authorization and exact readback/race tests |
| Upgrade rollback | [Candidate upgrades](candidate-upgrades.md#recover-an-interruption-or-review-a-reversal); separate reverse approval and journal fault tests | Operational qualification; restored bytes remain distinct from historical authenticity/readiness |
| Version/support/deprecation policy | [Compatibility](compatibility.md), [support-policy draft](support-policy-draft.md); experimental v0.1, exact runtime/schema boundary, proposed maintenance/deprecation convention and explicit unqualified minimum-system baseline | Maintainer-approved released support/deprecation commitments; no invented support window |
| Published security policy | [Security draft](security-reporting-draft.md); reporting scope, evidence and privacy prepared | Working private channel and owner acceptance before root SECURITY.md publication |
| Armorer dogfooding before public release | [Dogfooding](dogfooding.md), [own intent example](../examples/armorer-dogfood/README.md); own-source read-only inspection validated | Accepted hosted own producer/signing/consumer rehearsal and human acceptance; no published release exists at this snapshot |

Every named operational subject has a current guide or explicitly labelled
preparation. The outstanding column is part of the requirement, not an optional
appendix. The documentation goal and ticket #14 closeout are **not proven complete**.

Earn the last column. **This is the way.**


The [accepted source map](development-status.md) and [live acceptance ledger](v01-acceptance.md)
separate merged implementation from the remaining operational gates. Preserve the
[preceding documentation snapshot](../reviews/2026-10-03-documentation-coverage-before-reconciliation.txt)
and existing dated review receipts. PR #26 remains under separate human review.
