# Forge the guides alongside the tools

Give each reader a path they can follow. Verify it against the source.
Keep future work visible until its implementation and evidence are ready.
**This is the way.**

This map follows [the implementation plan's documentation requirements](../IMPLEMENTATION_PLAN.md#documentation-and-recovery-deliverables)
and ticket [#14, database ID 1317](https://kanban.luby.us/tasks/1317).
It is a local delivery plan, not a tracker status update or ticket closeout.
The [development map](development-status.md) identifies the source boundary.

## Guides for the current source

| Reader's duty | Guide | Validation boundary |
| --- | --- | --- |
| Begin a safe inspection | [Onboarding](onboarding.md) | Temporary CLI fixture; unchanged consumer inputs and expected findings |
| Inspect every profile and preserve an existing project | [Profiles](profiles.md), [training workspace](../examples/profile-workspace/README.md) | Two packages, library/CLI/service, four target selections, explicit service features; read-only inspection |
| Choose commands and interpret results | [CLI reference](cli-reference.md) | Current help, errors and finding codes |
| Correct failed inspection or local transaction | [Troubleshooting](troubleshooting.md) | Current error categories, feature/path/preview bounds and preserved recovery evidence |
| Know version and host boundaries | [Compatibility](compatibility.md) | Exact v1/runtime rules, target selections versus native verifier hosts; no invented support window |
| Review, apply and recover | [Apply and recovery](apply.md) | v1 toolchain-only transaction, exact preview and retained digest/journal |
| Understand intent and preview identity | [Local contracts](contracts-v1.md) | Current types, schemas and semantic rules |
| Inspect bootstrap authority | [Catalog](bootstrap-catalog.md) | Embedded identity and library rendering; no automatic adoption |
| Understand release evidence | [Trust contracts](trust-contracts-v1.md), [examples](../examples/trust-v1/README.md) | Structural/semantic interfaces; synthetic labels retained |
| Verify a scoped attestation | [Offline verifier](offline-verifier.md) | Library API; independent policy/roots; genuine native qualification separate |
| Prepare for release gates | [Release readiness](release-readiness.md) | Required evidence and open producer/publication/pilot gates |
| Inspect account/visibility and manual settings | [Platform prerequisites](platform-prerequisites.md) | Current GitHub documentation; manual read-only settings sample; no authenticated preflight claim |
| Renew evidence and prepare failure decisions | [Evidence maintenance](evidence-maintenance.md), [release recovery](release-recovery.md), [incident template](release-incident-template.md) | Current consistency/freshness checks plus explicitly labelled backend design; no remote recovery claim |
| Contribute in the creed | [Contributing](../CONTRIBUTING.md), [writing guide](writing-guide.md) | Pinned checks, all schemas and accurate Mandalorian prose |

These guides cover the current boundary. They do not complete ticket #14's
future operational scope or hosted Armorer dogfooding.

## Next guides and their gates

| Planned deliverable | Preparation now | Gate before a supported operator runbook |
| --- | --- | --- |
| New/existing adoption for library, CLI and service | [Candidate bootstrap and recovery](candidate-bootstrap.md) at #21's exact head; all three integrated profile regressions and temporary guide procedures pass | Reviewed integration and accepted provisioning; local configuration is not release evidence |
| Upgrade, migration and exact rollback | [Candidate upgrades](candidate-upgrades.md); integrated CLI and forward/reverse fault tests | Accepted integration; actual migration between two independently accepted catalogs |
| Complete-release and historical verification | [Candidate verification](candidate-verification.md); explicit context/mode routing and repaired subject binding at `3a21085a5864` | Genuine complete positive/negative controls and independent acceptance |
| Native Apple consumer checks | Exact #21 consumer contract, limits and hosted native receipt | Accepted integration; protected production signing remains separate |
| Account/visibility matrix and manual settings | Source-checked matrix and manual observation guide | Qualified preflight plus repository/account-specific evidence |
| Credential names, scopes, approvals and rotation | Preserve explicit ownership/permissions | Reviewed adapters define exact names and scopes |
| Advisory/root outages and resumption | [Evidence maintenance](evidence-maintenance.md): online preparation, offline import, clock boundaries and no fallback | Accepted policy and executable authenticated outage/retry paths |
| Draft conflict, partial upload and alias recovery | [Release recovery](release-recovery.md): identity decisions, receipt checks and pending alias adapter | Owned authentic receipts, backend and rehearsed faults |
| Post-publication incident/new version | [Recovery](release-recovery.md#respond-to-a-published-failure), [incident record](release-incident-template.md) | Verified publication backend, concrete authorization and operational acceptance |
| Support/deprecation/security policies and dogfooding | Current compatibility limits, reproducer guidance and [security reporting draft](security-reporting-draft.md) | Accepted support commitments, a working private reporting channel/policy and hosted Armorer rehearsal |

Do the preparation the source supports. Keep unsupported commands out of the
main walkthrough. **This is the way.**

The [original combined-source review](../reviews/2026-10-02-integration-pr21.md)
records #21's structural resolution and trust finding. The
[successor review](../reviews/2026-10-02-integration-pr21-successor.md) records its
source fix, regressions and remaining acceptance gates. Candidate guides are
reviewable preparation. Promote them into the
current-source walkthrough only after accepted integration and renewed source,
command and evidence checks. Keep the old receipts.

## Close each documentation loop

Inspect the accepted commit and CLI help after implementation changes. Update
commands, source boundary, expected outputs and recovery. Run copyable procedures
in an isolated temporary consumer. Compare discovery inputs before/after; check
links, schema names and shell syntax.

Retain exact source/run/policy identities for new validation. Preserve old
receipts. Read back authorized Veans writes and record both project index and
database ID. A local guide establishes no tracker closure, human acceptance or
published release.

Make the guide earn the reader's trust. **This is the way.**
