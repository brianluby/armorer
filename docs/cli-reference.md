# Commands of the forge

Use the command that matches your intent. Read its result. Keep its evidence.

This reference describes `src/main.rs` on main. Open PR commands are listed in
[the development map](development-status.md). From this checkout, replace
`armorer` below with `./target/debug/armorer` after building it.

**This is the way.**

## Inspect and preview

```sh
armorer --repository /path/to/workspace plan
armorer --repository /path/to/workspace check
```

`--repository` is global and defaults to the current directory. Both operations
require a valid root `armorer.toml` and installed selected toolchain. They emit
pretty JSON to stdout and preserve consuming files.

`plan` previews `rust-toolchain.toml` and records validated intent, discovered
workspace metadata, preimages, findings and the plan digest. Read
[the contract](contracts-v1.md) for digest and ownership semantics. On main,
the preview includes proposed content and hashes; exact before/after diff views
are proposed separately in PR #6.

`check` runs the same inspection with mode `check`. Every valid inspection
currently exits 2 because secure release readiness remains blocked.

## Apply approved intent

```sh
armorer --repository /path/to/workspace apply \
  --plan /path/outside-workspace/reviewed-plan.json \
  --expect-plan-sha256 APPROVED_64_CHARACTER_DIGEST
armorer --repository /path/to/workspace recover \
  --expect-plan-sha256 ORIGINAL_APPROVED_DIGEST
```

Replace the digest placeholders with the lowercase SHA-256 retained during review.
`apply` accepts the saved typed plan and independently approved digest, rechecks
the repository, and manages the toolchain transaction. `recover` rolls back an
uncommitted matching journal or finalizes a committed matching transaction.
Follow [the complete procedure](apply.md) before using either operation.

Exact approval binds your word to exact intent. **This is the way.**

## Inspect contract schemas

```sh
armorer schema config
armorer schema release-inventory
armorer --help
armorer apply --help
```

Schema output is structural JSON Schema. Runtime semantic checks and upstream
authentication are separate requirements.

| Contract family | Accepted `schema` names |
| --- | --- |
| Local setup | `config`, `lock`, `plan` |
| Artifact trust | `release-inventory`, `verification-policy`, `artifact-evidence`, `evidence-requirements` |
| Capability and catalog | `capability-config`, `catalog`, `capability-observation` |
| Lifecycle and publication | `lifecycle-record`, `github-receipt`, `publish-set`, `registry-receipt` |

There are 14 schema names on main. JSON examples under
[trust-v1](../examples/trust-v1/README.md) are synthetic contract fixtures.

## Read the exit status

| Result | Exit | Output |
| --- | --- | --- |
| Valid plan, successful apply/recovery, schema or help | 0 | JSON for operations; text for help |
| Operational failure | 1 | JSON error on stdout |
| Valid `check` inspection in this version | 2 | JSON plan on stdout |
| Invalid CLI arguments | 2 | Clap diagnostics on stderr |

Serialization and stdout write failures also exit 1; a failed output write cannot
promise a complete JSON result. A typical operational error has this shape:

```json
{"error":{"code":"invalid-config","message":"invalid configuration: unsupported schema_version"}}
```

Stable error categories are `io`, `invalid-toml`, `invalid-config`, `unsafe-path`,
`cargo-discovery`, `cargo-metadata`, `json` and `transaction`. TOML diagnostics omit
input values. Armorer does not print arbitrary Cargo stderr or request secrets.

## Read the findings

Findings record unmet requirements in an otherwise valid inspection. Invalid
selections and unsafe inputs stop the operation with an error instead.

| Finding | Next duty |
| --- | --- |
| `github-capabilities-unchecked` | Retain the gap; authenticated preflight is later work |
| `release-runtime-unavailable` | Retain the gap; main has no reviewed secure release runtime |
| `license-policy-unreviewed` | Review the project's dependency license policy |
| `reviewed-lock-missing` | Obtain reviewed workflow/tool pins; main does not generate them |
| `pins-not-authenticated` | Authenticate upstream pin identities and bytes through a qualified runtime |
| `cargo-lock-missing` | Prepare Cargo.lock through your normal reviewed development process |
| `license-file-missing` | Provide the configured project license document |
| `cargo-config-excluded` | Review excluded Cargo overrides before adoption |
| `native-build-review` | Review target-specific native links and build-script requirements |
| `toolchain-conflict` | Resolve the preserved customization and review a fresh plan |
| `transaction-recovery-required` | Retain the unfinished journal and follow explicit recovery |

No `init`, `upgrade`, `verify-release`, `preview` or `plan --preview` command is
available on main at this documentation baseline. Proposed interfaces in the
architecture and branch documentation describe later work.

Name the limitation. Hold the gate. **This is the way.**
