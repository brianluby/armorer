# Verify the cargo. Hold the trust boundary.

Candidate guide: [PR #21 at `c815a065fb08`](https://github.com/brianluby/armorer/tree/c815a065fb08dbbdc0321b4a80bed66f12da9f64).
Main has no complete-release or historical-comparison CLI. These procedures
describe the reviewed candidate's commands. Its complete-release evidence lookup
has an [open artifact-scope defect](../reviews/2026-10-02-integration-pr21.md): a
successful report from this head is insufficient for accepting the complete
provenance claim until that defect is fixed and tested.

Inspect the evidence. Earn the verdict. **This is the way.**

## Supply authority through an independent path

Set `armorer_candidate` to the exact candidate binary. Obtain the expected context
digest through independent review. Keep trusted inputs outside the downloaded
asset directory and under separate ownership. Hashing a context offered with a
release does not approve it.

The trusted directory contains `armorer.toml`, `armorer.lock`, `Cargo.lock`,
`catalog.json`, `verification-policy.json` and the explicitly selected context:

| Mode | Context file | Meaning |
| --- | --- | --- |
| `native-v2` | `armorer-verification-context-v2.json` | Runtime archive and target-specific native member authority |
| `legacy-v1` | `armorer-verification-context.json` | Frozen catalog v1 and exact distribution identities; genuine signatures still required |

The digest binds configuration/lock/catalog/policy, source/ref, caller and reusable
signer pins, run/attempt, selected deliverables, native validator and evidence
requirements. Reviews and time-sensitive evidence use the actual clock. A context
failure never retries another mode. Context/catalog schema agreement alone does
not establish accepted production roots or upstream tool authenticity.

```sh
"$armorer_candidate" verify-release \
  --directory /path/to/downloaded-release \
  --trusted-inputs /path/to/independently-reviewed-inputs \
  --expect-context-sha256 "$approved_context_sha256" \
  --context-kind native-v2 \
  --gh /path/to/qualified/gh \
  --trusted-root /path/to/approved/trusted-root.json \
  --cyclonedx /path/to/qualified/cyclonedx
```

The candidate authenticates context before release/tool reads, then inventory
before producer claims. It snapshots exact retained assets, authenticates required
bundles, validates whole SBOM documents, compares signed predicates with retained
JSON, and reconciles selected Cargo graphs against independent context. Runtime
approval/extraction is not a CLI operation; complete verification accepts an
already approved explicit context.

Offered executable paths are locations, not trust pins. Fixed adapters use private
snapshots, compiled/native approvals and sterile environments. Signature and SBOM
adapters run offline. The consumer never runs Cargo, executes payloads, downloads
tools, collects tokens or publishes. Stable filesystem ancestry and a trusted
local account remain assumptions.

Apple CLI/service selections additionally require native macOS Developer ID/team,
certificate, hardened runtime, timestamp and ticket checks. Linux cannot satisfy
that gate. Library source packages do not require executable signing. Fixed
codesign requests an online ticket check, but the system ticket store may supply
the result; this establishes no fresh-service-response guarantee or authenticated
producer submission UUID. See the
[exact native Apple contract](https://github.com/brianluby/armorer/blob/c815a065fb08dbbdc0321b4a80bed66f12da9f64/docs/apple-native-verification-v1.md).

The implemented success report uses `status: authenticated-release-files`,
`cryptographic_release_authenticated: true` and `provenance_verified: true` with
bound identities and verified counts. At this head, the missing platform-evidence
subject check prevents treating that last claim as complete acceptance. In every
case `publication_authorized` stays false and `slsa_build_level` stays null.
Operational errors use JSON/exit 1; argument errors use Clap stderr/exit 2.

Preserve failed evidence. Correct the explicit prerequisite. Retry against the
same independently approved expectations. There is no partial, unsigned,
source/signer override or automatic historical fallback.

Hold the gate until the evidence belongs to the artifact.
**This is the way.**

## Compare historical bytes only by explicit decision

This separate route accepts an independently approved historical allowlist. Obtain
its exact policy digest, repository, full source commit and stable `v` tag through
review. Keep the policy outside the asset-only directory.

```sh
"$armorer_candidate" verify-historical-bytes \
  --directory /path/to/historical-assets \
  --policy /path/to/independently-reviewed-policy.json \
  --expect-policy-sha256 "$approved_policy_sha256" \
  --source-repository owner/project \
  --source-commit "$approved_source_commit" \
  --source-tag v0.1.0
```

The command compares the complete exact asset set, names, digests and sizes at the
approved source. Missing/extra, nested, unsafe, symlinked or special-file entries
fail. Known modern inventory/attestation filenames are forbidden, even if listed
in the historical policy. Do not remove failed modern evidence to manufacture a
legacy exception. Modern and historical policy source lists cannot overlap.

Success is `status: historical-byte-match`, `authenticity: not-established`,
`provenance_verified: false` and a null SLSA level. It authenticates no source,
signer, provenance or build environment. Actual-clock policy/root/historical
review validity is checked before and after comparison; the frozen policy's root
review requirement does not mean this operation uses a signature verifier.

The [source limits and recovery guide](https://github.com/brianluby/armorer/blob/c815a065fb08dbbdc0321b4a80bed66f12da9f64/docs/historical-verification.md)
define byte/count/time bounds. Keep failed evidence and obtain corrected files or
a newly independently approved policy. Explicit historical matching never becomes
a fallback after authenticated verification fails.

The [integration review](../reviews/2026-10-02-integration-pr21.md) records ordinary
tests and hosted native checks separately from the missing complete own signed
producer positive, protected Apple production, immutable publication and pilots.
v0.1 targets SLSA Build L2. No candidate file report earns that level by itself.

Keep the source. Keep the limits. **This is the way.**
