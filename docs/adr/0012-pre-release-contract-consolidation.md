# ADR 0012: consolidate superseded contracts before the first release

Status: decision accepted by the owner, 2026-11-16. Implementation is pending
and is delivered as separately reviewed removals. This ADR authorizes no
release, tag or publication.

## Context

Armorer has no tagged release and no external adopter. Both pilots (Momus and
Rusty Brain) have not started. The only repository carrying Armorer-managed
files is Armorer itself, bootstrapped on 2026-11-16 at `bootstrap-v2`.

During development, each new contract was added beside its predecessor and the
predecessor was frozen. ADRs 0009, 0010 and 0011 and the comment in
`src/upgrade/authority.rs` state the rule: a stable ID or version has an
immutable meaning, and a successor is added without moving or removing it.

That rule protects adopters. Applied before any adopter exists, it has only a
cost. Main currently carries:

- two provisioning paths: version-one `plan` / `preview` / `apply` / `recover`
  (toolchain file only) and version-two `bootstrap` (all five files);
- three ownership formats: `.armorer/state.json`,
  `.armorer/bootstrap-state-v2.json` and the upgrade state;
- three verification-context readers: `legacy-v1`, `native-v2` and `native-v3`;
- two compiled catalogs: `bootstrap-v1` and `bootstrap-v2`, which authenticate
  the same tool bytes and differ only in the workflow commit.

Every retained version needs its own schema, examples, tests, guards against
cross-version confusion, and documentation. The 2026-11-16 review
(`reviews/2026-11-16-architecture-security-roadmap-review.md`, finding A4)
identified this as the main obstacle to unifying the transaction engine and to
adding other language ecosystems.

## Decision

**The immutability rule takes effect at the first tagged release.** Before that
tag, a superseded contract that no adopter depends on is removed, not frozen.

Armorer 0.1.0 ships exactly one supported version of each document and one
provisioning path. From the 0.1.0 tag onward, the existing rule applies without
exception: published IDs and versions keep their meaning, and change arrives
only as a separately reviewed successor with migration tests.

### Removed before 0.1.0

| Removed | Survivor |
| --- | --- |
| Top-level `plan`, `plan --preview`, `preview`, `apply` and `recover` commands and the `plan-v1` schema | `bootstrap plan`, `bootstrap check`, `bootstrap apply`, `bootstrap recover` and `bootstrap-plan-v2` |
| Version-one ownership state `.armorer/state.json`, journal `.armorer/journal.json`, and the guards and migration paths that exist only to keep them apart from version two (`guard_v1`, the `LegacyV1` upgrade source) | `.armorer/bootstrap-state-v2.json` and the upgrade state |
| `--context-kind legacy-v1` and `native-v2`, the `open` and `open_native_v2` readers, and the standalone `verification-context-v1` and `verification-context-v2` schemas | `native-v3` (`verification-context-v3`) |
| Compiled catalog `bootstrap-v1` as a production authority | `bootstrap-v2` |

Top-level `check` is kept as a command name but reads the surviving plan
contract. The removed command names stay reserved; reusing them is a separate
CLI decision.

### Kept

- Document version numbers are **not** renumbered. The survivors stay
  `bootstrap-plan-v2`, `verification-context-v3` and `bootstrap-v2`. Retained
  review receipts under `reviews/` refer to these numbers, and renumbering
  would make that evidence ambiguous.
- Inner records that the survivors embed keep their shape. In particular the
  version-three context still nests the `ReleaseExpectations` record, and
  `config-v1`, `lock-v1`, `ci-policy-v1` and the trust contracts are current,
  not superseded.
- Every fail-closed behavior stays. An unknown version, catalog ID or context
  kind is still an error with no fallback. Removal narrows what is accepted; it
  never widens it.
- Historical receipts under `reviews/` stay as written. They describe the
  source they were taken from and are not rewritten to match the new surface.

### Rules for the removal work

1. **Reject, do not ignore.** After removal, a repository that still carries
   `.armorer/state.json` or `.armorer/journal.json` is refused with an explicit
   unsupported-version error. It is never treated as unmanaged, because that
   would let an old ownership record be silently re-adopted.
2. **One removal per reviewed change.** Each row in the table is its own pull
   request with its schema, examples, tests and documentation, so a regression
   is attributable.
3. **Keep the upgrade path tested.** `bootstrap-v1` to `bootstrap-v2` is today
   the only exercised catalog-to-catalog migration. Before `bootstrap-v1` is
   removed as a production authority, the upgrade tests move to a test-only
   predecessor catalog that production builds cannot select.
4. **Adversarial tests follow the boundary.** Tests that prove version-one
   packets cannot drive version-two writes are replaced by tests that prove
   version-one packets are rejected outright.
5. **Record the freeze.** The 0.1.0 release notes list the supported version of
   every document. That list is the baseline the immutability rule protects.

## Consequences

- Fewer schemas, commands and guards to review, test and document, and a single
  ownership model for the transaction-engine unification to target.
- A user who built an unreleased commit and applied a version-one plan must
  remove that state by hand or re-create it with `bootstrap`. No such user is
  known. The removal change must say so in its description.
- Evidence gathered against removed commands (for example the version-one
  apply and rollback receipts) stops being reproducible on main. It remains
  valid for the commit it names and earns no credit for later source.
- ADRs 0009, 0010 and 0011 remain accurate descriptions of why the successors
  exist. Their statements that predecessors are retained are superseded by this
  ADR for the period before 0.1.0 only.
- The runtime-version rule adopted on 2026-11-16 (patch releases share a
  runtime contract; see `docs/upgrades.md`) is unaffected.

## Alternatives considered

- **Keep everything and apply immutability from the first commit.** This is the
  current behavior. It preserves reproducibility of old receipts on main, at
  the continuing cost described above, to protect adopters who do not exist.
- **Remove and renumber survivors to version one.** A cleaner public surface,
  but it invalidates references in retained evidence and risks a reader
  confusing the new version one with the removed one.
- **Hide the old paths behind a feature flag.** The code, tests and review cost
  remain, and a second build configuration is added.
- **Defer until after 0.1.0.** After the tag the immutability rule applies and
  the removal is no longer available.

## Out of scope

Unifying the three transaction engines, merging the bootstrap and upgrade
ownership formats, promoting `bootstrap` subcommands to top-level names, and
the ecosystem adapter boundary are separate decisions. This ADR only clears the
superseded contracts that stand in their way.
