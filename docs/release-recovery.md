# Keep the draft. Protect the published record.

Design-stage operator guide for partial uploads, conflicts, alias failure and
post-publication incidents. The current
[publication contracts](trust-contracts-v1.md#github-and-future-registry-receipts)
validate receipt consistency and retry identity. Main and the combined candidate
have no publication, upload-resume or alias-update CLI. Backend execution,
remote race checks and fault qualification remain required before this becomes
a supported executable recovery runbook.

Preserve the identity before deciding the next action.
**This is the way.**

## Establish the observed state

Retain repository, full source commit/ref, stable tag, config/lock/runtime digests,
workflow pins, run/attempt, inventory/policy bytes, release ID and the actual remote
state. Record exact asset names, digests and sizes; upload success messages do not
prove served bytes. Keep original owned receipts, approvals and failed-stage
evidence. A repeated workflow run does not establish ownership of an old draft.

For a read-only metadata observation, choose an explicit repository and stable tag:

```sh
release_repository=OWNER/REPO
release_tag=v0.1.0
gh release view "$release_tag" --repo "$release_repository" \
  --json databaseId,tagName,isDraft,isImmutable,publishedAt,targetCommitish,assets,url
```

This reports metadata only. It does not authenticate source ancestry, asset bytes,
ownership, approval or immutability configuration. Preserve the observation time
and errors. The supported fields are documented in
[the CLI release-view interface](https://cli.github.com/manual/gh_release_view).
Unknown/denied state cannot satisfy a publication prerequisite. Use
the [platform guide](platform-prerequisites.md) for effective settings and the
[candidate verifier](candidate-verification.md) for exact local file verification.
Neither a local consumer result nor this metadata observation authorizes publishing.

## Choose the recovery branch

| Observed situation | Required decision and evidence | Forbidden shortcut |
| --- | --- | --- |
| Exact owned draft; some uploads missing | Match complete retry identity; resume only missing identical assets, then download/verify the complete set again | Replacing an existing asset or borrowing another run's ownership |
| Existing asset has different bytes/size, or draft owner/identity is unknown | Record conflict; investigate the owned draft or review a new version | Clobbering, deleting evidence or relabelling another attempt as the original |
| Upload outcome is uncertain | Re-observe exact remote identities/bytes before deciding whether anything remains missing | Inferring success from timeout or blindly uploading again |
| Draft verified earlier; approval or setting observation is now stale | Obtain fresh valid approval/setting evidence and recheck current draft identity immediately before publication | Treating the earlier observation as permanently valid |
| Exact expected immutable release is already published | Independently verify served membership/bytes and provenance; retain the observed published result | Resuming it as a draft |
| Published bytes/evidence fail verification | Declare a release incident; hold promotion; review a corrected new version | Overwriting published assets, moving its stable tag or rewriting the old receipt |
| Publication verified; optional alias update failed | Reverify the immutable release, then review/resume only the separately authorized alias operation | Republishing or claiming the alias moved without readback |

These are design decisions, not backend commands. `GithubReceipt::same_owned_retry`
accepts only `owned-draft`, `uploaded`, `draft-bytes-verified` or `approved` states
with identical full inputs, tag, release ID, inventory and policy. Changed run
attempt, source or inventory conflicts. Published states cannot resume as drafts.
`validate` checks required receipt fields; it neither contacts GitHub nor proves
their content authentic. See [the source](../src/trust/publication.rs).

Keep identical retries exact. Keep conflicts visible. **This is the way.**

## Restore readiness after a draft interruption

The qualified backend must serialize release/tag work and authenticate ownership,
not merely rely on a concurrency-group name. Retain exact expected inventory,
current remote release/asset identities, fresh capability evidence and independent
approval. Before publishing, download and verify the complete served draft,
including Apple checks on macOS when required, then recheck identity/inventory
immediately before the protected transition. Missing/extra or partial assets fail.

If corrected evidence belongs to a new source, configuration, policy, inventory
or run/attempt, record that new identity and obtain a new decision. Do not mutate
an old receipt to make `same_owned_retry` accept it. A recovery action that writes
remote state requires explicit owner authorization for that concrete action.

The architecture requires current immutable settings, protected stable refs,
effective consuming-repository approvals, served-byte verification and subsequent
release-attestation/provenance verification. The
[release-readiness gates](release-readiness.md) remain open until each has authentic
evidence. Schema tests do not qualify these remote transitions.

## Respond to a published failure

Begin the [incident record](release-incident-template.md). Preserve the published
release and its original receipts. Record whether the failure concerns bytes,
membership, provenance, SBOM, platform checks, roots/policy or a verifier defect.
Identify exactly which artifacts and downstream references are affected; a
passing sibling target is not acceptance of the failed release.

The owner must decide containment, reporting channel and corrected version. Hold
downstream alias promotion while the release is unverified. Prepare remediation
in an isolated branch/worktree and rerun required source, producer, native consumer
and non-publishing rehearsal gates. A new release needs a new stable version,
full expected inventory and independent approval. Retain the failed version's
status and limitations; a successful correction does not rewrite its history.

This guide makes no automatic release deletion, yanking, revocation or notification
decision. Those actions need a reviewed scope, applicable platform contract and
explicit owner authorization. The private security reporting channel remains a
[pending policy decision](security-reporting-draft.md).

Protect the next version. Preserve what happened. **This is the way.**

## Resume only the authorized alias change

Optional moving major tags are separate from immutable stable tags. The design
advances only the newest stable version in a major; prerelease and older backport
versions cannot downgrade that alias. Generated consumers still pin immutable
workflow commits. A qualified alias adapter must bind old/new tag object and
peeled commit, expected release, authorization and post-write readback, with
explicit conflict handling. That adapter and its race/failure tests are pending.

An alias error cannot undo or authorize replacement of a published release.
Record `publication verified; alias pending` when supported by the separate
evidence, and retain the error until an authorized operation and exact readback
establish the new alias. Follow the
[architecture's retry rules](../ARCHITECTURE.md#draft-publication-retries-and-verification).

Every transition earns its own receipt. **This is the way.**
