# Explicit multi-file bootstrap, version 2

`armorer bootstrap` provisions a pinned toolchain, catalog-backed lock, explicit
project CI policy and two unprivileged caller workflows. It is separate from the
frozen version-one toolchain-only `plan`/`apply` contract. All fourteen existing
version-one schemas retain their exact bytes. New schemas are
`bootstrap-plan-v2.json` and `ci-policy-v1.json`; structural validation does not
establish catalog trust or hosted/release readiness.

## Review and apply

Build this CLI with the exact installed Rust 1.95.0 toolchain and `cargo build --locked`.
Author `armorer.toml` with exact package/binary/feature/target selections. Author an
explicit project CI policy in a regular UTF-8 file. Every section is required:
`licenses.allow`, `advisories.exceptions`, `sources.allow_git` and
`bans.multiple_versions` / `bans.deny`. There is no generated license allowlist,
waiver, source permission or default for these decisions. The
[profile examples](../examples/bootstrap-v2/README.md) illustrate the format.
License tokens receive syntax checks; actual SPDX recognition and dependency
coverage belong to enforced cargo-deny CI. Advisory exceptions require a unique
RUSTSEC ID, named owner, nonempty reason and quoted ISO UTC expiry from the current
day through 90 days. Expired policies block planning and apply.

```sh
armorer --repository /path/to/project bootstrap check --policy /path/to/reviewed-policy.toml
armorer --repository /path/to/project bootstrap plan --policy /path/to/reviewed-policy.toml > /path/outside-project/bootstrap-plan.json
```

Both commands are read-only and offline. Discovery uses a bounded temporary Cargo
manifest snapshot with installed tools; it does not compile repository code,
execute build scripts, resolve dependencies, install tools or write consuming
files. `check` returns 2 because secure release gates remain open. `plan` returns 0
for a valid view even with conflicts. Inspect the complete JSON, including every
before/proposed byte, disposition, ownership flag and unified diff. Missing files
are distinct from empty text. CRLF, Unicode and final-newline absence remain exact;
a complete escaped view exceeding 1 MiB fails instead of truncating a conflict.

Approve `plan_sha256` through your normal review process, then supply that digest
independently. Reading a digest from an unreviewed file does not create approval:

```sh
armorer --repository /path/to/project bootstrap apply --plan /path/outside-project/bootstrap-plan.json --expect-plan-sha256 APPROVED_SHA256
```

The approved plan embeds the exact policy bytes; replacing the original external
policy file does not replace approved policy. Apply revalidates those embedded
bytes against the current UTC day. Generate a new plan to approve a policy change.

The five fixed targets are `rust-toolchain.toml`, `armorer.lock`,
`.armorer/ci-policy.toml`, `.github/workflows/armorer-ci.yml` and
`.github/workflows/armorer-build.yml`. Paths, templates and workflow commits are
compiled authority; there is no custom shell, hook, URL or force-adoption input.
The embedded catalog is authenticated against an independent compiled digest
before parsing. The lock records all 18 target-qualified native tool distributions
and the exact accepted workflow revision described in
[catalog source and limits](bootstrap-catalog.md). CI checks PRs/pushes; the build
caller is manual and unsigned. Neither caller has OIDC, publication or secrets.

## Ownership, repeatability and recovery

Differing unowned files conflict. Identical unowned files remain unowned. A file
written by bootstrap has its exact generated base in
`.armorer/bootstrap-state-v2.json`. Only a matching current base allows a reviewed
update; edits and deletions conflict even if an edit happens to equal the next
candidate. Bespoke workflows, deny.toml and other files are preserved. The plan
binds config bytes, discovered manifests/locks/target paths, license preimages,
legacy inputs, catalog, ownership and all managed preimages. Rust source content
is not hashed by metadata discovery and is not claimed as release provenance.
Stale preimages require a freshly reviewed plan. Replaying the identical successful
plan checks intent, ownership and all final bytes, and changes no file or timestamp.

Both writers use the persistent kernel lock `.armorer/apply.lock`. The transaction
guard explicitly unlocks when it leaves scope, allowing immediate replay even
while a concurrent child briefly retains a duplicate file descriptor. A separate
writer still rejects while the transaction is active. Version 2 has a separate `.armorer/bootstrap-journal-v2.json`; an unfinished journal blocks apply.
The engine validates all preimages under the lock, writes a durable journal before
any managed byte, creates required parents, atomically replaces each file and
writes ownership last. Updates preserve modes. It rechecks inputs and the entire
final inventory before persisting a durable commit marker. These local filesystem
checks require a stable, cooperatively edited workspace; the kernel lock serializes
Armorer processes, not arbitrary external editors or hostile filesystem mutation.

```sh
armorer --repository /path/to/project bootstrap recover --expect-plan-sha256 ORIGINAL_APPROVED_SHA256
```

Recovery reconstructs the complete fixed operation inventory from the approved
plan and original ownership, rather than trusting journal paths or claimed bytes.
Uncommitted work restores exact prior managed bytes; committed work finalizes only
matching final files, including unchanged unowned files. Every rollback preimage
is checked before restoring any file. Intervening edits stop recovery with the
journal preserved. A failed commit-marker write retains final bytes and the
recoverable journal because its durable state may be either committed or
uncommitted. Do not delete the journal or apply another plan to bypass a conflict.
Recovery may roll back an expired-policy transaction; expiry never prevents
restoring prior bytes. If an external edit blocks recovery, retain the journal,
identify the edit and restore a matching recorded before/after image through
explicit owner review, then retry with the original approved digest.

The empty apply-lock file and empty parent directories may remain after failure.
Rollback restores file contents and preserves existing modes, not inode identity,
original timestamps or directory absence. Unknown directories are never deleted.

## Compatibility and remaining gates

A version-one plan still authorizes only its single toolchain target and cannot be
submitted as a version-two plan. Existing `.armorer/state.json` requires an explicit
reviewed migration (#5); it is shown and preserved, not imported or force-adopted.
A legacy `rust-toolchain` blocks the new toolchain and is shown verbatim for review.
Recover unfinished v1 journals using v1 `recover` first. The updated v1 writer
refuses active v2 state/journals. Older CLI builds do not understand the v2 ownership
contract; switching back requires a separately reviewed rollback/migration.
Ownership also records the exact workflow repository/commit. Changed workflow,
catalog or runtime identities fail closed pending #5's reviewed upgrades.
The separate [reviewed upgrade command](upgrades.md) implements explicit v1/v2
ownership migration, supported imported workflows, stored generated-base previews
and historical reversal. It remains in review; migration between two independently
accepted catalogs still needs the future authenticated successor. Active upgrade
ownership/journals require that command; bootstrap planning/apply refuse them. No
moving reference is automatically adopted.

`configured: true` means these local bytes were provisioned. It does not mean a
missing Cargo.lock, unrecognized SPDX license, native dependency or Cargo override
has passed CI. The receipt separately leaves `ci_verified`, `release_rehearsed`,
`published` and `provenance_verified` false. GitHub capabilities remain unknown.
Real final-byte verification, protected Apple signing/notarization, immutable draft
publication and pilot rehearsals remain required. No SLSA level is earned by this
configuration transaction. PR acceptance/merge and public publication remain
separate human gates.
