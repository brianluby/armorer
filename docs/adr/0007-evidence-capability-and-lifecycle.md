# ADR 0007: evidence scope, capabilities and distinct lifecycle gates

Status: proposed, 2026-10-01.

Context: optional scanners, native inventory and Apple transformations have
unequal coverage and trust. Producer assertions and authenticated platform
identity must remain distinguishable at the v0.1 Build L2 target.

Decision: use per-artifact input/selection/tool/freshness/coverage/exception and
stage byte records. Independent evidence requirements fix workflows, exact tool
inputs, scope/omissions, maximum ages and allowed exceptions. Every required
control fails closed for unknown/unsupported/error/reporting outcomes. Enumerated
reviewed adapters accept no commands, URLs or custom predicates. Optional
#17–#24 remain design inputs and are not enabled by adding their contract IDs.

Keep availability, enforcement, outcome and lifecycle on separate axes.
Configured/CI verified/release rehearsed/published/provenance verified require
separate source/config/policy-bound gate evidence. Explicit release and rehearsal
policies keep branch/CI evidence from authorizing stable-tag publication. Hashes of authentication,
review and platform records are references; their validation is not proof of
identity or review. #8/#9/#10 must authenticate referenced content and live state.

macOS executable evidence preserves build → sign → notarize → package byte
identities, run/attempt/workflows and Apple assertions. Final provenance covers
final distributed bytes. Source `.crate` packages do not inherit executable
signing. Standalone online ticket checks remain distinct from stapled packages.
No L3, hermeticity or reproducibility claim follows from this chain.

Alternatives: one passed/ready boolean, scanner score as a release gate, or an
opaque signed producer predicate. Each hides missing coverage/trust boundaries.

Consequences: manual protected settings and credential prerequisites remain
explicit; hosted caller environment and actual Apple evidence require rehearsal.
Expected evidence comes through the independent policy/catalog channel, not
from the producer record it validates. See [contract reference](../trust-contracts-v1.md).
