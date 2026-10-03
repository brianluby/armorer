# Give each identity its own proof

Snapshot: 2026-10-03 UTC. Workflow candidate
`e88e25fc46c1bb5298579c5cd49f52d43215b9e1` and Armorer candidate
`56d7380a5534583b7973b5eedc55a1223b46b60e` retain their reviewed heads.
The documentation continuation checks implemented credential interfaces and
private proof export lifetimes. It is not a new exhaustive module security audit.

Keep the names exact. Let no receipt become a permit. **This is the way.**

Compared source and documentation for the fixed OIDC, mapped-job and artifact-
writer helpers, their worker child environment and current development/combined
qualification bindings. `ARMORER_READ_TOKEN` and `ARMORER_WORKFLOW_READ_TOKEN`
are distinct interfaces. Artifact service and REST credentials are separate;
writer observation deletes its two environment bindings after intent/origin
validation. Runtime JWT decoding is routing only. OIDC service variables are
platform-provided and are not operator-created repository secret names.

The original issuer proof exports only while unexpired and within 300 seconds
of observation. Mapped export additionally requires a native view at most 30
seconds old. Writer audit export requires its original handle within 30,000
milliseconds. Clock rollback and copied/reconstructed objects fail. Matched
environment claims/configuration do not establish effective protection or
approval. All signing/publication authority remains false.

An immutable source export was tested with a subprocess environment containing
only PATH, private TMPDIR and locale. No ambient token, OIDC or Apple variable
was inherited. Node is **v22.22.3** locally; this is compatibility/protocol
evidence, separate from the hosted Actions Node 24 qualifications.

| Exact source suite | Result | Evidence boundary |
| --- | --- | --- |
| `tests/producer_oidc_v1.mjs` | 11 Node tests pass | Real native RSA over ephemeral synthetic issuer keys; exact claims, proof expiry/forgery and actual request deadline control |
| `tests/mapped_producer_v1.mjs` | 12 Node tests pass | Synthetic native/issuer join, exact IDs, prerequisite changes, sterile owned children and cancellation |
| `tests/artifact_writer_v1.mjs` | One Node test script passes 35 owned protocol groups | Fixed mocked service/REST join, distinct credentials and job/check IDs, private-handle lifetime, substitutions and bounded faults |

The last row is one Node runner test containing 35 groups, not 35 independently
authenticated native provider positives. Synthetic clocks/service responses and
test keys are labelled; no current bearer value or real issuer request was used.
The [workflow integration record](2026-10-03-workflow-integration.md) retains its
distinct 85-test local and hosted native fixture receipts.

Updated [credential boundaries](../docs/credential-boundaries.md) and
[producer evidence](../docs/candidate-producer-evidence.md) name each current
interface and explain renewal without audit-time rewriting. Accepted production
bindings, effective current-attempt approval, an authorized live protected
producer positive, Apple transformation, complete own signed consumer acceptance,
publication/recovery and pilots remain required. No credential, setting, source
workflow, tag, pilot or hosted review comment was changed.

Earn the joined identity. Hold the protected gate. **This is the way.**
