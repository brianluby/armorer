# ADR 0006: additive release contracts and independent trust

Status: proposed, 2026-10-01; acceptance requires review of ticket #2 delivery.

Context: config/lock/plan and the unsigned workflow inventory are already merged.
Changing strict readers or treating unsigned builder data as trust would regress
completed work. Release contracts were explicitly deferred.

Decision: preserve those v1 bytes/meanings. Add separately versioned JSON release
inventory, independent verification policy, evidence/requirements, capability
sidecar/catalog and publication receipts. Use fixed role/subject naming derived
from reviewed config/policy, exact SHA-256/size identities, and bounded strict
loading. The new policy is JSON rather than the architecture's unimplemented
TOML proposal so the same strict loader/schema applies to all new documents.
Incompatible field/enum/required-asset changes require a new version and explicit
catalog/reader migration; no unknown-field tolerance or silent downgrade.

Artifact/SBOM/evidence bundles precede inventory generation. Inventory and its
own authentication bundle remain outside its asset list. Require the detached
bundle independently. Keep baseline attestations mandatory; reporting controls
cannot waive them. Historical mode requires independently reviewed exact
source/tag/bytes and never follows a verification failure automatically.

Alternatives: mutate existing config/lock/plan, trust release-supplied policy,
list inventory authentication inside its own inventory, or fallback to checksums.
These break reader compatibility or introduce circular/self-selected trust.

Consequences: #4/#5 must integrate/authenticate sidecars/catalog through review;
#8 implements real cryptographic verification and explicit historical routing.
No production roots, tool authentication or approved upstream pins are invented.
The fixed release layout is new controller work; unsigned builder names remain.
See [contract reference](../trust-contracts-v1.md) for limits and migration rules.
