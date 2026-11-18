# ADR 0013: the compiled catalog defines the managed file set

Status: implemented proposal for review, 2026-11-17. Human acceptance and hosted
validation are separate gates; no release claim.

The managed file set was a compile-time array of five paths in the bootstrap
module. Plan validation, bootstrap and upgrade ownership, journal bounds, import
scope and the license-path guard each compared against it. One more managed file,
or a second ecosystem with a different set, would touch every site and change the
plan contract (review finding A2).

Each compiled catalog entry now declares its set as one ordered table of path and
content source: caller template, toolchain template, rendered lock, approved
owner policy bytes or fixed template. Rendering is driven by that table, and the
sorted managed paths are derived from it, so the declared and rendered sets
cannot diverge. The global constant is removed. A bootstrap plan is checked
against the catalog it names, bootstrap ownership against the catalog of its
workflow pin, and a bootstrap journal against its embedded plan. Upgrade import
scope and ownership use the target and recorded authority; an upgrade journal is
bounded by the union of its source and target sets. Guards that run before any
catalog is known use the union of every compiled set.

An upgrade covers the target's rendered files plus every owned file the target no
longer renders. A file only the target renders is an ordinary create. No merge
rule defined an owned file absent from the target, so it is now a blocking
conflict: its candidate is shown as absent and nothing is proposed. It is never
silently deleted or silently released. With equal sets the scope is the target's
set, and existing plans, states, journals, locks, receipts, errors and schemas
are byte-for-byte unchanged.

The test-only `bootstrap-test-wide` catalog, compiled only with
`test-predecessor`, is `bootstrap-v2` plus one fixed file at revision 3, pinned to
a clearly fake workflow commit. Tests drive upgrade, replay, refresh, rollback,
downgrade conflict and forged-inventory rejection through it. Production builds
reject the ID like `bootstrap-v1`, and the development workflow asserts that.

Adding a production file needs a table entry, a template, regenerated examples
and a reviewed catalog successor, since it changes every consumer's plan. See
[the catalog contract](../bootstrap-catalog.md) and `tests/file_set.rs`.
