# Commands of the forge

Use the command that matches your intent. Read its result. Keep its evidence.

This reference describes `src/main.rs` on main. Open PR commands are listed in
[the development map](development-status.md). From this checkout, replace
`armorer` below with `./target/debug/armorer` after building it.

**This is the way.**

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
| Bootstrap and upgrades | `ci-policy`, `bootstrap-plan`, `upgrade-plan`, `upgrade-rollback` |
| Graph and native contexts | `cargo-graph-v2`, `native-catalog-v2`, `runtime-distribution-v1`, `verification-context-v3` |

There are 24 schema names on main. JSON examples under
[trust-v1](../examples/trust-v1/README.md) are synthetic contract fixtures.

## Read the exit status

| Result | Exit | Output |
| --- | --- | --- |
| Valid plan/catalog, successful apply/recovery, schema or help | 0 | JSON for operations; text for help |
| Operational failure | 1 | JSON error on stdout |
| Top-level `check` or `bootstrap check`, provisioning in sync | 0 | JSON plan on stdout |
| Top-level `check`, drift or open setup finding | 2 | JSON plan on stdout; reasons on stderr |
| `bootstrap check`, drift or open setup finding | 2 | JSON plan on stdout; reasons on stderr |
| `bootstrap check --release-ready`, any release gate open | 2 | JSON plan on stdout; reasons on stderr |
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

## Provision, migrate and verify through separate contracts

Main also provides the following explicit interfaces:

| Command | Subcommands / guide |
| --- | --- |
| `bootstrap` | `plan`, `check`, `apply`, `recover`; [five-file procedure](bootstrap-v2.md) |
| `upgrade` | `plan`, `apply`, `recover`, `rollback-plan`, `rollback-apply`; [reviewed migration](upgrades.md) |
| `verify-release` | Independently approved context, exact assets and required bundles; [complete verifier](verify-release-cli.md) |
| `verify-historical-bytes` | Explicit independently approved historical policy; [weaker byte comparison](historical-verification.md) |
| `publication` | `plan`, `inspect`, `stage`, `publish`, `recover-published`; [owned-draft controller and native limits](owned-draft-controller-v1.md) |

Run each command with `--help` for its exact input flags. Bootstrap and upgrade
packets are separate contracts; neither authorizes the other's writes.
Verification failures do not select
historical comparison or a weaker context. Native publication writes remain
blocked until every required gate is Enforced and a separate publication approval
is supplied; the current-attempt approval and runner serialization gates are now
observable through the pinned controller's attempt-bound deployment (PR32).
There is no `init` command. The original offline library API remains available.
Use the reviewed checkout's `./target/debug/armorer`; building does not install
the executable on PATH.

Name the limitation. Hold the gate. **This is the way.**

Follow [troubleshooting](troubleshooting.md) for a stopped inspection or transaction,
and [compatibility](compatibility.md) before changing contract/runtime versions.
