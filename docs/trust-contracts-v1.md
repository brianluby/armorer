# Version-one release trust contracts

Status: proposed freeze for ticket [#2](https://kanban.luby.us/tasks/1305),
2026-10-01. Typed contracts and semantic validation are usable library interfaces;
release verification, signing and publication implementations remain #8/#9/#10.
No schema, fixture, producer record or passing local test establishes SLSA L2.
v0.1 targets Build L2; L3 remains deferred.

## Gap map and compatibility

| Existing contract | Preserved behavior | Remaining gap / decision |
| --- | --- | --- |
| config-v1 | Strict intent, package/binary/target/features, required attestations | Capability decisions live in a separate config-digest-bound sidecar; existing consumers cannot silently enable it |
| lock-v1 | Exact config bytes, runtime version, workflow SHA and tool distribution digest syntax | Independently authenticated catalog and authentication records supplement pins; syntax alone is not authentication |
| plan-v1 | Deterministic local preview/preimages and existing apply digest | No readiness promotion; new lifecycle records remain separate |
| workflow build-inventory-v1 | Unsigned artifact/SBOM/Cargo graph, source/run/attempt and tracked input identities | Trusted controller rehashes outputs; release inventory is a different document with a final-byte chain |
| Release contracts | Previously deferred | New inventory, policy, evidence, capability/catalog and receipt schemas, owned by this PR |

The frozen existing config/lock/plan files are byte-for-byte unchanged. The
workflow repository at `772ca83e386c883c88cc3b936d69f8cb3216c91e` is the inspected
compatibility baseline. Its build inventory remains authoritative in that
repository and is neither renamed nor reinterpreted as authenticated provenance.
Its selection `artifact_id = deliverable--target--feature_set` is retained as
our release-layout key. Its three files remain builder handoff inputs; the
release controller creates fixed-format packages and additional evidence later.

Each new document uses `schema_version = 1`, unknown fields are errors, and
semantic validators reject unsupported versions. Any incompatible field,
required asset, enum or meaning change gets a new schema/document version and
explicit reader/catalog migration. An optional field is not permission for
an old reader to ignore trust requirements. Root/catalog upgrades require an
independent reviewed distribution channel; no downloaded inventory chooses its
own policy, tools, trusted roots or compatibility mode.

Capability intent is `CapabilityConfig` (JSON sidecar, proposed path
`.armorer/capability-policy.json`), bound to exact `armorer.toml` bytes, the
reviewed catalog byte identity and an expiring review record. Its validator takes
the expected catalog identity independently and compares both digest and size. This preserves
old Rust and Python readers. Existing check/plan do not load these sidecars or
claim support for their execution. Catalog installation, rendering and upgrades
remain #5/#4 integration work; no candidate pins are supplied for production.

## Schemas and library API

All new types are under `armorer::trust`; `armorer schema <kind>` emits the
committed schema. `load_json<T>(root, relative)` rejects symlinks, unsafe paths,
files over 1 MiB, duplicate JSON keys at every depth, and malformed typed input.
It returns the exact bytes read once. Raw Serde deserialization is structural;
call the appropriate semantic validator separately. Errors omit supplied values.

| Schema / kind | Main validator |
| --- | --- |
| release-inventory | `ReleaseInventory::validate_against`, `compare_asset_bytes`, `compare_published_bytes` |
| verification-policy | `VerificationPolicy::validate`, `accept_source`, `accept_signer`, `compare_historical_bytes` |
| artifact-evidence | `ArtifactEvidence::validate_against_requirements` |
| evidence-requirements | Validated as independent input by the artifact-evidence validator |
| capability-config | `CapabilityConfig::validate` |
| catalog | `Catalog::validate` against existing lock compatibility |
| capability-observation | `CapabilityObservation::satisfies` |
| lifecycle-record | `LifecycleRecord::validate` |
| github-receipt | `GithubReceipt::validate`, `same_owned_retry` |
| publish-set | `PublishSet::validate`, `identity` |
| registry-receipt | `RegistryReceipt::validate_against`, `same_retry_set` |

Schemas constrain structure and selected syntax/ranges. Semantic validation adds
cross-document bindings, version support, identities, uniqueness, required asset
sets, relationships, time/freshness, coverage, DAG and retry rules. Neither layer
performs signature verification, platform API checks, SBOM graph validation,
archive extraction, Apple inspection, approval authentication or publication.
The public API accepts independently established expected context explicitly.
Calling it with expectations copied from the record defeats that boundary.

Byte identities are lowercase SHA-256 and nonzero sizes, capped at 1 GiB per
asset in this v1 layout. Empty/partial uploads cannot satisfy a required asset.
Portable asset names are capped at 255 bytes. Existing v1 configuration can
accept long IDs whose derived release filenames exceed this cap; release-layout
validation then reports `invalid-derived-asset-set` and blocks readiness. Shorter
reviewed IDs or a future versioned layout are required; no silent truncation occurs.
Records use UTC Unix seconds; caller-provided verifier time must be trusted.
All deserialized contracts are bounded to 1 MiB; larger releases require a
reviewed future layout, not a larger unchecked parser limit.

## Inventory, subjects and hash order

`expected_assets` derives names and roles from separately reviewed config and
policy. Each target/feature deliverable uses `id--target--feature_set`:

| Suffix | Role / identity |
| --- | --- |
| `.crate` for library, `.tar.gz` for CLI/service | Final distributed bytes; source package and executable archive remain distinct |
| `.cdx.json` | Primary target-associated Cargo SBOM |
| `.cargo-graph.json` | Selected Cargo graph evidence |
| `.build.json`, `.package.json` | Per-artifact evidence records |
| `.apple.json` for macOS executable deliverables | Unsigned/sign/notarize/package transformation record |
| `.native.cdx.json`, `.embedded.json`, `.diagnostic.json`, `.rebuild.json` | Policy-selected complementary native, embedded, diagnostic and rebuild roles |
| `<asset>.provenance.sigstore.json` | SLSA v1 provenance bundle for every supplied non-bundle asset |
| `<distributable>.sbom.sigstore.json` | SBOM bundle with final bytes as subject and published Cargo SBOM as predicate asset |

This layout is new release behavior for #8/#10, not a change to existing unsigned
builder filenames (`.bin`, `.cdx.json`, `.cargo-graph.json`). No caller-selected
filename, generic attestation job, custom predicate or checksum-only substitute
is introduced. A future checksum role requires its own layout decision; this
v1 inventory already supplies exact SHA-256 and size for every listed asset.

Relationships point bundle → evidence/SBOM → distributable. Validators compare
exact independently derived relationships, rejecting cycles, dangling references,
subject swaps, duplicates, extra files and omitted baseline proof. Optional
reporting evidence cannot replace any baseline asset. If optional evidence is
supplied, its provenance bundle becomes required; every supplied bundle needs
cryptographic verification in #8, even if the control was reporting-only.

Generation order is final artifacts/SBOM/evidence → their bundles → inventory →
inventory provenance bundle. `armorer-release-inventory.json` lists none of its
own bytes or its own authentication bundle. The separately required
`armorer-release-inventory.provenance.sigstore.json` authenticates inventory bytes
and is outside that list. `compare_published_bytes` requires precisely listed
assets plus these two detached files and independently verified identities.
There is no inventory/bundle hash cycle and no bundle-of-bundle requirement.

Transformation stages retain unsigned executable, signed executable, notarized
object and final package byte identities in the evidence record. They need not
all be public release assets; their run-bound retained evidence must be available
to #9's verifier. The build inventory remains unsigned and cannot substitute
for provenance over the final `.tar.gz` bytes.

## Independent verification policy

The JSON policy can be stored as independently distributed `armorer-policy.json`;
architecture's originally proposed TOML policy is superseded only for this new
not-yet-implemented document. It has explicit `release` and `rehearsal` modes. Release mode binds stable
tag refs; rehearsal mode binds separately reviewed protected branch refs. Both
bind exact repository and source commits; signer repository/workflow/full revision; fixed predicates and
evidence scopes; hosted-runner requirement; public/private trust backend;
reviewed trusted-root bytes and exact verifier version/distribution bytes.
Root expiry/review and offline root provisioning are explicit prerequisites.
There are no URLs, regex trust identities or custom predicate fields.

A genuine signature from a wrong repository, source commit/ref, signer revision,
workflow, predicate or evidence scope must fail. #8 delegates crypto to a pinned
`gh`, compares authenticated certificate/source claims to this policy, compares
actual artifact bytes, and compares verified SBOM predicate JSON to published
SBOM bytes plus the selected Cargo graph. Producer predicate fields remain
producer assertions. Authenticated certificate/timestamp evidence must be
retained separately and never replaced by a `verified = true` producer field.

Historical compatibility is an explicit selection of exact repository/tag/commit
and an exact name/size/digest allowlist, with owner/rationale/review/expiry and
limitations. Historical and baseline refs cannot overlap. It returns only weaker
historical byte-match evidence, never provenance-verified or an L2 claim. No
missing/invalid required evidence falls back to historical mode. #8 must keep
mode selection explicit and reject authenticity failures; the contract helper
compares allowlisted bytes and does not implement that runtime mode router.

## Per-artifact evidence and Apple limits

`ArtifactEvidence` captures source/config/Armorer lock/Cargo lock/runtime bytes,
run/attempt/workflow, exact package version/target/binary/features, catalog bytes,
runner label/image, tools/actions/helpers/databases/query/schema/policy/native
inputs, authentication records, timestamps/freshness, coverage/omissions,
outcomes, enforcement and narrow reviewed exceptions. The actual runner image
is mutable. Package versions and source snapshots need source-resolved builder
checks; selection/config consistency cannot prove them.

`EvidenceRequirements` comes from independently reviewed config/catalog/policy.
It binds exact inputs/selection, stage workflows, tool versions/bytes/authentication
records and observation times, database freshness ceilings, required tested
subject/scope/omissions and exact exception IDs, maximum evidence age, permitted
exceptions and expected Apple team. Producer records cannot refresh an approved
observation time, widen freshness or reduce enforced scope. Allowing an exception
record does not authorize attaching it to exception-free required coverage; its
use must match the independently approved coverage record. Exceptions bind tool,
rule/version, tested subject, policy, owner/rationale/review/expiry and adjacent
positive-control evidence. Source/signer/byte/trigger/publication/credential
requirements cannot be waived. Authentication-record hashes do not themselves
verify upstream identity: #5/#8 must authenticate their referenced content.

A complete macOS executable chain is build → sign → notarize → package. Every
step records exact input/output identities, run/attempt, workflow and times;
signing must change bytes, notarization can preserve a standalone executable's
identity, and packaging output must equal the inventory's final distributed
bytes. Missing, failed, cross-attempt and unsigned-final substitutions fail.
Library `.crate` files are source packages, including on a macOS-associated
build target; they do not acquire Developer ID signing requirements.

Developer ID team/certificate digest, hardened runtime, secure timestamp,
notarization submission/status/log digest and online-versus-stapled ticket mode
are producer assertions until #9 authenticates and inspects them. Standalone
Mach-O evidence uses online notarization checks; do not promise stapled/offline
acceptance for it. Platform-evidence references must be present for a complete
chain, but their authenticity is a separate #8/#9 gate. Final-byte provenance
must be generated after all mutations; retaining references or hashing bytes
does not establish platform isolation, reproducibility or Build L3.

## Capabilities, prerequisites and lifecycle

Capability IDs enumerate baseline platform attestations, immutable releases,
protected tags/publish/signing environments, Apple signing, Cargo SBOM and
project dependency policy; optional CodeQL/vet/embedded/native/egress/rebuild/
Scorecard/dist/registry capabilities remain design inputs. `disabled`,
`reporting`, `required` are reviewed policy decisions. Availability is separately
`unknown`, `unsupported`, `error`, `available`; enforcement is `disabled`,
`not-tested`, `skipped`, `reporting`, `enforced`. A required capability needs
fresh positive evidence and enforced state for the independently specified
`CapabilityId`; one capability observation cannot satisfy another. An HTTP 403 is unknown/error and
cannot satisfy publication prerequisites. Catalog adapters are enumerated and
compatibility-bound; a record is not qualification or an enabled implementation.

Configure branch/tag rulesets, workflow CODEOWNERS/review, effective required
checks and caller-scoped protected `release-publish` / `release-signing`
environments manually. #10 checks current immutable-release settings using
isolated narrowly scoped Administration read access, and qualifies account/
visibility/plan support. Attestation jobs need `contents: read`,
`attestations: write`, `id-token: write`; build jobs retain read-only access.
Apple credentials are named by the eventual reviewed adapter and provisioned
locally by the owner. Never request secret values or use `secrets: inherit`.
Effective cross-repository environment context, protected ref/ancestry and
credential boundaries require hosted rehearsals, not schema inference.

`configured`, `ci-verified`, `release-rehearsed`, `published`,
`provenance-verified` have separate source/config/policy-bound lifecycle receipts.
Branch/PR source identity is allowed for configuration and CI records; published
and provenance-verified stages require stable tag identity. Rehearsal policy
cannot authorize a GitHub release or registry publish set. One stage does not imply another. Lifecycle receipt validation checks recorded
consistency; #8/#10 authenticate gate evidence. Check/plan remain read-only and
never execute a capability adapter or build script to discover support.

## GitHub and future registry receipts

GitHub receipt states are owned draft, uploaded, draft bytes verified, approved,
published, release attestation verified, provenance verified, conflict. They
bind source/config/locks/runtime/run/attempt, stable tag, release ID, exact
inventory and policy, draft-download receipt, independent approval record,
current immutable-setting observation and separate post-publication evidence.
Snapshots can be consistent without proving their claims; #10 must authenticate
observations and enforce transitions and publication approval. Draft retries
require identical ownership, byte and policy identity, including attempt;
conflicting attempts cannot overwrite. Published states cannot resume as drafts.

Future crates.io uses a distinct `PublishSet`: exact approved per-crate name,
version, `.crate` bytes and a topologically ordered internal dependency DAG.
Its identity hashes compact typed JSON, with struct field order and declared
crate/dependency order retained, excluding every mutable receipt/status field.
It is an identity rule, not a general JSON canonicalization standard. The future
adapter must separately bind external dependencies, publisher/environment/owner
identity, `publish = false`, staging/repackaging and exact transmitted objects.
Registry selection is an enum, not a caller URL or credentials field.

Per-crate states are prepared, uploaded, upload-result-unknown, registry-observed,
registry-bytes-verified, conflict. Archive/index byte evidence is allowed only in byte-verified or conflict receipts;
other states cannot carry contradictory byte-verification fields. Visibility alone is insufficient: verified
state needs the approved archive identity, index checksum and retained independent
observation. Dependents cannot upload before prerequisites are observed and
byte-verified. Index/CDN lag remains pending. Retries bind the same publish-set;
changed bytes/name/version/DAG require a new review. Conflict recovery is explicit
investigation/reviewed new-version/incident handling, never clobber, automatic
yank or rollback. GitHub and registry publication order remains a future adapter
review decision: no cross-service atomicity or multi-crate rollback is claimed.

## Examples, tests and handoffs

[Examples](../examples/trust-v1/README.md) cover library, Linux CLI, ARM service,
combined workspace, final macOS and historical unattested input. All pins,
approvals, bundles, notarization values and platform references are **synthetic**.
They exercise contracts, not real SBOM parsing, cryptography or hosted settings.

| Owner | Concrete handoff / required integration evidence |
| --- | --- |
| #8, verification | Authenticate independent policy/root/catalog bytes; use bounded loader and fixed asset layout; verify each required bundle and detached inventory bundle with real pinned gh; enforce source/signer/ref/predicate/scope/hosted claims; rehash downloads and compare verified SBOM predicate/graph; explicit historical router; real signed positive/adversarial and offline-root fixtures |
| #9, Apple | Convert run-bound unsigned builder inventory to stage identities; bounded archive/producer handoff validation before credentials; derive exact stage workflow expectations; authenticate Developer ID/Apple acceptance and platform references; preserve unsigned/signed/final objects; attest final bytes and rehearse online/offline package limits |
| #10, release | Independently derive assets and expected context; live capability/trigger/ref/ancestry/actor checks; ownership/precondition/draft download checks; human approval and current immutable setting; safe retries and immediate prepublish recheck; verify served bytes/release attestation after publication; no actual release is authorized by this PR |

Contract rejection tests cover tampering, source/signer/predicate mismatches,
missing/extra/duplicate/cyclic assets, unsafe triggers, scope/pin/freshness drift,
failed Apple steps and unsigned-final swaps, unknown/unsupported/reporting
capabilities, historical downgrade attempts, publication evidence gaps and
conflicting retry/DAG/index identities. Existing discovery/apply tests remain.
Runtime signature validation, Apple checks, hosted eligibility, actual SBOM graph
validation and remote state-machine races remain explicitly deferred above.

## Primary-source verification and limits

Rechecked on 2026-10-01: [gh attestation verify](https://cli.github.com/manual/gh_attestation_verify)
exposes source/signer digest/ref, predicate, hosted-runner and offline bundle/root
controls; it distinguishes certificate/timestamp identity from producer predicates.
[SLSA v1.2 Build requirements](https://slsa.dev/spec/v1.2/build-requirements)
separate L2 authentic hosted provenance from L3 isolation/unforgeability.
[GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
describes published asset/tag immutability and release attestations. These are
upstream capabilities, not proof our future controller or a pilot qualifies.
Optional adapter/registry behavior remains the dated #16 assessment; this PR
adds no new support claims and does not refresh or enable those adapters.
