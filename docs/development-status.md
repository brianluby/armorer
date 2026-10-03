# What stands. What is still being forged.

This is a development snapshot from **2026-10-02, America/Los_Angeles**.
The accepted main source is `d5f048240db04fb40d73515e571a4028f049b973`.
Thirteen open Armorer PRs were inspected at the exact heads listed below.
Update this map when those heads or merge states change.

**This is the way.**

## The current main implementation

Main exposes `check`, `plan`, `apply`, `recover` and `schema`.
Follow [the command reference](cli-reference.md). Apply manages only the toolchain;
release contracts validate structure and consistency. No SLSA level is claimed.

## Work under review

Every row below is **unmerged** at this snapshot. Branch guides are linked by full
commit so that their commands stay tied to the implementation they describe.
A passing PR check establishes evidence for that candidate, not acceptance of the
whole stack or a completed release.

| PR | Base | What it proposes | Exact head / branch guide |
| --- | --- | --- | --- |
| [#6: Exact bounded setup previews](https://github.com/brianluby/armorer/pull/6) | main | Read-only exact before/after content and complete diffs; saved-plan digest checking | [`90784cfcbd81`](https://github.com/brianluby/armorer/blob/90784cfcbd816567e969ac6a4721ebb58ced5ca0/docs/apply.md) |
| [#7: Bootstrap catalog authority](https://github.com/brianluby/armorer/pull/7) | main | Compiled catalog authority, 18 target-qualified tool pins and fixed unprivileged callers | [`3496ad1787bf`](https://github.com/brianluby/armorer/blob/3496ad1787bf9adf318d840a397e695315e56ea3/docs/bootstrap-catalog.md) |
| [#8: Versioned bootstrap provisioning](https://github.com/brianluby/armorer/pull/8) | #7 | Explicit policy and transactional provisioning of five local targets | [`241d09e28a64`](https://github.com/brianluby/armorer/blob/241d09e28a6405cf2f0f215fde698c4f60f1b190/docs/bootstrap-v2.md) |
| [#9: Reviewed upgrades and rollback](https://github.com/brianluby/armorer/pull/9) | #8 | Scoped imports, conservative three-way decisions, ownership migration and separately approved reversal | [`e184062d61a7`](https://github.com/brianluby/armorer/blob/e184062d61a73a72e7d7fefdedb65523bc138820/docs/upgrades.md) |
| [#10: Offline attestation verification](https://github.com/brianluby/armorer/pull/10) | main | Pinned native gh, approved roots and exact certificate/source/signer/run/subject checks | [`4b9e8c4f9917`](https://github.com/brianluby/armorer/blob/4b9e8c4f9917625bd0f9f0e93ff5b79390d1d046/docs/offline-verifier.md) |
| [#11: Selected Cargo graph v2](https://github.com/brianluby/armorer/pull/11) | #10 | Independent graph context and exact Cargo component, dependency and scope reconciliation | [`919a94892531`](https://github.com/brianluby/armorer/blob/919a9489253176e15b8bf62b91176fe230c749c2/docs/cargo-graph-v2.md) |
| [#12: Complete SBOM validation](https://github.com/brianluby/armorer/pull/12) | #11 | Pinned native CycloneDX validation of the whole 1.5 document | [`178e9d39b2f3`](https://github.com/brianluby/armorer/blob/178e9d39b2f323042c50e30dd476f9c515112f36/docs/complete-sbom-validation.md) |
| [#13: Authenticated inventory consumer](https://github.com/brianluby/armorer/pull/13) | #12 | Independent approved context; inventory-first authentication and complete asset/evidence checks | [`b133a9288589`](https://github.com/brianluby/armorer/blob/b133a9288589e8e24fcb2f23be3a6fda05f2d0e3/docs/authenticated-inventory.md) |
| [#14: Exact historical byte comparison](https://github.com/brianluby/armorer/pull/14) | #13 | Explicit independently approved historical allowlist; no provenance or signature claim | [`30d26139b4c7`](https://github.com/brianluby/armorer/blob/30d26139b4c7424f62a3fa0f461321bd7c9560d1/docs/historical-verification.md) |
| [#15: Modern builder identity](https://github.com/brianluby/armorer/pull/15) | #14 | Exact reusable-workflow builder identity and independent hosted-runner assertions | [`005f08099a2b`](https://github.com/brianluby/armorer/blob/005f08099a2b08681080a4e532b3615f3232d449/docs/offline-verifier.md) |
| [#16: Native catalog and runtime v2](https://github.com/brianluby/armorer/pull/16) | #15 | Archive versus native member identities, explicit context v2 and bounded common runtime loading | [`c649051a04e4`](https://github.com/brianluby/armorer/blob/c649051a04e49c25885502db65b1aeb6f14a6bba/docs/runtime-native-v2.md) |
| [#17: Current provenance build type](https://github.com/brianluby/armorer/pull/17) | #16 | Exact qualified current workflow build-type URI while retaining strict legacy behavior | [`cd47288b3b17`](https://github.com/brianluby/armorer/blob/cd47288b3b17268af079e882614431f7520caec3/docs/offline-verifier.md) |
| [#18: Complete-release CLI](https://github.com/brianluby/armorer/pull/18) | #17 | Explicit-context verify-release command over the strict offline consumer | [`a5d19efa10b3`](https://github.com/brianluby/armorer/blob/a5d19efa10b3b5596930ad5eedb75ce7c18598a0/docs/verify-release-cli.md) |

## Integrate with care

The bootstrap path is #7 → #8 → #9. The verification path is #10 → #11 → #12 →
#13 → #14 → #15 → #16 → #17 → #18. Preview PR #6 branches independently from main.
The two stack tips conflict in the development workflow, CLI and module exports;
preview also overlaps the CLI. Two independent proposals currently use ADR 0009.
Resolve these as reviewed integration work, retaining every command and validation
gate. Details and exact review evidence are in
[the open-PR review packet](../reviews/2026-10-02-open-prs.md).

## Earn the release

All thirteen current PR heads have passing hosted Rust checks. All existing inline
review threads were resolved at the snapshot. Several automated reviews were
skipped or failed; those outcomes supply no approval. Local review and tests do
not establish a complete own signed producer rehearsal or production trust approval.

Production catalogs and roots, a qualified final-byte producer, the complete
signed non-publishing rehearsal, protected Apple finalization, immutable
publication, both pilots and human acceptance remain separate gates. Historical
byte matching cannot substitute for authenticated provenance. v0.1 targets SLSA
Build L2; L3 stays on the future-version backlog.

Hold each gate until its evidence stands. **This is the way.**
