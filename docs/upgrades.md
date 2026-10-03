# Reviewed upgrades, imports and historical rollback

The `upgrade` commands use new `upgrade-plan-v1.json` and
`upgrade-rollback-v1.json` contracts. Frozen v1 toolchain-only plans and v2
bootstrap plans retain their fields and write permissions. They cannot authorize
an upgrade or accept either new packet. This implementation is delivered for
review; it does not establish acceptance, CI/release readiness or a SLSA level.

## Select and review an upgrade

Start with [explicit configuration and policy](bootstrap-v2.md). Discovery needs
the exact selected Rust toolchain already installed. A missing toolchain or unknown
configuration/ownership version fails explicitly; planning never installs one,
executes repository code, resolves dependencies, downloads catalogs, or writes
consuming files. The examples use the installed Rust 1.95.0 toolchain.

```sh
armorer --repository /path/to/project upgrade plan --policy /path/to/reviewed-policy.toml --catalog bootstrap-v1 > /path/outside-project/upgrade.json
```

The catalog ID is mandatory. Currently only immutable `bootstrap-v1` is supported:
workflow repository `brianluby/armorer-workflows`, commit
`772ca83e386c883c88cc3b936d69f8cb3216c91e`, and independently compiled tool-catalog
SHA-256 `e21e6cbd2cdd1125dbb9817116b1f46d043807b4d92128abc9dc07256dbb4b13`.
Every embedded catalog is authenticated before parsing. A future reviewed source
update must add a new stable catalog ID and preserve this entry's meaning. No
accepted second catalog is invented from Git ancestry. Actual migration between
two accepted workflow/catalog revisions remains an integration gate when the
release runtime produces that successor. Ordinal downgrade unit tests do not
establish such operational evidence. Unknown IDs, moving branches/tags, source
substitution and failed catalog authentication never trigger import or downgrade
fallback; `--allow-downgrade` cannot make an unknown authority trusted.

Review the complete JSON. Each of the five fixed local targets shows its stored
generated base, exact current bytes, target-generated candidate, proposed bytes,
all digests, merge decision and both candidate/proposed unified diffs. Pin changes
include removals and additions of target-qualified native distributions; workflow
before/after and compatibility/policy changes are explicit. Before pins read from
an existing lock are syntax-checked declarations, never authenticated historical
catalog roots. Source ownership, exact config, bounded Cargo discovery, license and
legacy-toolchain input hashes, policy bytes and UTC review day are bound into the
approval digest. A view larger than 1 MiB fails without truncating any image.

A generated base equal to current bytes allows a candidate update. If only current
bytes changed while the candidate equals the base, supported owner customization
is preserved. Changes to both current and candidate, deletion of an owned file,
unsupported customization or differing unowned bytes produce explicit conflicts.
Editing an owned file to the candidate does not silently readopt it. Compatibility
reports distinguish supported, blocked and manual-review prerequisites. Missing
Cargo.lock/license, declared native build scripts or excluded Cargo overrides
still need review before CI/build execution; local provisioning cannot verify them.

Supported workflow customization is intentionally narrow: an exact compiled caller
prefix followed by additional unique jobs with two-space job headers and children
indented at least four spaces. Job headers cannot use protected `verify`/`build`
identities, directives, anchors or tabs. Outdented YAML and edits to the compiled
trigger, permissions, trusted job or immutable workflow pin conflict. These bytes
are preserved, not reformatted. This is a bounded supported shape, not a general
YAML merge engine. Separate bespoke workflow files remain untouched. Owner policy
changes can be preserved only when the generated policy remains unchanged and the
result is supported and valid under current UTC expiry rules. If both policies
change, resolve the conflict through owner review before generating a fresh plan.

## Explicit imports and apply

V1 ownership migrates only its recorded toolchain. V2 ownership supplies its five
recorded generated bases; those workflow/lock bases are checked against independent
compiled catalog identity rather than trusting their self-declared hashes. New
upgrade ownership retains generated and approved customized images separately, so
later previews still use the actual generated base.

Identical unowned files stay unowned by default. Import each supported file through
its own flag, then review its before and proposed images:

```sh
armorer --repository /path/to/project upgrade plan --policy /path/to/reviewed-policy.toml --catalog bootstrap-v1 --import-file .github/workflows/armorer-ci.yml --import-file armorer.lock > /path/outside-project/upgrade.json
```

An import may adopt an identical compiled target, a supported appended-job caller,
or a strictly supported v1 lock declaration that the target catalog replaces.
Differing bespoke policies/toolchains, moving-pin locks and unsupported workflow
shapes conflict. Already-owned files cannot be reclassified as imports. Imports
never waive unknown runtime/schema versions or contradictory catalog/base identity.

After normal human review, supply the approved digest independently:

```sh
armorer --repository /path/to/project upgrade apply --plan /path/outside-project/upgrade.json --expect-plan-sha256 APPROVED_UPGRADE_SHA256
```

Apply rederives every input, merge, pin change and compatibility claim. It enforces
current policy expiry; a plan reviewed on a different UTC day requires a fresh
plan. Changing the original external policy file does not replace the policy bytes
already approved in the packet. It shares `.armorer/apply.lock` with earlier
writers, stages `.armorer/upgrade-journal-v1.json`, atomically writes changed
managed files, removes only the exact former ownership preimage and writes
`.armorer/upgrade-state-v1.json` last. It rechecks inputs and every final byte before
the durable commit marker. All earlier writers refuse an active upgrade owner or
journal. Replaying the identical successful packet verifies config, ownership and
all final bytes and leaves file bytes/timestamps unchanged.

Updates preserve existing modes. Byte rollback does not promise original inode,
timestamps or directory absence; a deleted ownership file recreated during
migration recovery uses the platform's creation mode. The persistent lock and
empty parent directories may remain. Kernel locking serializes Armorer writers;
these checks assume a stable, cooperatively edited filesystem and cannot serialize
arbitrary editors or hostile filesystem mutation.

## Interrupted migration and separately reviewed reversal

For an interrupted upgrade, retain the journal and use its original approved
upgrade digest. Recovery reconstructs the full fixed inventory, including owner
deletion/replacement. It checks all targets (including unchanged ones), all owner
slots and exact journal bytes before restoring anything. An intervening edit blocks
the entire restoration with every partial byte preserved. Historical policy expiry
cannot prevent uncommitted recovery. A commit-marker error retains final bytes and
the journal because the durable marker may or may not have persisted.

```sh
armorer --repository /path/to/project upgrade recover --expect-plan-sha256 ORIGINAL_APPROVED_UPGRADE_SHA256
```

A committed journal finalizes only exact final files/ownership. If configuration,
Cargo inputs or current policy changed after commit, finalizing those historical
bytes does not report current configured readiness. Never delete a journal or
force-import state to bypass recovery. When an edit blocks recovery, preserve the
journal, review that edit and restore an exact recorded before/after image before
retrying with the original approval.

Reverting a completed upgrade is a distinct read-only review and apply operation.
It requires the original independently approved upgrade packet, its exact final
images/ownership, and explicit historical downgrade intent:

```sh
armorer --repository /path/to/project upgrade rollback-plan --upgrade-plan /path/outside-project/upgrade.json --expect-upgrade-plan-sha256 ORIGINAL_APPROVED_UPGRADE_SHA256 --allow-downgrade > /path/outside-project/rollback.json
armorer --repository /path/to/project upgrade rollback-apply --plan /path/outside-project/rollback.json --expect-plan-sha256 SEPARATELY_APPROVED_ROLLBACK_SHA256
```

The reverse packet includes the original approval, exact reverse byte/digest/diff
inventory, ownership restoration/deletion and limitations. It has its own approval
digest. Edits after the upgrade or after reverse review require a fresh decision;
there is no force flag. Configuration, Cargo inputs and the external source policy
are not reverted. Restoring a previous lock can therefore leave its old config
binding incompatible with current intent. Before enabling CI, re-run check/plan,
review the restored ownership/version and explicitly reconcile that binding.
Historical lock declarations remain declarations; restoration is never evidence
of their authenticity or a fallback after a failed verification.

A reversal uses the same kernel lock and upgrade journal. An interrupted reverse
uses `upgrade recover --expect-plan-sha256 SEPARATELY_APPROVED_ROLLBACK_SHA256`.
Uncommitted reverse recovery restores the upgraded bytes; a committed reverse
finalizes the exact older images. Repeat application checks every restored byte.
`configured` remains false for historical reversal and uncommitted recovery. Local
upgrade success leaves `ci_verified`, `release_rehearsed`, `published` and
`provenance_verified` false. Actual crypto, protected Apple signing, immutable
publication, genuine adversarial integration and both pilots remain separate gates.

[Profile review packets](../examples/upgrades/README.md) and the fault tests cover
supported v1/v2 migration, imports, customizations, rollback, old-version rejection,
concurrency and every write/owner/commit-marker boundary in both directions.

## Runtime version boundary

`bootstrap-v1` freezes runtime `0.1.0` as part of its independently reviewed
authority. The compiled runtime must match that entry before an upgrade plan can
be made or accepted. A crate version bump fails with an explicit catalog-successor
error; it does not reinterpret this ID or widen historical lock acceptance. A new
runtime requires a separately reviewed catalog ID and explicit migration tests
for stored generated bases, ownership state and rollback.
