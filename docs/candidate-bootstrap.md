# Equip the candidate. Keep the approval.

Candidate guide: [PR #21 at `0ef1145bf7d6`](https://github.com/brianluby/armorer/tree/0ef1145bf7d6fc8a92a276ace0fe13dc6dab0baa).
These commands are absent from the main source in the
[development map](development-status.md). Use a binary built from that exact
reviewed candidate. Keep production adoption behind its remaining review gates.

Know what you are carrying. **This is the way.**

## Inspect a training project

The candidate contains dependency-free `library`, `cli` and `service` fixtures in
`examples/bootstrap-v2/`. Each has a Cargo.lock, license and explicit fixture-only
CI policy. Copy one to temporary storage. Keep review packets outside the project.
Rust 1.95.0 must already be installed; discovery never installs it.

Set the first two paths to your reviewed checkout and its compiled binary:

```sh
candidate_checkout=/path/to/reviewed-candidate-checkout
armorer_candidate=/path/to/reviewed-candidate-binary
training_dir=$(mktemp -d)
cp -R "$candidate_checkout/examples/bootstrap-v2/cli" "$training_dir/project"
project="$training_dir/project"
policy="$project/ci-policy.reviewed.toml"
"$armorer_candidate" --repository "$project" bootstrap check --policy "$policy"
check_status=$?
test "$check_status" -eq 2
"$armorer_candidate" --repository "$project" bootstrap plan --policy "$policy" \
  > "$training_dir/bootstrap-plan.json"
```

`check` returns JSON and exit 2 because release gates remain open. Run that command
where an expected nonzero status will not abort your shell before you capture it.
`plan` returns 0 for a valid complete preview, including a preview with conflicts.
Both inspect without changing consumer files, compiling source, running build
scripts, resolving dependencies or downloading tools/catalogs. Repeat with the
other two profiles to inspect their selected packages and targets.

The example's MIT allowlist is its owner's fixture decision. Author your own
policy for a real project. All sections are explicit: `licenses.allow`,
`advisories.exceptions`, `sources.allow_git`, `bans.multiple_versions` and
`bans.deny`. Advisory exceptions need a unique RUSTSEC ID, owner, reason and quoted
UTC expiry from the current day through 90 days. Syntax checks do not establish
dependency coverage or successful enforced CI.

Read the whole plan. Each of five fixed targets has exact before/proposed text,
digests, disposition, ownership and diff:

- `rust-toolchain.toml`
- `armorer.lock`
- `.armorer/ci-policy.toml`
- `.github/workflows/armorer-ci.yml`
- `.github/workflows/armorer-build.yml`

Missing and empty files differ. Differing unowned files conflict; identical
unowned files remain unowned. Owned edits and deletions conflict. Bespoke workflow
files and existing Cargo.lock bytes stay intact. There is no force-adoption flag.
CRLF and final-newline absence retain their exact meaning; a preview beyond the
1 MiB bound fails without truncating the evidence.

Review every image before you accept the work. **This is the way.**

## Apply an independently approved packet

Obtain approval for the exact `plan_sha256` through the owner's review process.
Set `approved_bootstrap_sha256` to that independently approved digest; extracting
a hash from an unreviewed packet supplies no approval.

```sh
"$armorer_candidate" --repository "$project" bootstrap apply \
  --plan "$training_dir/bootstrap-plan.json" \
  --expect-plan-sha256 "$approved_bootstrap_sha256"
```

Apply checks current inputs, all managed preimages, catalog authority and embedded
policy expiry before writing. Changed intent or preimages require a new plan and
approval. Replacing the external policy file cannot alter the approved embedded
policy. Ownership is retained in `.armorer/bootstrap-state-v2.json`; replay checks
exact final bytes and ownership without changing bytes or timestamps.

The generated CI caller handles PRs/pushes. The build caller is manual and
unsigned. Both remain unprivileged: no OIDC, publication or secrets. A receipt's
`configured: true` describes local provisioning. `ci_verified`,
`release_rehearsed`, `published` and `provenance_verified` stay false.

Keep local configuration and earned release evidence distinct.
**This is the way.**

## Recover the matching transaction

Retain the original approval and `.armorer/bootstrap-journal-v2.json`. Use the
same candidate and original digest:

```sh
"$armorer_candidate" --repository "$project" bootstrap recover \
  --expect-plan-sha256 "$approved_bootstrap_sha256"
```

An uncommitted journal restores exact prior managed bytes. A committed journal
finalizes only matching final files and ownership. Every preimage is checked
before any restoration. Intervening edits stop recovery and preserve the journal;
review those edits and restore a recorded before/after image before retrying.
Policy expiry cannot prevent uncommitted recovery. A commit-marker error retains
final bytes and journal because persistence may be uncertain.

Do not delete a journal to clear a conflict. `.armorer/apply.lock` serializes
Armorer writers; it cannot serialize arbitrary editors. Empty parents and the
lock may remain, and byte restoration does not promise inode or timestamp
restoration. Recover v1 transactions with v1 `recover`; use
[candidate upgrades](candidate-upgrades.md) for explicit ownership migration.

The [source contract and fault coverage](https://github.com/brianluby/armorer/blob/0ef1145bf7d6fc8a92a276ace0fe13dc6dab0baa/docs/bootstrap-v2.md)
give the detailed version limits. The
[integration review](../reviews/2026-10-02-pr21-context-ordering.md) records exact-head
tests, open findings and acceptance boundaries. Hosted production, final-byte
producer evidence, publication and pilots remain separate gates.

Preserve the journal. Protect the project. **This is the way.**
