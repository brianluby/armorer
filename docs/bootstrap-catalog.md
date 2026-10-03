# Reviewed bootstrap catalog foundation

Inspect the authority before you forge a lock. Keep each native target bound to
its own bytes. **This is the way.**

This #4 slice supplies an independently anchored, immutable bootstrap catalog,
v1-compatible lock rendering and fixed CI/unsigned-build caller templates. It
performs no adoption or consumer mutation itself. [Version-two bootstrap](bootstrap-v2.md)
combines this authority with explicit project policy and transactional provisioning.
The release trust Catalog is a separate contract; no bootstrap snapshot qualifies
attestations, Apple signing, release publication or SLSA Build L2.

## Authority and source evidence

`armorer catalog` authenticates the embedded exact tool-catalog bytes before
parsing them. The expected SHA-256 and workflow repository/full commit live in
reviewed CLI source; they are never accepted from a consuming lock, downloaded
inventory, schema, API response or caller-supplied expected hash. A modified or
reformatted catalog fails authentication even if its JSON schema is valid.
The operator must already trust the CLI source/distribution. This is trust in
membership in that reviewed source, not Sigstore verification of the CLI or tools.

The catalog was read from the authenticated GitHub HTTPS API at workflow commit
`772ca83e386c883c88cc3b936d69f8cb3216c91e`, compared byte-for-byte with its clean
checkout and checked against Git blob `6f001a407768dcd716324624faff60a1633362f6`.
Its SHA-256 is `e21e6cbd2cdd1125dbb9817116b1f46d043807b4d92128abc9dc07256dbb4b13`;
size is 5,970 bytes. The [source/asset receipt](../catalogs/bootstrap-tools-v1-source.json)
retains all 18 native distribution identities from six publishers' exact release
assets, rechecked against publisher GitHub asset digests on 2026-10-02. These
HTTPS metadata observations are review evidence, never a replacement root at
runtime. The accepted workflow installer rehashes actual downloaded bytes before
extraction/execution. No upstream download is initiated by `catalog` or rendering.

The tool versions remain cargo-deny 0.20.2, actionlint 1.7.12, Zizmor 1.30.1,
Gitleaks 8.30.1, cargo-cyclonedx 0.5.9 and CycloneDX CLI 0.33.1. This deliberately
uses the accepted workflow baseline rather than selecting new moving versions.
Upgrades require reviewed source changes and #5 compatibility/migration previews.

## Compatible lock and caller rendering

`AuthenticatedCatalog::render_lock` reads and validates exact `armorer.toml`
bytes without executing Cargo or source. It renders the existing v1 lock schema,
binds its config SHA-256 and names each distribution `tool--target`. Every tool
has three explicit native distribution records, so multi-target adopters never
interpret a single platform's SHA-256 as every platform's bytes. The lock contains
18 entries even when the selected project uses fewer targets. No schema is changed.
The old v1 reader permits these identifier keys; existing imported unqualified
locks are preserved and require an explicit #5 migration rather than silent rewrite.
The workflow's pinned runtime tool catalog remains authoritative during download.

Review the pins. Preserve existing locks until a migration is approved.
**This is the way.**

`caller_workflows` renders two fixed files shown in [examples](../examples/bootstrap-catalog/README.md).
CI supports ordinary PR/push/manual events; the build caller is explicitly manual
and unsigned. Both jobs pin the reviewed full workflow commit and grant only
contents read. Neither exposes shell, inputs, custom provenance, OIDC, signing
credentials, environments or inherited secrets. Existing custom workflows/jobs
stay separate; this renderer does not write or adopt files.

[GitHub's reusable-workflow documentation](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)
confirms job-level full-SHA calls and that nested permissions cannot be elevated.
[GitHub release asset API](https://docs.github.com/en/rest/releases/assets)
documents publisher asset identities/digests. These platform capabilities are
not a hosted-adoption or credential-boundary rehearsal for generated callers.

## Validation and next gate

Tests reject schema-valid substituted digests/URLs and reformatted/oversized
catalog bytes; verify exact config binding and all 18 target-qualified lock pins;
preserve existing locks/custom jobs; and print the catalog with no consumer
files, PATH tools or build execution. Independent actionlint/Zizmor validation
covers the example callers. The complete existing schema/fixture and Rust suite
must pass before committing/pushing this slice; hosted results and review findings
are retained at its exact PR head.

The additive [version-two provisioning plan](bootstrap-v2.md) provides explicit
policy, complete before/proposed views, generated bases and multi-file recovery.
A v1 toolchain plan still cannot authorize writes to these callers or policy.
Reviewed three-way migration/upgrades, hosted adoption and release verification
remain separate gates.

## Preserved catalog-foundation local receipt — 2026-10-02

This original receipt records the catalog slice before integration with other
main changes. Preserve its counts and source scope; use
[the development map](development-status.md) for the current merged boundary.

Pinned Rust 1.95.0 locked build, fmt, strict all-target/all-feature Clippy and
**75 tests** pass (9 library, 10 apply, 6 catalog, 24 inspection,
25 trust-contract and 1 doctest). Existing **14** v1 schemas remain byte-for-byte
compatible; independent hash-locked validation passes **54 positive examples /
7 structural rejections**. Catalog-authenticated actionlint/Zizmor checks pass
for development CI and both exact caller templates. Hosted Linux/macOS will run
the same native workflow audits and record results at the PR's immutable head.
