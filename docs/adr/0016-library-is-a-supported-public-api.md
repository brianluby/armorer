# ADR 0016: the library is a supported public API

Status: proposed for owner review, 2026-11-19. The decision that the library is
supported is accepted by the owner; the compatibility rule and the check below
are proposals. This ADR authorizes no release, tag or publication.

## Context

Armorer is a library crate and a binary. Until now the library's `pub` surface
grew for the convenience of `src/main.rs` and the integration tests in `tests/`,
and no one treated it as a contract. ADR 0012 freezes document IDs and versions
at the 0.1.0 tag and says nothing about crate exports. The owner decided on
2026-11-19 that the library is a supported public API
([architecture review](../architecture-review-2026-11-16.md#owner-decisions-2026-11-19)).

[The inventory](../public-api-inventory.md) records what is exported at `9244948`:
29 modules and 217 items (110 structs, 38 enums, 46 functions, 22 constants, one
type alias), with 118 public methods, 548 public fields on 93 structs and 167 enum
variants beneath them. Features add 10 more items. Only 66 items are named by the
binary and 71 by tests alone; 80 are named by neither. No enum and no struct is
`#[non_exhaustive]`, and `Error` is an exhaustive 11-variant enum. Several
constants pin third-party tool versions and digests that change with every
reviewed bump.

There is no tagged release, no known external caller and no check that notices a
change to the surface. Cargo treats a removed or changed public item as an
incompatible change, so each export is a promise once 0.1.0 is tagged.

## Decision (accepted)

**From 0.1.0 the Rust library is a supported public API**, not only the binary.

## Proposed rule

1. **Incompatible changes need a signalling version.** From the 0.1.0 tag, an
   incompatible change to an exported item (removing or renaming it, changing a
   signature, adding a field to a struct with public fields, adding a variant to
   an exhaustive enum, or changing a public trait implementation) requires a
   version that signals it under Cargo's semantic-versioning rules. While the
   major version is 0, that is a minor version bump.
2. **Release notes list the surface.** The 0.1.0 release notes list the supported
   library surface next to the supported document versions that ADR 0012 rule 5
   requires. A release reviewer following ADR 0012 then sees both.
3. **Reduce before the tag.** The surface is reduced to what is intended before
   0.1.0, because removing an export afterwards is itself an incompatible change.
   The owner works through the inventory's `restrict` and `decide` rows. The
   decisions most in need of an owner are: whether pinned versions and digests
   stay exported, whether the GitHub publication controller is library API,
   whether to mark enums and document structs `#[non_exhaustive]`, what stable
   shape `Error` has, and what the `test-predecessor` and `fuzzing` features
   promise.

## Proposed check (not implemented)

No option is implemented by this ADR. Each needs a decision from the owner.

- **(a) `cargo-semver-checks` in CI against a baseline revision.** It detects
  signature, field, variant and trait changes, not only names. Cost: a third-party
  tool joins a supply chain this project is strict about. Every tool is pinned by
  version, URL and SHA-256 in `catalogs/bootstrap-tools-v1.json`, authenticated
  through `catalog::TOOLS_SHA256` in `src/catalog.rs`, and reviewed on a window.
  Adding the tool means a new catalog entry and a reviewed digest, and its
  reliance on rustdoc JSON ties it to a toolchain version, which must be checked
  against the pinned 1.95.0 (`rust-toolchain.toml`); that has not been verified.
  Needs from the owner: approval of the tool as a pinned dependency, and the
  baseline (the 0.1.0 tag; before it there is nothing to compare with).
- **(b) A committed, generated snapshot of the surface that a test compares
  against.** Cheap to run and reviewable in a diff. A complete snapshot needs
  rustdoc JSON, which needs a nightly toolchain (`-Z unstable-options`) and whose
  format changes between nightlies; the repository pins stable 1.95.0, so CI
  would need a second pinned toolchain. A source-level approximation (a test that
  scans `src/` for `pub` items, as `src/filesystem.rs` already does for another
  rule) needs no nightly, but sees names only: it misses signature changes,
  re-exports and trait implementations. Needs from the owner: a choice between
  a pinned nightly and the weaker source-level check, and who regenerates the
  snapshot.
- **(c) Review plus the existing automated review comments.** The status quo. No
  tooling, no new supply-chain entry, and no enforcement: an incompatible change
  passes unless a reviewer notices. Needs from the owner: acceptance of that risk
  and a pull-request checklist line.

**Recommendation.** Use (c) and the inventory until the tag, because the surface
is being reduced on purpose and a baseline would only record churn. Adopt (a) at
the tag, with the 0.1.0 tag as the baseline and the tool pinned in the catalog
like the others. Prefer (a) over (b): it checks signatures and needs no committed
snapshot to keep in step. If the owner will not add a tool, take (b) with a
pinned nightly. This recommendation depends on the unverified toolchain
compatibility above.

## Consequences

- The reduction work comes first: roughly the 71 test-only and 16 binary-only
  exports need a decision, and narrowing those that tests or the binary use needs
  a mechanism (moving code into the binary, a feature, or in-crate tests), not
  only `pub(crate)`. That is a code change for a separate pull request.
- After the tag, every exported field and variant has a cost. Marking types
  `#[non_exhaustive]` is itself an incompatible change if done later, so it is a
  before-the-tag decision.
- Security guarantees that rest on private fields in the verifier's proof types
  become part of the supported contract; making such a field public is a security
  change.
- ADR 0012 is unchanged. Document IDs and versions stay frozen on their own rule;
  this ADR adds the Rust surface beside them.

## Out of scope

- Publishing the crate to crates.io. `Cargo.toml` carries `include`, `readme` and
  no `publish = false`, but whether to publish is a separate, undecided question;
  this ADR concerns the contract whether or not it is published.
- The A5 workspace split and a standalone verifier crate, which would change which
  crate owns which exports.
- Compatibility of the command line, the JSON output and the schemas, which
  are governed elsewhere (see `docs/compatibility.md`).
- The deprecation and support window, which the support-policy draft leaves open.
