# Synthetic version-one trust examples

Train on fixtures. Bring real evidence to a release.
**This is the way.**

**Every credential-free pin, bundle, root, review, Apple assertion and platform
reference here is synthetic and unauthenticated. Do not install these pins or
use these policies for releases.** Asset bytes in integrity tests are exactly
`fixture:<asset-name>`; they are not real archives, CycloneDX documents or Sigstore
bundles. No secrets, pilot inputs or production downloads appear here.

* `library`: zero-dependency source package scope associated with Linux.
* `linux-cli`: final unsigned Linux packaging; platform provenance is still required.
* `workspace-service`: ARM Linux service selection.
* `workspace/armorer.toml`: combined library/CLI/service workspace intent, including
  macOS CLI and an explicit service feature case; pair with source-resolved metadata.
* `macos-final`: unsigned/sign/notarize/final-package byte chain with standalone
  online notarization assertions; it does not demonstrate Apple acceptance.
* `historical-verification-policy.json`: explicitly reviewed exact legacy source/
  tag/bytes with weaker status; never an authenticity-error fallback.

Each complete profile includes policy, inventory, evidence, independent evidence
requirements, capabilities/catalog, lifecycle, GitHub draft and future registry
examples. Optional diagnostic policy is reporting; required baseline evidence
is fixed. Every example uses exact config/lock digests. Package and runtime pins
remain fixture values, not approved production inputs.

Regenerate deterministic data with `python3 scripts/generate-contract-fixtures.py`.
Run `cargo test --locked --test trust_contracts`. Schema export/consistency is
checked for every old and new schema. [Contract reference](../../docs/trust-contracts-v1.md)
describes structural/semantic/runtime gates and handoffs.

Keep the synthetic label attached to every result. **This is the way.**
