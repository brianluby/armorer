# ADR 0005: Additional defenses and bounded publishing adapters

Status: proposed for review, 2026-10-01; [ticket #16](https://kanban.luby.us/tasks/1319).

## Context

The existing baseline has strict CI policy and target-aware unsigned build
inventories. SLSA Build L2 requires provenance but does not replace source review,
dependency trust, installed-binary auditing or native inventory. Optional tools
can improve those areas while introducing code execution, mutable services,
maintenance costs and new credentials. Ticket #16 blocks the remaining release
contract freeze in #2; its adapter implementations need not block v0.1.

## Decision

Retain the lightweight v0.1 baseline. Prioritize opt-in CodeQL Rust/Actions,
auditable executables and supplementary native inventory, then dependency
vetting, Linux egress qualification and independent unsigned rebuilds. Run
Scorecard as diagnostics. Require reviewed capability/coverage/exception
records before promoting any new gate; unavailable required capabilities fail
closed. Optional controls never replace baseline verification.

CodeQL Rust `none` still executes build scripts/macros; keep it outside
read-only discovery and all credentialed release jobs. Embed auditable metadata
before signing and validate retention from final bytes. Keep Cargo and native
SBOM scope distinct. Neither metadata nor an allowlisted endpoint establishes
malicious-code absence, hermeticity or a higher SLSA level.

Defer dist and crates.io publication to bounded adapters. A dist spike must
qualify adapter-generated effective configuration, fixed commands and exact
selections without caller hooks or backend publication/attestation. Armorer
retains inventory, signing, verification and release state ownership. Reject an
adapter that cannot preserve those boundaries.

A crates.io spike must prove the actual uploaded `.crate` equals the approved
object, then verify registry-served bytes and the index checksum. Isolate OIDC
from packaging/build scripts; protect the caller workflow/environment because
current registry authorization does not pin the reusable workflow SHA. Handle
first-publication prerequisites, non-atomic per-crate DAG completion and
conflicting retries explicitly. No token fallback or cross-service atomicity
claim.

Feed capability IDs, evidence roles/scope/results and independent trust policy
requirements into #2. Freeze the actual schemas there; this assessment adds no
configuration fields or runtime behavior. The [assessment](../supply-chain-assessment.md)
contains primary sources, prioritization, eligibility, acceptance tests and
maintenance responsibilities.

## Alternatives considered

* Enable every scanner by default: increases operational cost, untriaged noise
  and required subscriptions without demonstrated coverage. Reject.
* Replace the release controller with dist-generated CI: gains distribution
  features but has not demonstrated our credential and final-byte boundaries.
  Defer pending a bounded adapter qualification.
* Publish with a standing crates.io token in a Cargo build job: easier initial
  setup, but couples untrusted code execution to credentials. Reject as default.
* Defer all optional decisions until after schemas: risks conflicting artifact
  roles and lifecycle semantics. Assess now, implement separately.

## Consequences and review

The core baseline remains usable without paid services or maintainer dependency
audits. Separate optional follow-ups have owners, estimates and failure tests.
The cost is qualifying more evidence kinds and recording explicit coverage
limits. No optional implementation, platform setting change or pilot adoption
occurs in this slice. L3 remains deferred.

Human PR review decides acceptance of this ADR. After merge, #16 can close and
#2 can proceed with its remaining contracts; #2 is not completed by this ADR.
