# ADR 0008: independent GitHub and registry publication receipts

Status: proposed, 2026-10-01; registry implementation remains future scope.

Context: GitHub owned draft publication and multi-crate registry uploads have
different guarantees. Upload polling and service visibility cannot establish
atomic completion or exact bytes.

Decision: GitHub receipts bind owned draft ID/tag/source/config/lock/runtime/
run/attempt, inventory/policy bytes, verified draft receipt, approval, current
immutable setting and separate post-publication release/provenance evidence.
Only identical owned draft identities may resume; published receipts cannot
resume as drafts. Conflicts require explicit recovery and never replacement.

Reserve a separate crates.io publish-set identity: compact typed JSON of frozen
source/config/lock/runtime context and topologically ordered crate name/version/
archive/dependency intent, excluding mutable status receipts. Registry receipts
record prepared/uploaded/unknown-upload-result/observed/byte-verified/conflict per
crate. Dependents cannot upload before dependencies are byte-verified; served
bytes and the index checksum must match the approved archive. Changed intent
requires new review. No automatic yanking/version choice or multi-crate rollback.

Alternatives: overload GitHub lifecycle or hash mutable registry receipts as set
identity. These obscure partial completion and make safe retry identity unstable.

Consequences: #10 implements authenticated remote transitions and races; future
#24 must qualify exact upload objects, external dependencies, publisher identity,
first-publication prerequisites and publication order without standing-token
fallback. Publication order is a future adapter decision. No cross-service
atomicity or rollback is claimed or implemented. See [contract reference](../trust-contracts-v1.md).
