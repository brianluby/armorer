# Armorer implementation plan

Status: proposed for review, 2026-09-30. Architecture/discovery only; no implementation, GitHub repository creation, pilot edits, merges or releases have occurred.

Epic: [Armorer #1](https://kanban.luby.us/tasks/1304), project 16. All child tasks have explicit acceptance criteria and parent/dependency relations. Momus reference: [#48, task ID 1303](https://kanban.luby.us/tasks/1303), project 15. Project-local indexes and database IDs are distinct.

## Scope and milestones

**First usable Linux L2 baseline:** bootstrap config/discovery/plans/apply, required CI, fixed Linux builder, Cargo SBOM, isolated attestations, strict consumer verification and verified immutable draft workflow. Start with a small public fixture and Armorer dogfooding. Expect roughly **15–22 engineering days**, including security tests and initial documentation. This is a usable subset, not completion of all profile/platform/pilot acceptance criteria.

**Complete v0.1 baseline:** reviewed upgrades/migrations, supported library/CLI/service executable profiles, three initial targets including protected Apple finalization, safe retries, two pilot rehearsals and operational documentation. Expect **30–42 engineering days total** (about 6–9 focused solo weeks), including the first milestone. Estimates have approximately 30% uncertainty; native downloads, workspace packaging, Apple service waits and GitHub account settings can move the schedule. Effort is additive work, not a promise of elapsed delivery time.

**Future-version backlog:** L3 assessment and gap closure are deferred and unscheduled, following the user's scope decision on 2026-09-30. They are not part of v0.1, do not block its release, and require no current effort reservation. The previous 3–7 day assessment estimate is historical planning context only; re-estimate assessment and actual gap closure if a future version selects this work. Extra OCI/crates.io/Windows/distribution adapters and deeper hardening also get scoped follow-ups.

The recommended initial targets are native Linux x86_64/ARM64 and macOS ARM64. Windows/musl, containers, deployments, arbitrary custom shell hooks, automatic GitHub administration, arbitrary source hosting and general package-manager installers are beyond v0.1. Library package validation and GitHub-attested .crate archives are included; native crates.io publication is a separate OIDC adapter. CI-only adoption is explicitly useful for unsupported private repositories but does not satisfy secure-release acceptance.

## Reviewable slices and ownership

| Ticket | Scope | Owner boundary | Depends on | Estimate |
| --- | --- | --- | --- | --- |
| [#2](https://kanban.luby.us/tasks/1305) | Architecture and versioned trust contracts | Contract owner | — | 1–2 days |
| [#3](https://kanban.luby.us/tasks/1306) | Typed configuration and Cargo workspace discovery | Contract owner | #2 | 2–3 days |
| [#4](https://kanban.luby.us/tasks/1307) | Idempotent check, plan and transactional apply | Bootstrap owner | #3 | 3–4 days |
| [#5](https://kanban.luby.us/tasks/1308) | Reviewed upgrades and migration of existing customizations | Bootstrap owner | #4 | 2–3 days |
| [#6](https://kanban.luby.us/tasks/1309) | Reusable Rust CI profiles and policy gates | Workflow owner | #3 | 2–3 days |
| [#7](https://kanban.luby.us/tasks/1310) | Trusted fixed-command builders and target-aware SBOMs | Workflow owner | #3, #6 | 3–4 days |
| [#8](https://kanban.luby.us/tasks/1311) | Signed final-byte attestations and strict consumer verifier | Verification owner | #7 | 3–4 days |
| [#9](https://kanban.luby.us/tasks/1312) | Protected Apple signing and build-to-package evidence | Signing owner | #7 | 3–4 days |
| [#10](https://kanban.luby.us/tasks/1313) | Capability checks, immutable drafts and safe release retries | Release owner | #8, #9 | 3–4 days |
| [#11](https://kanban.luby.us/tasks/1314) | Adversarial verification and release-state tests | Verification owner | #10 | 2–3 days |
| [#12](https://kanban.luby.us/tasks/1315) | Momus isolated adoption and signed release rehearsal | Adoption owner | #5, #11 | 1–2 days |
| [#13](https://kanban.luby.us/tasks/1316) | Rusty Brain workspace adoption and release rehearsal | Adoption owner | #5, #11 | 2–3 days |
| [#14](https://kanban.luby.us/tasks/1317) | Open-source onboarding, upgrades and recovery runbooks | Documentation owner | #4, #10 | 2 days |
| [#15](https://kanban.luby.us/tasks/1318) | [Future version] SLSA Build L3 assessment and gap closure | Security reviewer | #12, #13; future scope selection | Deferred; re-estimate later |
| [#16](https://kanban.luby.us/tasks/1319) | Assess additional supply-chain defenses and adapters | Security reviewer | #2 | separate scoped follow-ups |

These are component ownership boundaries, not authorization to start parallel agents. One coordinating maintainer controls shared schemas/catalog, Git staging/commit/rebase/push and cross-repository integration. Future parallel implementation requires explicit delegation and frozen interfaces.

1. **Contracts before code (#2–3):** finalize proposed ADRs and versioned config/lock/plan/inventory/policy/evidence schemas. Define compatibility/error codes and sample library/CLI/service workspace configurations. Provision empty public MIT repositories only after architecture acceptance; initialize isolated branches and protect workflow/runtime ownership.
2. **Bootstrap (#4):** deterministic discovery/planning, ownership/preimage checks, transactional local apply. Fixture-driven mutation testing; no GitHub writes. Ship a reviewable CLI that can propose an existing-repository migration.
3. **CI (#6):** reusable unprivileged profiles and policy gates, SHA-pinned tools/actions, synthetic Cargo fixtures. Preserve bespoke consuming jobs. Build/tool installs fail explicitly.
4. **Linux build/SBOM (#7):** fixed-command targets, explicit packages/binaries/features, controlled run-bound artifacts and target-aware graph validation. No signing/publication credentials.
5. **Verification (#8):** isolated platform attestations and authenticated inventory, strict source/signer/predicate policy and real cryptographic fixtures. Finish offline trust-root contract and historical compatibility.
6. **Publication (#10, Linux backend first):** capability preflight, owned draft state machine, download verification, publication approval and post-publication integrity checks. Rehearsal-only tests precede any separately authorized real release. Apple acceptance for this ticket remains pending until #9.
7. **Apple backend (#9):** protected credentials, validated unsigned handoff, isolated sign/notarize/package and final-byte attestations. Add macOS consumer checks. Review existing apple-signing as a candidate adapter rather than copying its trust claims.
8. **Migration/upgrades (#5):** three-way customization handling, reviewed pin/catalog changes and downgrade controls. Develop against stable schemas; lock evolution owned by contract maintainer.
9. **Adversarial integration (#11):** test publication state failures, triggers/inputs, cross-run provenance and real Sigstore verification. Negative coverage is developed in each earlier slice; this ticket closes cross-component cases.
10. **Pilots (#12–13):** fresh main-state inspection and isolated adoption plans; hosted CI plus full no-publication rehearsal. Momus work must finish before its migration begins. Do not overwrite active feature branches or infer readiness from generated plans.
11. **Operations/open source (#14):** docs, SECURITY/contribution/license policies, upgrades/recovery, support matrix and Armorer dogfooding. Documentation evolves alongside each slice; this ticket is the usability/closeout gate.
12. **Optional defense assessment (#16):** prioritize tool/adaptor follow-ups by threat and cost. L3 assessment/gap closure (#15) is a separate unscheduled future-version backlog item, not a v0.1 slice or release gate. Selecting it later cannot retroactively label earlier unassessed artifacts as L3.

The tracker dependency for #10 includes #9 because the ticket's complete acceptance requires Apple parity. A Linux-only milestone can be reviewed first, while the ticket remains open. Equivalent partial-scope handling applies to first CLI baseline versus complete library/service acceptance. Nothing is closed merely because an early subset passes.

## Frozen interfaces required before independent work

- Versioned config: profile list, package/bin/target/feature cases, release and exception policy. Unknown keys are errors.
- Lock/catalog: immutable workflow/helper/tool pins, distribution digests, compatibility ranges and reviewed version lineage.
- Plan: preimage digests, exact patches, manual actions, capability observations, schema version and plan digest.
- Inventory: artifact IDs, source/config/run identities, target/features, hashes/sizes, required assets and authenticated transformation references; avoid self-referential bundle digests.
- Trust policy: allowed repository/source refs, accepted signer workflow SHAs/predicate types/hosted runners and narrowly defined legacy exceptions; independent of downloaded release data.
- Evidence: separate configured/CI/rehearsal/publication/verification results with source/config/policy binding, timestamps, run URLs and explicit gaps.

Before adding dependencies, review license, maintenance/security status and exact compatible release versions. No source-code or workflow pin is presumed reviewed just because it appears in the research snapshot.

## Pilots verified during discovery

| Candidate | Structure and current observation | Recommendation |
| --- | --- | --- |
| Momus | Public, three release targets, Apple signing; v0.2.0 six assets and immutable=false. Current provenance branch has live uncommitted changes. Future-release immutable setting currently enabled. | Pilot 1, after its ongoing work finishes; consume its lessons without edits now |
| Rusty Brain | Public 18-crate workspace; library + CLI/daemon, additional hook/install binaries, optional local ONNX and native SQLite/vector dependencies. Existing distinct semantic/contract CI and older build-provenance action. Checkout is on an active fix branch. | Confirmed pilot 2; fresh isolated main-based adoption and explicit per-binary/feature/native scope |
| Rusty Flow | Private four-crate workspace, facade library optional fs feature, CLI, Cargo.lock/deny.toml, no exact toolchain file; CI uses moving pins and forgiving installs. | Good later CI-only/capability-denial migration fixture |
| RustCopy | Private five-crate workspace, several feature sets/macOS integration; organization API reports free plan. | Later private capability/complex packaging fixture; native attestations unavailable on current plan |
| Forge | Public library/CLI, multi-platform release and existing SBOM/generic-generator integration | Later Windows/legacy provenance migration candidate |

Read-only checkout snapshots: Momus HEAD d888b052ed5f5a277e18edbed9903e929b55f248; Rusty Brain ada813ef62922e1c10d1327fb618fca5d47ecbc5; Rusty Flow 2a45199f86cd5536e988c185fdb6b04303ab33db; RustCopy 5541e1a53afa64b1f69e24951745726a45deeda8; Forge 704df7213b1aa9967e805bb0b0bd40bacbcfc12d. These are discovery snapshots, not adopted or approved source versions. Reverify before migration.

No pilot discovery SHA is a runtime default or approved release input.

## Meaningful acceptance tests

| Failure injected | Required observation |
| --- | --- |
| Archive byte changed, same checksum filename | Cryptographic verification fails; no extraction/execution/publication |
| SBOM changed; schema remains valid | SBOM provenance or verified predicate comparison fails |
| Genuine artifact from wrong repo/source/signer/predicate | Strict policy rejects it despite valid signature |
| Missing, duplicate, unexpected or incomplete asset | Inventory/draft gate fails before publication |
| Unsigned/replaced release inventory | Inventory authentication fails; never trust replacement builder policy |
| Manifest changes accepted signer pin | Independently loaded verifier policy remains authoritative |
| PR/fork/unsafe trigger or dispatch on unsupported ref | Credentialed/signing/attestation/publication jobs do not start |
| Injected package/feature/path/config input | Typed validation fails; no shell execution or checkout escape |
| Cross-run/attempt artifact or artifact-name substitution | Producer/source/run and digest checks fail |
| Apple signing or notarization failure | No final-byte evidence or publication; other target success is insufficient |
| Immutable setting unavailable/403 or disabled | Explicit unknown/unsupported result; secure publication blocked |
| Historical-policy downgrade applied to new release | Required verification still enforced |
| Conflicting draft retry or concurrent tags | No replacement/mixed inventory or unauthorized moving-tag update |
| Crash during apply, stale plan, symlink file or concurrent apply | Rollback/resumability; conflicts preserve unrelated bytes |
| Workspace feature union/target SBOM mistake | Semantic fixture dependencies expose incorrect graph pairing |
| Valid zero-dependency library | Validation passes with truthful empty dependency scope |

Verifier unit tests need real signed-bundle fixtures and positive controls. Fake gh executables are useful for state-machine faults, but cannot validate cryptography. At least one hosted fixture must execute pinned real gh verification of published or rehearsal attestations. Retain exact failed stage and proof no publish/tag mutation occurred. Avoid testing only implementation branches; exercise independent consumer expectations.

## Documentation and recovery deliverables

Onboarding by profile; new versus existing repository adoption; current account/capability matrix; manual settings with required scopes; credential names and local setup without secret collection; Apple approval/rotation procedures; check/plan/apply/upgrade examples; exact-byte verification; online/offline roots; historical policy; advisory outage behavior; partial draft retry/conflict handling; post-publication incident/new-version recovery; alias update failure; upgrade rollback; support/deprecation/version policy and per-artifact claims.

## Decisions before implementation

Already confirmed: Armorer name, brianluby ownership/two public repositories, MIT, fail-closed secure releases and Rusty Brain pilot. Recommended defaults: Rust implementation; TOML plus lock; GitHub Actions only; native Linux/macOS targets; service executable and library package support first; automatic crates.io/OCI/Windows later; protected signing and publication environments; no silent setting changes or custom release hooks.

The architecture, initial target/distribution scope and baseline tool budget need review. The first contract slice must also determine the exact release toolchain/MSRV, tool/catalog trust bootstrap and whether an admin-read GitHub App can be provisioned for live immutable-setting verification. These are implementation decisions, not missing secrets. A producer cannot enter secure publication until required manual platform prerequisites are demonstrably satisfied.

## Child-ticket acceptance criteria

### #2: Architecture and versioned trust contracts

- Document repository boundaries, threat model, profile scope, and explicit non-goals before code.
- Freeze config, lock, plan, artifact inventory, verification policy and per-artifact evidence schemas; write proposed ADRs.
- Verify Momus #48 and upstream capabilities; choose public Rusty Brain as second pilot; capture decisions and open risks.

### #3: Typed configuration and Cargo workspace discovery

- Support library, CLI and service profiles, workspace inheritance, explicit packages/binaries, target triples and feature sets.
- Use armorer.toml plus armorer.lock; reject unknown keys, unsafe paths, unsupported targets and incompatible pins.
- Metadata discovery does not execute builds; unsafe Cargo config and native dependency requirements are reported.

### #4: Idempotent check, plan and transactional apply

- Check is read-only; plan shows exact diffs and manual prerequisites; apply requires matching file preimages and plan digest.
- Repeated apply is a no-op; partial failures roll back managed files; dirty unrelated files remain byte-identical.
- Never silently overwrite unowned workflows, deny policy or toolchain customizations; reject symlink/path traversal and concurrent apply.

### #5: Reviewed upgrades and migration of existing customizations

- Upgrade previews workflow/tool SHA changes, compatibility and policy changes; no automatic mutable-major pin adoption.
- Three-way merge uses stored generated base; edited managed files yield explicit conflicts rather than replacement.
- Test old config versions, rollback and imported existing workflows; preserve unrelated customized jobs.

### #6: Reusable Rust CI profiles and policy gates

- Pinned Rust/tools and actions; locked workspace tests, doc tests, formatting and Clippy with explicitly selected feature cases.
- RustSec and project-specific cargo-deny license/source/bans policy; bounded expiring exceptions and fail-closed release database policy.
- Actionlint, zizmor and redacted secret scanning; fork PR jobs have read permissions and no signing secrets/OIDC; retain project-specific CI alongside managed jobs.

### #7: Trusted fixed-command builders and target-aware SBOMs

- Trusted reusable controller derives validated build matrix and artifact IDs; caller cannot supply shell, provenance, subjects or digests.
- Build explicit package/bin/target/features with --locked on hosted runners without OIDC/signing secrets or release caches.
- Pin cargo-cyclonedx and schema; validate per-deliverable CycloneDX JSON graph/scope, lock digest, package/version, target/features and library zero-dependency cases.

### #8: Signed final-byte attestations and strict consumer verifier

- Use SHA-pinned consolidated actions/attest with explicit paths for final artifacts, SBOM files and authenticated inventory.
- Verify bytes, repository/source ref and commit, signer workflow/repository/digest, hosted runner and predicate types; compare SBOM predicate to published JSON.
- Trust policy is independent of release content; exact historical allowlist only; no authenticity-failure fallback; bundle and offline trust-root paths documented.

### #9: Protected Apple signing and build-to-package evidence

- Protected caller signing environment with exact ref restrictions; named credentials only; no secret values requested or logged.
- Isolate build code from signing; verify build handoff, record unsigned/signed executable and archive digests, workflow pins, team identity and notarization evidence.
- Attest final signed/notarized/package bytes; verify codesign/expected team/hardened runtime/timestamp and Apple acceptance on macOS; fail every target on signing failure.

### #10: Capability checks, immutable drafts and safe release retries

- Distinguish supported/unsupported/unknown GitHub capabilities, including public/private attestation eligibility and environment/ruleset availability; fail secure release closed.
- Check immutable setting with narrowly scoped admin-read capability; 403 is unknown and blocks publication; report manual setup without exposing credentials.
- Draft exact inventory, re-download and verify served bytes, approve/publish only after gates, then verify immutable release and release attestation.
- Serialize release/tag operations; identical owned drafts resume by digest; conflicting drafts or published bytes never overwritten; stable tags immutable and moving tags controlled.

### #11: Adversarial verification and release-state tests

- Real Sigstore fixture verification plus pinned-gh integration rejects tampered artifacts/SBOMs, wrong source/signer/predicate, missing or extra assets and invalid manifest signatures.
- Reject unauthorized triggers/refs, injected inputs, cross-run substitution, failed signing, capability denial and historical downgrade attempts.
- Exercise conflicting retries, draft mutation before publish, concurrent tags and failed publication/tag move; assertions show no unauthorized publish or execution.

### #12: Momus isolated adoption and signed release rehearsal

- Wait for current Momus work to finish; freshly inspect main and preserve unrelated edits; use isolated adoption branch/worktree.
- Compare Armorer plan against Momus three-target pipeline and consumer contract; do not duplicate or regress its protections.
- Hosted CI and complete signed rehearsal pass; preserve pre-attestation v0.2.0 compatibility and record evidence; no merge or release publication without human authorization.

### #13: Rusty Brain workspace adoption and release rehearsal

- Use public brianluby/rusty-brain, freshly verified main and isolated worktree; do not disturb active feature branches.
- Cover libraries plus rusty-brain/rb-hooks/rb-install deliverables, default/local feature configurations and native dependency scope; preserve bespoke semantic/contract CI.
- Verify exact multi-binary inventory and target SBOM pairing; hosted CI and release rehearsal with no publication; list external model/native downloads and remaining gaps.

### #14: Open-source onboarding, upgrades and recovery runbooks

- MIT licensing, contribution/security policy, version/support contract, examples for every profile and public/private capability matrix.
- Document credential names and setup scopes, signing approvals, onboarding/upgrade, historical verification, outage/retry recovery and manual settings.
- Report configured, CI verified, release rehearsed, published and provenance verified using separate artifact/source-bound evidence; dogfood Armorer before any public release.

### #15: [Future version] SLSA Build L3 assessment and gap closure

Deferred, low-priority backlog item. The following criteria apply only if a future version selects this work; the pilot dependencies preserve useful sequencing but do not schedule it automatically. v0.1 acceptance requires L2 evidence and honest limitations, not this assessment.

- Assess full build/sign/package transitive trust boundary against authentic/unforgeable provenance and isolated-build requirements.
- Review Cargo build scripts/native downloads, OIDC separation, caches, workflow pins and handoff input substitution; demonstrate origin of every trusted claim.
- Publish requirement/evidence/gap record for every artifact; report L2 where L3 remains unsupported; no certification or L3 claim based only on attached attestations.

### #16: Assess additional supply-chain defenses and adapters

- Prioritize CodeQL Rust/Actions, cargo-vet, cargo-auditable, supplementary Syft/native SBOMs, egress controls, Scorecard and reproducibility by threat addressed and operational cost.
- Assess cargo-dist adapter and native crates.io OIDC publishing, including exact .crate byte verification and multi-crate retry semantics.
- Keep core baseline lightweight; document unavailable features, false-positive/exception policy and maintenance commitments before adding default gates.
