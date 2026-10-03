# Keep the support promise within earned armor

**Draft proposal. Maintenance-window decision pending.** This is preparation for
[documentation ticket #14](documentation-plan.md), not a published support promise.
Neither repository has an accepted released support policy at this snapshot.
Current implemented compatibility remains in [the version guide](compatibility.md).

Make the promise precise. Earn each supported path. **This is the way.**

## Choose the maintenance window

The proposed default maintains the **current released minor line only**, separately
for Armorer and `armorer-workflows`. The alternative maintains each stream's
current and previous minor lines. No choice is accepted yet. A prerelease or
unmerged candidate does not retire a supported released line.

| Decision | Current minor only: proposed default | Current and previous minors: alternative |
| --- | --- | --- |
| Routine fixes | Latest stable patch in each current minor line | Latest stable patch in each of the two maintained minor lines |
| Older patches | Ask the adopter to move to the maintained patch through reviewed catalog/upgrade procedures | Same patch rule within both maintained lines |
| Backports | No routine backport commitment after a new minor becomes accepted and released | Reviewed backports to the previous minor require its own qualification |
| Qualification burden | Maintain the accepted current catalog combinations and their complete gates | Maintain separate accepted combinations, tools, hosts and complete gates for both lines |

An old release remains an immutable historical artifact. Retention does not
promise new fixes, continued upstream service availability or evergreen root
approval. An emergency correction outside the chosen window needs an explicit
maintainer decision and separately qualified release; it does not silently expand
the ordinary support window. This draft promises no response SLA or delivery date.

Armorer and the workflow repository advance independently. Support belongs to an
**accepted catalog combination**, not to equal version numbers or arbitrary
pairings of two maintained releases. Generated callers retain full immutable
workflow commits; choosing a support window does not authorize moving their pins.
The [catalog guide](bootstrap-catalog.md) explains the current boundary. Migration
between two accepted catalogs remains an outstanding operational qualification.

Choose the window. Keep the pins exact. **This is the way.**

## Declare the interfaces before promising compatibility

A released policy must name the CLI commands/error and report contracts, config/
lock/plan/state readers, library APIs and generated files it supports. Workflow
interfaces must identify fixed callers, runtime/helpers, tool distributions and
selection/host constraints. Internal candidate helpers are not automatically
public adopter interfaces. List exact reader/schema versions and accepted catalog
combinations in that release's compatibility record.

SemVer treats `0.y.z` as development with no stable public-API guarantee, and it
requires changed released contents to receive a new version.
[Semantic Versioning 2.0.0](https://semver.org/#spec-item-3),
[major-zero rule](https://semver.org/#spec-item-4).
The following **additional project convention is proposed**, not inferred from
SemVer or from the current `0.1.0` Cargo metadata:

- Within a maintained `0.y` line, patches preserve the declared valid interfaces.
  A security repair may reject inputs previously accepted through a defect; name
  the repaired boundary, consequence and adoption change in release notes.
- An intentional incompatible valid-interface change uses a new `0.y` line and
  an explicit migration. A future `1.x` policy needs its own acceptance.
- Incompatible trust meanings need an explicit versioned reader/schema/context
  transition and renewed independent expectations. A version bump, optional field
  or changed timestamp cannot approve weaker source, subject or root requirements.

Use [candidate upgrade/reversal guidance](candidate-upgrades.md) only at its
stated source boundary. Never change a stored schema/runtime number to bypass
rejection. Historical byte comparison does not restore cryptographic authenticity
or enroll an unsupported version in maintenance.

## Announce deprecation without weakening the gate

For a planned removal, identify the affected interface/contract version, the last
maintained release, its replacement and the reviewed migration/recovery path.
Publish that notice in the release notes and compatibility guide before the
accepted removal release. Do not invent a calendar grace period or minimum number
of intermediate releases; those remain maintainer decisions.

Separate ending maintenance from deleting evidence. Keep old release identities,
receipts, documented limitations and explicit historical policies. A dangerous
boundary may need immediate restriction; record the reason and a qualified safe
path rather than retaining an insecure bypass solely to avoid change. Published
bytes and stable tags stay unchanged. Follow [incident/recovery preparation](release-recovery.md).

Name the change. Preserve the old proof. **This is the way.**

## Earn the platform claim

A release support matrix must distinguish intent discovery, native CI/build,
artifact execution compatibility, complete independent verification and protected
release production. Evidence for one column cannot fill another.

| Claim | Required release evidence still to bind |
| --- | --- |
| Linux x64/ARM64 executable compatibility | Exact artifact, native architecture, minimum glibc/loader/system-library baseline and execution on that declared baseline |
| macOS ARM64 executable compatibility | Exact artifact and minimum OS/deployment target, native execution plus the required Developer ID/timestamp/notarization consumer checks |
| Library package support | Exact `.crate`, selected feature/target graphs, package validation and declared MSRV; native/system dependencies and exclusions visible |
| Complete release verification | Exact independent context/catalog/policy/roots, all final assets/bundles and native Apple checks where required |
| Protected release production | Effective approval, actual signed/attested own producer/consumer rehearsal, immutable publication/recovery and pilot acceptance |

The initial target names do not establish the minimum glibc or macOS deployment
baseline. A hosted runner label or successful build is insufficient to declare an
older system supported. The current guides therefore promise no minimum Linux ABI
or macOS OS version. Windows/musl, container distribution and native crates.io publication keep their
existing out-of-scope status until separately accepted and qualified.

Before promotion, the maintainer must choose the maintenance window, accept the
interface/deprecation convention and fill the actual release/catalog/platform
record. A working private reporting channel and accepted security policy remain
separate requirements. Link this accepted policy from the release and version
guide only after those promises and their evidence exist.

Promise only what the clan can maintain. **This is the way.**
