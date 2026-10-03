# Prepare the gates before the release

Know the account. Inspect the effective settings. Keep the observation with its
source and time. **This is the way.**

GitHub documentation was checked on **2026-10-02, America/Los_Angeles**.
This guide is a manual preparation checklist. Main check/plan do not contact
GitHub, configure settings or authenticate capability observations. A completed
checklist does not constitute a producer rehearsal or secure publication result.

The workflow repository now has a separately labelled
[candidate prerequisite observer](candidate-workflow-prerequisites.md#observe-configuration-without-granting-authority).
It performs fixed authenticated native reads and retains blocking states, while
all current-attempt, signing and publication authority stays false. It is not
integrated into current main check/plan or the catalog's pinned callers.

## Check account and visibility

For GitHub.com on current plans, the documented platform matrix is:

| Repository / plan | Artifact attestations | Required environment reviewers | Environment secrets |
| --- | --- | --- | --- |
| Public / any current plan | Available | Available | Available |
| Private / Free | Unavailable | Unavailable | Unavailable |
| Private / Pro or Team | Unavailable | Unavailable | Available |
| Private or internal / Enterprise Cloud | Available | Verify effective enterprise environment policy | Available with supported environment configuration |

Legacy Bronze/Silver/Gold plans do not support artifact attestations. Private or
internal attestations require Enterprise Cloud. See
[GitHub's attestation eligibility](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations).
Environment feature eligibility differs by plan; inspect
[GitHub's environment rules and secrets](https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments).

Armorer's current public-good evidence-slot verifier has genuine positive fixture
qualification. Private-backend positive qualification remains incomplete;
internal visibility and Enterprise Server are unsupported by that adapter.
Platform eligibility alone cannot establish Armorer support. See
[the verifier limits](offline-verifier.md#execution-and-result-checks).
An unsupported secure-release backend cannot silently become checksum-only mode.
An explicit CI-only configuration mode remains future work; v1 config accepts
only `attestations = "required"`.

Name the capability you have. Retain the one you lack.
**This is the way.**

## Inspect protected execution

Record exact protected source/tag rules, required checks and review ownership for
workflow/runtime changes. Record which consuming repository supplies the effective
`release-signing` and `release-publish` environments, their approved refs,
reviewers, self-review policy and administrator bypass behavior. These are Armorer
design names; the final producer adapters still need integration and qualification.

GitHub's “protected branches only” environment option allows all branches if no
branch protection exists. Required reviewers gate access to environment secrets.
Inspect explicit branch/tag rules and effective bypass behavior rather than relying
on an environment name. See
[the environment reference](https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments).

Keep ordinary PR/build jobs unprivileged. Elevated attestation, signing and
publication jobs have separate purposes. Prove the cross-repository environment
context in a hosted rehearsal; a generated caller and workflow-host settings
cannot prove effective consuming-repository protection.

## Inspect release immutability

After owner-approved setup, inspect the repository's effective immutable-release
setting using a separately scoped Administration-read identity. The documented
read endpoint is `GET /repos/OWNER/REPO/immutable-releases`:

```sh
gh api --method GET \
  -H 'Accept: application/vnd.github+json' \
  -H 'X-GitHub-Api-Version: 2026-03-10' \
  repos/OWNER/REPO/immutable-releases
```

Replace `OWNER/REPO` with the intended repository and use the configured scoped
identity locally. On the owner's workstation, prefix the command with `rtk proxy`.
Never put a token value in a command, guide or receipt.

The API documents an enabled response with `enabled: true` and
`enforced_by_owner`, and lists 404 for a disabled setting. A manual read of
Armorer on 2026-10-02 America/Los_Angeles using the sample's API version instead
returned HTTP 200 with `enabled: false` and `enforced_by_owner: false`.
Treat explicit false as disabled and retain the actual response; do not rely on
status 200 alone. Retain HTTP outcome, repository, observer role and time.
If visibility/permission is uncertain, a 404 cannot distinguish an absent or
disabled resource from a hidden one. A 403 or failed read leaves the gate unproven.
This sample reads settings only. See
[the endpoint and Administration-read requirement](https://docs.github.com/en/rest/repos/repos#check-if-immutable-releases-are-enabled-for-a-repository).

Manual enablement is documented in
[Preventing release changes](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/establish-provenance-and-integrity/prevent-release-changes).
Enabling the setting applies to future releases; it does not retroactively
authenticate an older release. Recheck settings near publication through the
future qualified preflight rather than relying on a bootstrap observation.

Immutable publication locks assets and the associated tag while the release
exists. Title/notes and latest/prerelease labels remain editable. GitHub recommends
attaching all assets to a draft before publication and generates a release
attestation when an immutable release is created. This release membership proof
does not replace build/SBOM provenance. See
[GitHub's immutable-release boundary](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases).

Inspect the served bytes before publication. Preserve them after it.
**This is the way.**

## Retain observations honestly

Record the capability ID, independently selected policy (`disabled`, `reporting`
or `required`), availability (`unknown`, `unsupported`, `error` or `available`),
actual enforcement and retained evidence. A required capability needs fresh
positive evidence for that same capability, with enforced state. A JSON producer
record or a settings screenshot alone does not authenticate the whole gate.
The [capability contract](trust-contracts-v1.md#capabilities-prerequisites-and-lifecycle)
defines consistency checks; the future preflight must authenticate observations.
The candidate observer reports prerequisite `configured`/`disabled`/`denied`/
`unknown`/`error` states. Those adapter observations do not supply the frozen
capability contract's release enforcement or current-job authorization.

The same manual snapshot returned 404 for both Armorer release environments.
Their visibility/existence remains unproven under that read identity. These
read-only observations changed no setting and establish no protected current-job
approval or production credential binding.

Keep credential **names**, scopes, ownership and approval boundaries in the
eventual adapter setup guide. The owner provisions values locally. Presence cannot
prove validity; protected rehearsal must exercise the actual boundary. Do not
inherit an unrestricted secret set or ask contributors to send values.

Finish with [release readiness](release-readiness.md). Retain gaps until their
own evidence stands. **This is the way.**
