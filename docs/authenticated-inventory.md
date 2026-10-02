# Independent contexts and inventory-first file verification

This library slice supplies `TrustedReleaseContext` and
`AuthenticatedReleaseFiles`. It authenticates an inventory before reading its
asset names, then authenticates all declared file bytes and offered bundles and
checks selected Cargo graphs, complete SBOM documents and evidence records.
Its result has a private constructor and cannot be deserialized from release
JSON. It does not establish publication, protected Apple finalization, full
platform-report semantics or SLSA Build L2.

## Approve expectations outside the download channel

A trusted input directory contains `armorer-verification-context.json`,
`armorer.toml`, `armorer.lock`, `Cargo.lock`, `catalog.json` and
`verification-policy.json`. The context conforms to
`schemas/verification-context-v1.json`; obtain its structural schema with
`armorer schema verification-context`. Schema validity is only a structural
check. No whole-release CLI or automatic context generator is supplied yet.

The approved context SHA-256 comes from a separate reviewed source. Never obtain
it, the catalog or policy approval digest, source/ref/commit, invoking workflow,
run/attempt, reusable signer pin or root identity from the release download.
Review fields alone cannot establish approval. Changing approved inputs requires
new independently reviewed bytes and a new approved context digest.

`TrustedReleaseContext::open` matches the context digest before parsing. It checks
the exact configuration, both locks, catalog and verification-policy bytes;
current context/catalog/policy/root review expiry; the input runtime version and
workflow commit; source and caller identities; and one independently selected
signer for every fixed evidence scope. Signers must be allowed by the exact
approved policy and match the locked reusable workflow repository/commit.
Release policies accept `push`; rehearsal policies accept `workflow_dispatch`.
Neither accepts pull-request authentication or silently changes backend.

Every configured deliverable/target appears exactly once, with its independently
resolved package version, root component name, feature selection and toolchain.
Each selection supplies independently reviewed `EvidenceRequirements`. Build and
package steps are mandatory; macOS CLI/service selections also require sign and
notarize steps, an expected Apple team and the Apple adapter. Required tool
identities and authentication records must match admitted catalog pins. All pins
from the Rust, Cargo CycloneDX and GitHub Sigstore baseline adapters are required;
Apple pins are additionally required for signed macOS executables. Cargo SBOM,
dependency-policy and platform-attestation coverage must be passed/enforced;
Apple signing coverage is also mandatory where applicable. Empty or substituted
baseline requirements cannot be approved into a weaker context.

Opening this context reads inert bounded files. It does not run Cargo, resolve
dependencies, invoke a build script, discover an adopter repository or write
consuming files. The synthetic tests include a consuming build script trap and
verify exact preimages after opening.

## Authenticate the fixed inventory first

Open the [offline signature verifier](offline-verifier.md) using the exact policy
approval digest, the independently qualified native `gh` and exact approved local
root bytes. Open the [complete SBOM validator](complete-sbom-validation.md) using
the independently approved compiled native CycloneDX identity.

Then call `AuthenticatedReleaseFiles::verify(download_directory, &context,
&verifier, &sbom_validator)`. The download directory must be a stable directory of
regular leaves. The consumer snapshots only `armorer-release-inventory.json` and
its fixed detached bundle initially. It authenticates that inventory's observed
bytes against independent inventory source/signer/run/trigger/root expectations
before parsing its JSON, reading asset names or accessing other files. The
observed inventory digest is a subject candidate, not an independently approved
asset-list assertion. Authenticity comes from the approved signer and complete
cryptographic identity checks.

Only after authentication succeeds does the consumer validate the inventory
against independent config/policy/input identities. It requires the exact file
set, rejects missing/extra assets, directories and symlinks, and snapshots and
rehashes every declared asset. All supplied bundle slots are verified against
independent role/predicate/signer expectations. No successful subset is returned.

For every independent selection, complete CycloneDX 1.5 validation must succeed.
The verified CycloneDX predicate for the final artifact must equal the published
JSON document semantically. The strict v2 Cargo graph must match independent
source/config/locks/runtime/run/attempt and selected root/version/features, and
its complete required component/node/edge set must match that document. Build,
package and applicable Apple records must satisfy independent evidence
requirements. Platform-evidence references and notarization logs must identify
actual retained authenticated inventory assets; a bare producer digest is
insufficient. Final evidence steps must identify the distributed bytes.

The full operation rechecks review expiry and every private file snapshot before
returning. Its read-only getters expose verified identities and documents.
`asset_bytes` rehashes its private inert snapshot again before returning bytes.
Payloads, `.crate` files and archive contents are never executed or extracted.

## Limits and pending gates

This API supports only the previously qualified native hosts: Linux x86_64,
Linux ARM64 and macOS ARM64. It uses a 4 MiB context limit, the existing 1 MiB
configuration/lock/catalog/policy limits, 17 MiB metadata, 64 MiB bundles, 1 GiB
distributable assets, 4 GiB total bytes, at most 8,194 regular leaves and a
20-minute complete-operation deadline checked between stages. Each native
invocation retains its own existing 30-second/output limits. JSON retains the
262,144-node and 64-depth bounds. This is intended for a stable filesystem;
hostile same-account mutation or directory ancestry changes remain outside the
snapshot isolation guarantee documented by the native adapters.

Optional reporting assets are still byte-authenticated and every offered bundle
is checked. A required supplemental semantic adapter currently fails explicitly
as unsupported: native SBOM, embedded metadata, independent rebuild and other
supplemental report interpretation are not silently reduced to a digest check.
Retained platform reports have byte/reference authentication here; their complete
typed platform semantics and protected Apple execution are separate pending
gates. Native/system/downloaded dependency limitations remain explicit in the
selected Cargo graph.

The current implementation has genuine native inventory-authentication negative
coverage and synthetic independent-context/substitution/no-build-execution
coverage. It does not yet have an own-repository authenticated complete-inventory
positive rehearsal. A real SHA-pinned final-byte producer, retained qualified
platform reports, complete signed nonpublishing rehearsal, genuine private
backend positive qualification and human acceptance remain required before
claiming complete #8 acceptance. An external wheel signature fixture establishes
only the native signature adapter and the inventory-first rejection boundary.

Historical compatibility is never attempted by this operation, including after
failed authentication. An explicit exact historical consumer route remains
pending; existing historical policy records do not enable automatic downgrade.
Draft ownership, served-byte re-download, ancestry/actor checks, protected
publication and immutable-release attestations are separately tracked #10 gates.

## Run the native inventory rejection gate

Use the already documented native tool qualification scripts, then run:

```sh
ARMORER_TEST_GH=/path/to/qualified/gh \
ARMORER_TEST_CDX=/path/to/qualified/cyclonedx \
cargo test --locked --test release_context -- --ignored --test-threads=1
```

The test uses the retained genuine unrelated bundle and actual pinned verifier,
and asserts authentication fails before malformed inventory JSON, path claims,
other assets or a compatibility route can be processed. It does not manufacture
a successful signature or a complete inventory proof. Development CI runs this
gate explicitly after qualifying both native tools on all three hosts.
