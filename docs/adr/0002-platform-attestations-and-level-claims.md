# ADR 0002: platform attestations and evidence-based level claims

Status: accepted design, 2026-09-30. v0.1 targets L2; implementation and hosted evidence remain pending. L3 is deferred.

Context: artifact subjects must cover published final bytes, while compiler code, Apple credentials and provenance signing have distinct trust boundaries. Current generic SLSA generator upstream is no longer actively maintained.

Decision: consolidate on SHA-pinned actions/attest and pinned gh verification, Cargo-aware CycloneDX JSON, a trusted full release controller and separate privileged jobs. Deliver SLSA v1.2 Build L2 for v0.1 and preserve build/sign/package digest chains. Following the user's 2026-09-30 scope decision, defer L3 assessment and gap closure to an unscheduled future-version backlog item; it is not a v0.1 release gate.

Alternatives: standalone cosign/custom provenance increases backend maintenance; the generic generator adds a separate provenance mechanism and upstream maintenance/pinning concerns; filesystem SBOM-only generation misses Cargo target/feature intent.

Consequences: private GitHub attestations require Enterprise Cloud. L2 trace records may contain documented tenant-produced fields. Reusable workflows and final-byte hashing do not alone prove the entire transformation chain reaches L3. Native dependency coverage remains explicit.
