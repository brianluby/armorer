# Commands of the forge

Use the command that matches your intent. Read its result. Keep its evidence.

This reference describes `src/main.rs` on main. Open PR commands are listed in
[the development map](development-status.md). From this checkout, replace
`armorer` below with `./target/debug/armorer` after building it.

**This is the way.**

## Initialize a workspace

```sh
armorer --repository /path/to/workspace init --repo-name owner/name [--toolchain x.y.z]
armorer --repository /path/to/workspace init --repo-name owner/name [--toolchain x.y.z] \
  --write --expect-proposal-sha256 REVIEWED_PROPOSAL_SHA256
```

| Flag | Meaning |
| --- | --- |
| `--repo-name` | Required GitHub `owner/name` recorded in the proposed `armorer.toml` |
| `--toolchain` | Exact stable `x.y.z`; when omitted, only an exact `toolchain.channel` in `rust-toolchain.toml` is used |
| `--write` | Create the proposed files; requires `--expect-proposal-sha256` |
| `--expect-proposal-sha256` | The reviewed lowercase `proposal_sha256`; requires `--write` |

Without `--write`, `init` emits an `armorer-init-proposal` document (schema
`init-proposal`) and a reminder on stderr. It writes nothing. The proposal holds
the exact `armorer.toml` and `armorer-policy.draft.toml` text with SHA-256, the
discovered workspace, findings and `proposal_sha256`: the SHA-256 of compact JSON
with that field blank, bounded at 1 MiB. Discovery is the same isolated
`cargo metadata --no-deps --offline --locked` snapshot that inspection uses.

With `--write`, `init` recomputes the proposal, requires the supplied digest to
match, takes `.armorer/apply.lock`, creates the draft and then `armorer.toml`,
and never overwrites or deletes a file. Its receipt reports `outcome: written`,
the paths written, and `configured`, `policy_reviewed`, `ci_verified`,
`release_rehearsed`, `published` and `provenance_verified` all false.

`init` refuses an existing or symlinked `armorer.toml`, an invalid `--repo-name`,
a missing or inexact toolchain, a package or binary name that cannot be an
`armorer.toml` identifier, a workspace with no proposable deliverable or more
than 128 of them, and, on write, a malformed or stale digest or an existing
destination. Errors omit input values. The draft is not reviewed policy:
`bootstrap plan` rejects it until `licenses.allow` lists at least one license.

## Inspect and preview

```sh
armorer --repository /path/to/workspace bootstrap plan \
  --policy /path/to/reviewed-policy.toml
armorer --repository /path/to/workspace check
```

`--repository` is global and defaults to the current directory. Both operations
require a valid root `armorer.toml` and installed selected toolchain. They emit
pretty JSON to stdout and preserve consuming files.

`bootstrap plan` records validated intent, discovered workspace metadata,
preimages, findings, the plan digest, and exact current/proposed text with
complete diffs for all five managed files. Read
[the contract](contracts-v1.md) for digest semantics and
[the bootstrap procedure](bootstrap-v2.md) for review and approval bounds.
The digest identifies the plan being reviewed; human approval remains a
separate step.

`check` runs the same inspection over the managed `.armorer/ci-policy.toml`.
It exits 0 when provisioning is in sync and exits 2 with sorted drift reasons
on stderr otherwise. Without a managed policy it fails closed with a JSON error.
Check stays usable after an upgrade owns the files; only bootstrap writers refuse
the upgrade owner. Leftover version-one ownership is always refused.

## Inspect the catalog

```sh
armorer catalog
```

This command authenticates embedded bootstrap catalog bytes against the identity
compiled into the reviewed CLI, then emits JSON. It needs no consuming config or
network service. The library can render locks and fixed unprivileged callers;
this command writes none of them. The release trust `catalog` schema describes a
different document. Follow [the catalog guide](bootstrap-catalog.md).

The output also carries the static `reviewed_on` (UTC date the pins were last reviewed)
and `review_window_days` (180). When the catalog is older than that window, `catalog`,
`check` and `bootstrap check` each print one stderr line,
`armorer: warning: catalog pins were last reviewed <N> days ago (window <W>); update Armorer or review the catalog`.
The warning never changes an exit status. `bootstrap check --release-ready` treats a
stale catalog as the sorted blocker `catalog-review-stale`.

Know which authority you hold. **This is the way.**

## Apply approved intent

```sh
armorer --repository /path/to/workspace bootstrap apply \
  --plan /path/outside-workspace/reviewed-plan.json \
  --expect-plan-sha256 APPROVED_64_CHARACTER_DIGEST
armorer --repository /path/to/workspace bootstrap recover \
  --expect-plan-sha256 ORIGINAL_APPROVED_DIGEST
```

Replace the digest placeholders with the lowercase SHA-256 retained during review.
`bootstrap apply` accepts the saved typed plan and independently approved digest,
rechecks the repository, and provisions the five managed files. `bootstrap recover`
rolls back an uncommitted matching journal or finalizes a committed matching
transaction. Follow [the complete procedure](bootstrap-v2.md) before using either
operation.

Exact approval binds your word to exact intent. **This is the way.**

## Choose the output format

```sh
armorer --format text bootstrap plan --policy /path/to/reviewed-policy.toml
armorer check --format text
```

`--format <json|text>` is accepted before or after the subcommand. The default is
`json`: every command prints exactly the pretty JSON documented here, byte for byte,
and `--format json` is the same as no flag. `text` is opt-in and exists so a human
can review a plan. It changes only stdout. Exit statuses and stderr are identical in
both formats, with one exception below.

| Output | What `--format text` shows |
| --- | --- |
| Error | `error [code]: message` |
| Bootstrap plan (`bootstrap plan`, `bootstrap check`, `check`) | Plan digest and catalog id; one line per change with disposition, owned or unowned, and path; the `unified_diff` of every change that is not `unchanged`; findings as `code: detail`; the `bootstrap apply --plan <file> --expect-plan-sha256 <digest>` hint |
| `init` proposal | Repository, toolchain, each proposed file with its path and full content, findings, digest and the `--write --expect-proposal-sha256` hint |
| `upgrade plan`, `upgrade rollback-plan` | Direction, source and target catalogs, one line per change with its decision and path, the proposed diff for changes that are not `unchanged`, pin changes, compatibility notes or limitations, digest and the matching apply hint |
| Receipts (`init --write`, `bootstrap apply`, `bootstrap recover`, `upgrade apply`, `upgrade recover`, `upgrade rollback-apply`) | `outcome` first, then every other field as `name: value`; arrays are comma-separated |
| `catalog` | Workflow repository and commit, then a table of tool, version and platforms |
| Anything else (schemas, release verification, historical comparison, publication) | No summary exists. Armorer prints the same pretty JSON as `json` mode and writes `armorer: no text view for this output; showing JSON` on stderr. This line is the only stderr difference between the formats. |

Text mode escapes terminal-hostile characters. It prints file contents and names that
come from the repository you are inspecting, so every character for which Rust's
`char::is_control()` is true, except newline and tab, is replaced by a visible
escape: `\x1b` (lowercase hex up to 0xFF, `\u{...}` above it), and a carriage return
becomes `\r`. The Unicode bidirectional embedding, override and isolate characters
U+202A through U+202E and U+2066 through U+2069, and the line and paragraph
separators U+2028 and U+2029, are escaped the same way, for example
`\u{202e}`. Text mode adds no color or terminal control sequences of its own and ends
with exactly one newline.

The text view is a summary, never an approval input. Approval digests are computed
over the JSON document and never over the text view. To approve a plan, save the JSON
output, review it, and pass its digest to the matching apply command.

## Inspect contract schemas

```sh
armorer schema config
armorer schema release-inventory
armorer --help
armorer bootstrap apply --help
```

Schema output is structural JSON Schema. Runtime semantic checks and upstream
authentication are separate requirements.

| Contract family | Accepted `schema` names |
| --- | --- |
| Local setup | `config`, `lock` |
| Artifact trust | `release-inventory`, `verification-policy`, `artifact-evidence`, `evidence-requirements` |
| Capability and catalog | `capability-config`, `catalog`, `capability-observation` |
| Lifecycle and publication | `lifecycle-record`, `github-receipt`, `publish-set`, `registry-receipt`, `publication-policy`, `publication-plan`, `publication-approval` |
| Initialization, bootstrap and upgrades | `init-proposal`, `ci-policy`, `bootstrap-plan`, `upgrade-plan`, `upgrade-rollback` |
| Graph and native contexts | `cargo-graph-v2`, `native-catalog-v2`, `runtime-distribution-v1`, `verification-context-v3` |
| Release inputs | `release-inputs-manifest` |

There are 26 schema names on main. JSON examples under
[trust-v1](../examples/trust-v1/README.md) are synthetic contract fixtures.

## Read the exit status

| Result | Exit | Output |
| --- | --- | --- |
| Valid plan/catalog/init proposal, successful apply/recovery/init write, schema or help | 0 | JSON for operations; text for help |
| Operational failure | 1 | JSON error on stdout |
| Top-level `check` or `bootstrap check`, provisioning in sync | 0 | JSON plan on stdout |
| Top-level `check`, drift or open setup finding | 2 | JSON plan on stdout; reasons on stderr |
| `bootstrap check`, drift or open setup finding | 2 | JSON plan on stdout; reasons on stderr |
| `bootstrap check --release-ready`, any release gate open (including `catalog-review-stale`) | 2 | JSON plan on stdout; reasons on stderr |
| Invalid CLI arguments | 2 | Clap diagnostics on stderr |

Serialization and stdout write failures also exit 1; a failed output write cannot
promise a complete JSON result. A typical operational error has this shape:

```json
{"error":{"code":"invalid-config","message":"invalid configuration: unsupported schema_version"}}
```

Stable error categories are `io`, `invalid-toml`, `invalid-config`, `unsafe-path`,
`cargo-discovery`, `cargo-metadata`, `json`, `transaction` and `unsupported-host`
(non-Unix hosts). TOML diagnostics omit
input values. Armorer does not print arbitrary Cargo stderr or request secrets.

## Read the findings

Findings record unmet requirements in an otherwise valid inspection. Invalid
selections and unsafe inputs stop the operation with an error instead.

| Finding | Next duty |
| --- | --- |
| `github-capabilities-unchecked` | Retain the gap; authenticated preflight is later work |
| `release-runtime-unavailable` | Retain the gap; main has no reviewed secure release runtime |
| `cargo-lock-missing` | Prepare Cargo.lock through your normal reviewed development process |
| `license-file-missing` | Provide the configured project license document |
| `cargo-config-excluded` | Review excluded Cargo overrides before adoption |
| `native-build-review` | Review target-specific native links and build-script requirements |
| `transaction-recovery-required` | Retain the unfinished journal and follow explicit recovery |

`init` proposals add their own findings. `license-file-missing` there means no
`LICENSE`, `LICENSE.md`, `LICENSE.txt`, `LICENSE-MIT`, `LICENSE-APACHE` or
`COPYING` regular file exists and the proposal names `LICENSE`.

| Init finding | Next duty |
| --- | --- |
| `policy-draft-incomplete` | List accepted dependency licenses and review the draft before `bootstrap plan` |
| `targets-review` | Remove proposed targets the project does not ship |
| `profile-review` | Change a long-running binary's `cli` profile to `service` where it applies |
| `license-file-missing` | Add the license file, or change `policy.license_file` |
| `required-features-skipped` | Review the counted binary targets that default features do not enable; add them by hand if intended |
| `package-without-deliverable` | Review the counted packages with no default-enabled binary or library target |
| `policy-draft-exists` | Move the existing `armorer-policy.draft.toml` aside; `init --write` never overwrites it |
| `toolchain-file-differs` | An existing `rust-toolchain.toml` does not pin the `--toolchain` value (a different or inexact channel, no channel, or a file that is not valid TOML), or a legacy `rust-toolchain` file exists; resolve it before `bootstrap plan` reports the conflict. It checks the channel only: `bootstrap plan` also reports a conflict for a file that pins the same channel with other content |

## Provision, migrate and verify through separate contracts

Main also provides the following explicit interfaces:

| Command | Subcommands / guide |
| --- | --- |
| `bootstrap` | `plan`, `check`, `apply`, `recover`; [five-file procedure](bootstrap-v2.md) |
| `upgrade` | `plan`, `apply`, `recover`, `rollback-plan`, `rollback-apply`; [reviewed migration](upgrades.md) |
| `verify-release` | Independently approved context, exact assets and required bundles; [complete verifier](verify-release-cli.md); `--manifest` replaces the seven input flags |
| `verify-historical-bytes` | Explicit independently approved historical policy; [weaker byte comparison](historical-verification.md) |
| `publication` | `plan`, `inspect`, `stage`, `publish`, `recover-published`; [owned-draft controller and native limits](owned-draft-controller-v1.md) |

`verify-release` and `publication` also accept `--manifest <file>
--expect-manifest-sha256 <64 lowercase hex>` in place of their individual path and
digest flags. The two manifest flags require each other, and combining `--manifest`
with any individual input flag is an argument error (exit 2). Armorer hashes the
exact manifest bytes and compares them with the digest before parsing it or opening
any path in it, so the digest is the approval and a mismatch fails with
`release-inputs-manifest-digest-mismatch` (code `invalid-config`). Relative paths in
the manifest resolve against its directory. A `publication` subcommand with a
manifest lacking its `publication` block fails with
`release-inputs-manifest-has-no-publication-block`. Success output gains
`inputs_manifest` (`sha256`, `size`). The manifest does not make verification weaker
or stronger; it moves the values into one reviewed file. See the
[example](../examples/release-inputs/README.md) and `armorer schema
release-inputs-manifest`.

Run each command with `--help` for its exact input flags. Bootstrap and upgrade
packets are separate contracts; neither authorizes the other's writes.
Verification failures do not select
historical comparison or a weaker context. Native publication writes remain
blocked until every required gate is Enforced and a separate publication approval
is supplied; the current-attempt approval and runner serialization gates are now
observable through the pinned controller's attempt-bound deployment (PR32).
`init` proposes and creates only `armorer.toml` and the policy draft; it provisions
nothing. The original offline library API remains available.
Use the reviewed checkout's `./target/debug/armorer`; building does not install
the executable on PATH.

Name the limitation. Hold the gate. **This is the way.**

Follow [troubleshooting](troubleshooting.md) for a stopped inspection or transaction,
and [compatibility](compatibility.md) before changing contract/runtime versions.
