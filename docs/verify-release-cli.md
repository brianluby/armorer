# Complete release verification from the CLI

`verify-release` authenticates the complete release set with the inventory-first
strict consumer. Signature/SBOM adapters operate offline; required Apple
executable checks use macOS Security.framework and request an online ticket
check through fixed system codesign. The final native requirement may use the
system ticket store; no fresh service-response or offline availability guarantee
is claimed.
It authenticates the independent context before opening executable adapters or
reading a release. It then verifies the inventory before parsing producer claims,
rehashes every exact declared asset, verifies every required Sigstore bundle,
compares signed SBOM predicates with retained JSON and selected Cargo graphs, and
validates complete evidence chains against the independently reviewed requirements.
A failed required member produces no successful subset.

```sh
armorer verify-release \
  --directory /path/to/downloaded-release \
  --trusted-inputs /path/to/independently-reviewed-inputs \
  --expect-context-sha256 '<independently approved 64-character SHA-256>' \
  --context-kind native-v2 \
  --gh /path/to/qualified/gh \
  --trusted-root /path/to/approved/trusted-root.json \
  --cyclonedx /path/to/qualified/cyclonedx
```

The placeholder digest must be replaced by an independently approved digest;
hashing an offered release's context and treating that hash as approval defeats
the trust boundary. Keep the trusted input directory under separate ownership
from the downloaded release. It contains `armorer.toml`, `armorer.lock`,
`Cargo.lock`, `catalog.json`, `verification-policy.json` and the explicitly
selected context:

| `--context-kind` | Context file | Catalog semantics |
| --- | --- | --- |
| `native-v2` | `armorer-verification-context-v2.json` | Reviewed runtime distribution and target-specific native members |
| `legacy-v1` | `armorer-verification-context.json` | Frozen v1 catalog and exact distribution byte identities |

The context digest binds exact config/lock/catalog/policy identities, source and
ref, caller and reusable signer pins, run/attempt, native validator, selections
and evidence requirements. Required reviews and time-sensitive evidence are
rechecked using the actual clock. Catalog/schema agreement alone does not prove
upstream authenticity or acceptance; reviewed native distribution/root/tool
qualification remains an independent prerequisite. Context failures never retry
another version. Legacy v1 here still requires genuine signatures; the separate
`verify-historical-bytes` command retains its explicit unauthenticated comparison
semantics and is never a verification fallback.

Executable paths are offered locations, not pins or generic command inputs. The
consumer snapshots the executables and root privately, compares their exact
bytes with independent context/policy and compiled native approvals, checks
native executable format and invokes the signature/SBOM adapters only through
their fixed offline commands with a sterile environment. After whole-file
authentication, required Apple executables also undergo the native macOS
signature, timestamp, team, runtime and ticket checks described below. It does not search PATH for replacements, learn source or
signer expectations from bundles, collect tokens, download tools, contact release
APIs, run Cargo, execute/extract payloads, mutate tags or publish releases. Stable
filesystem ancestry and no hostile process sharing the consumer's OS account
remain required. See [offline verifier details](offline-verifier.md) for native
version/root support, bounds and limitations.

On complete success, stdout is JSON with `status: authenticated-release-files`,
`cryptographic_release_authenticated: true`, `provenance_verified: true`, the
independent context/catalog/input identities, inventory and detached bundle
identities, approved native verifier/root/policy identities, all inventory assets
and counts of verified bundles/SBOMs. The command exits 0 only after every required
consumer check succeeds. This report is an audit record, not a reconstructable
private authorization proof. `publication_authorized` remains false and
`slsa_build_level` remains null: file verification alone does not establish hosted
release acceptance, protection, Apple operational qualification or publication.

Operational errors use the existing JSON error format and exit 1; argument errors
use Clap's stderr format and exit 2. Native verifier diagnostics are redacted by
the existing adapters. Fix the explicit failed prerequisite and rerun the same
independent expectations. There is no partial, unsigned or historical downgrade
switch, source/signer override or custom provenance input.

## Evidence and remaining gates

CLI failure cases cover unapproved context bytes before release/tool reads,
explicit version rejection, no caller-selected fake executable use and argument
claim injection. The required genuine native inventory integration also invokes
the CLI with a real retained Sigstore bundle and qualified gh/CycloneDX; unrelated
bundles cannot authenticate malformed inventories or activate historical fallback.
These tests do not substitute for a complete own signed producer positive.

Production catalog/root acceptance, qualified final producer integration, the own
complete signed non-publishing rehearsal, protected Apple finalization, immutable
publication, both pilots and human acceptance remain incomplete. This command
makes the consumer usable without weakening or completing those external gates.

Apple CLI/service selections also require the [native Apple verification v1](apple-native-verification-v1.md) gate on macOS. Required native failures yield no successful complete-release result; library/Linux sets report `apple_verification: not-required`.
