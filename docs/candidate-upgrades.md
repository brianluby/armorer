# Change the armor. Retain the old plates.

Candidate guide: [PR #21 at `c815a065fb08`](https://github.com/brianluby/armorer/tree/c815a065fb08dbbdc0321b4a80bed66f12da9f64).
Use the exact candidate binary, an isolated consumer and independently reviewed
policy. These commands are absent from the current main CLI. Begin with
[candidate bootstrap](candidate-bootstrap.md) for the example paths and policy.

Every change needs its own approval. **This is the way.**

## Review the selected authority and images

Only the immutable catalog ID `bootstrap-v1` is compiled into this candidate.
It identifies workflow commit `772ca83e386c883c88cc3b936d69f8cb3216c91e` in
`brianluby/armorer-workflows` and tool-catalog SHA-256
`e21e6cbd2cdd1125dbb9817116b1f46d043807b4d92128abc9dc07256dbb4b13`.
A moving tag, branch or unknown catalog fails. `--allow-downgrade` cannot approve
an unknown authority. No accepted second catalog or future runtime compatibility
is established by this exercise.

```sh
"$armorer_candidate" --repository "$project" upgrade plan \
  --policy "$policy" --catalog bootstrap-v1 \
  > "$training_dir/upgrade-plan.json"
```

Planning is read-only and offline. It never installs a missing toolchain, executes
repository code, resolves dependencies or writes consumer files. Review all five
fixed targets: stored generated base, current image, candidate image, proposed
image, exact digests, merge decision and candidate/proposed diffs. Inspect pin
additions/removals, workflow changes and compatibility prerequisites too.

If current bytes equal the generated base, a candidate update is possible. If
only current bytes changed and the candidate still equals the base, supported
customization can survive. Changes to both current and candidate, deleted owned
files and unsupported shapes conflict. Copying candidate text into an owned
file does not silently establish ownership.

Supported appended workflow jobs must follow the exact compiled caller prefix,
use unique two-space job headers and children indented at least four spaces.
Protected `verify`/`build` jobs, trigger/permission/pin edits, directives, anchors,
tabs and outdented YAML conflict. Separate bespoke workflow files remain intact.
This is a bounded merge rule; inspect preserved bytes rather than assuming a
general YAML merge.

Keep the generated base. Review the owner's changes. **This is the way.**

## Import deliberately, then apply

V1 ownership migrates only its recorded toolchain. V2 supplies five generated
bases, checked against compiled catalog authority. Identical unowned files stay
unowned unless each supported import is explicitly named:

```sh
"$armorer_candidate" --repository "$project" upgrade plan \
  --policy "$policy" --catalog bootstrap-v1 \
  --import-file .github/workflows/armorer-ci.yml --import-file armorer.lock \
  > "$training_dir/upgrade-plan.json"
```

Imports accept an identical compiled target, supported appended-job caller or
supported v1 lock declaration. They cannot waive a moving pin, unknown version,
contradictory catalog/base or unsupported customization. Historical lock hashes
are declarations; import does not authenticate them.

Obtain approval for the complete packet and supply its digest independently:

```sh
"$armorer_candidate" --repository "$project" upgrade apply \
  --plan "$training_dir/upgrade-plan.json" \
  --expect-plan-sha256 "$approved_upgrade_sha256"
```

The plan binds configuration, discovery inputs, license/legacy preimages,
ownership, embedded policy and UTC review day. Apply rederives merge, pins and
compatibility. A different UTC day requires a fresh plan. The external policy
cannot replace approved embedded bytes. Local success still leaves CI,
rehearsal, publication and provenance claims false.

Apply shares `.armorer/apply.lock`, writes `.armorer/upgrade-journal-v1.json`
before changes and `.armorer/upgrade-state-v1.json` last, then verifies final
bytes before the durable commit marker. Replay checks every final byte. Earlier
writers refuse active upgrade state/journals.

## Recover an interruption or review a reversal

An interruption uses the digest of the transaction actually in the journal:

```sh
"$armorer_candidate" --repository "$project" upgrade recover \
  --expect-plan-sha256 "$approved_upgrade_sha256"
```

Uncommitted recovery restores prior bytes and ownership; committed recovery
finalizes matching final images. All targets and owner slots are checked before
any restoration, including unchanged ones. An intervening edit blocks the whole
restoration with partial bytes and journal preserved. Historical policy expiry
cannot block uncommitted recovery. Do not delete the journal or force-import state.

A completed upgrade needs a separately reviewed reverse packet. Retain the
original approved upgrade packet and exact final images:

```sh
"$armorer_candidate" --repository "$project" upgrade rollback-plan \
  --upgrade-plan "$training_dir/upgrade-plan.json" \
  --expect-upgrade-plan-sha256 "$approved_upgrade_sha256" --allow-downgrade \
  > "$training_dir/rollback-plan.json"
"$armorer_candidate" --repository "$project" upgrade rollback-apply \
  --plan "$training_dir/rollback-plan.json" \
  --expect-plan-sha256 "$approved_rollback_sha256"
```

Review and independently approve the rollback digest before the second command.
An interrupted rollback uses `upgrade recover` with `approved_rollback_sha256`.
Uncommitted reverse recovery restores upgraded bytes; committed reverse recovery
finalizes older images. Byte rollback preserves existing modes but promises no
original inode, timestamps or directory absence.

Rollback leaves configuration, Cargo inputs and the external source policy in
place. An older lock may therefore disagree with current intent. Re-run inspection
and reconcile that binding before enabling CI. `configured` stays false for
historical reversal and uncommitted recovery. Restored bytes establish no
historical authenticity or safe fallback after failed signature verification.

The [exact source guide](https://github.com/brianluby/armorer/blob/c815a065fb08dbbdc0321b4a80bed66f12da9f64/docs/upgrades.md)
and [integration review](../reviews/2026-10-02-integration-pr21.md) retain detailed
contracts, fault-test evidence and the missing two-accepted-catalog gate.

Keep both approvals. Keep every receipt. **This is the way.**
