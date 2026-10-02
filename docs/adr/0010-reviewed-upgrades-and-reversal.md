# ADR 0010: separate upgrade approval, stored generated bases and exact reversal

Status: implemented proposal for #5 review, 2026-10-02. Human acceptance, hosted
validation and migration between two accepted catalogs are separate gates.

Frozen v1/v2 approval and journal meanings cannot gain migration writes. Introduce
`armorer-upgrade-plan` version 1, an independently approved exact packet over the
five fixed generated files, source ownership, intent/discovery/preimages, current
UTC policy and immutable compiled target authority. Validate every display/merge
claim from those inputs and rederive the current snapshot before apply.

Retain generated and applied images separately in version-one upgrade ownership.
Use conservative three-way equality: update an unchanged base, preserve supported
customization when the candidate is unchanged, otherwise conflict. Support appended
jobs only behind an exact protected caller prefix, and explicit individually scoped
imports only for recognized shapes. Unknown schemas/runtime/authority, contradicting
generated bases and authentication errors never become unowned import candidates.

An immutable catalog ID is compiled authority; source files and existing declared
pins cannot nominate it. Only the authenticated initial catalog exists now. A real
successor must be independently reviewed and retained alongside the initial entry;
synthetic ordinals do not satisfy operational upgrade/downgrade acceptance.

Share the persistent kernel apply lock. Journal changed files plus exact former
owner deletion and new ownership last; bound and reconstruct the operation list.
Prevalidate every managed/ownership slot and the journal before restoring any byte.
Commit-marker uncertainty keeps final bytes and journal for explicit recovery.

Completed-upgrade reversal is a separately versioned `armorer-upgrade-rollback`
packet with its own approval digest and explicit downgrade intent. Bind the original
approved plan and complete reverse images; require exact current upgrade output.
Use the same journal with an explicit operation discriminator. Reverse never alters
caller intent/discovery or promotes historical declarations to trust roots; report
configured/release/provenance readiness conservatively. Restore prior approved
local bytes even when policy has since expired. Preserve intervening edits rather
than forcing restoration. Read-only paths never execute consuming builds.

See [operator contract and limits](../upgrades.md) and upgrade tests for faults,
process interruptions, state migration/deletion, marker uncertainty, forged journal
inventories, imports, custom jobs/policy, replay, concurrency and reversal.
