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
| Choose commands and interpret results | [CLI reference](cli-reference.md) | Current help, errors and finding codes |
| Review, apply and recover | [Apply and recovery](apply.md) | v1 toolchain-only transaction, exact preview and retained digest/journal |
| Understand intent and preview identity | [Local contracts](contracts-v1.md) | Current types, schemas and semantic rules |
| Inspect bootstrap authority | [Catalog](bootstrap-catalog.md) | Embedded identity and library rendering; no automatic adoption |
| Understand release evidence | [Trust contracts](trust-contracts-v1.md), [examples](../examples/trust-v1/README.md) | Structural/semantic interfaces; synthetic labels retained |
| Verify a scoped attestation | [Offline verifier](offline-verifier.md) | Library API; independent policy/roots; genuine native qualification separate |
| Prepare for release gates | [Release readiness](release-readiness.md) | Required evidence and open producer/publication/pilot gates |
| Contribute in the creed | [Contributing](../CONTRIBUTING.md), [writing guide](writing-guide.md) | Pinned checks, all schemas and accurate Mandalorian prose |

These guides cover the current boundary. They do not complete ticket #14's
future operational scope or hosted Armorer dogfooding.

## Next guides and their gates

| Planned deliverable | Preparation now | Gate before a supported operator runbook |
| --- | --- | --- |
| New/existing adoption for library, CLI and service | Preserve profile fixtures and explicit selections | Integrated reviewed policy/provisioning; all profiles/customization paths tested |
| Upgrade, migration and exact rollback | Link #9's immutable guide | Integrated CLI, versioned schemas and recovery tests |
| Complete-release and historical verification | Link the stack guides separately | Integrated independent authority and genuine complete positive/negative controls |
| Native Apple consumer checks | Link #19's exact guide and limits | Accepted macOS integration; production signing remains separate |
| Account/visibility matrix and manual settings | List observations in readiness guide | Qualified preflight plus current primary-source account/plan evidence |
| Credential names, scopes, approvals and rotation | Preserve explicit ownership/permissions | Reviewed adapters define exact names and scopes |
| Advisory/root outages and resumption | Explain freshness and independent policy | Accepted policy and executable outage/retry paths |
| Draft conflict, partial upload and alias recovery | Retain state-machine requirements | Owned receipts, backend and rehearsed faults |
| Post-publication incident/new version | Preserve immutable-release rule | Verified publication backend and operational acceptance |
| Support/deprecation/security policies and dogfooding | Keep experimental scope explicit | Accepted commitments/reporting channel and hosted Armorer rehearsal |

Do the preparation the source supports. Keep unsupported commands out of the
main walkthrough. **This is the way.**

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
