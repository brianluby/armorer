# When the forge stops

Read the failure. Preserve the evidence. Correct the input that failed.
**This is the way.**

Use this guide for current local inspection, preview and toolchain transactions.
It follows main source `96457ee418439dde097339cfcd374c08f2cc98ad`.
[The CLI reference](cli-reference.md) lists commands and stable error categories.
Complete release/draft recovery remains a separate implementation gate.

## Start with the exit status

| Observation | Meaning | Next duty |
| --- | --- | --- |
| `plan`/`preview` exits 0 with findings | Inspection succeeded; setup requirements remain | Review findings, exact changes and missing evidence |
| `check` exits 2 with plan JSON | Expected current readiness result | Track prerequisites; main always retains a release-runtime gap |
| Exit 1 with JSON `error` | Operation could not complete | Match the category below and correct its input |
| Exit 2 with stderr usage text | Invalid CLI arguments | Read that command's `--help`; saved preview needs both plan and digest |
| Incomplete stdout | Output failed | Obtain a fresh complete result before review or mutation |

A successful local apply receipt can say `toolchain_configured: true` while
`configured`, `ci_verified`, `release_rehearsed`, `published` and
`provenance_verified` stay false. Treat each state as a separate claim.

## Correct configuration and selection errors

`invalid-toml` covers malformed TOML, invalid UTF-8 and unknown-field/type
mismatches. Values are omitted from its diagnostic. Inspect the file locally and
compare it with [the profile example](profiles.md); do not paste credential
values into reports. Unsupported versions and semantic selections use
`invalid-config`.

| Selection problem | Correction |
| --- | --- |
| Missing/ambiguous package | Use the Cargo package name from the workspace-root metadata |
| Missing library/binary | Use a declared Cargo target; omit `binary` only for a library |
| Required binary features disabled | Select declared package features/defaults that enable the required local edges |
| Feature not declared | Select the package's feature name; dependency feature syntax is not a substitute |
| Unsupported target | Use one of the three [current triples](compatibility.md#targets-and-hosts), or retain the unsupported scope |
| Package MSRV too new | Review a suitable exact installed compiler and regenerate affected lock/plan bindings |
| Lock does not match config bytes | Review changed config and regenerate bindings through the qualified catalog/migration path |

A valid hash's syntax proves no upstream identity. Do not invent a lock or alter
its binding to silence a finding. Missing Cargo.lock is a finding; generating one
belongs to the project's ordinary reviewed Cargo workflow, outside discovery.

Restore supported intent. Review it again. **This is the way.**

## Correct discovery placement and path errors

`unsafe-path` rejects absolute/parent paths, symlinks, redirected parents and
special input files. Keep workspace/path dependencies inside the consuming root
and use regular files. Discovery excludes `.git`, `.worktrees`, `target`,
`node_modules`, `.armorer` and `.cargo`; move required package inputs out of
excluded trees through reviewed project changes.

The snapshot's temporary directory must resolve outside the consumer and have
no ancestor `.cargo/config` or `.cargo/config.toml`. Use a trusted external
temporary directory with no inherited Cargo configuration. Armorer checks presence
without opening that configuration. A Cargo home override or repository compiler
wrapper cannot override the cleared discovery environment.

`cargo-discovery` identifies the failed installed-toolchain/metadata stage and
omits arbitrary child stderr. Ensure the declared toolchain exists; Armorer will
not install it. Inspect manifests with trusted installed tools in a context you
control. `cargo-metadata` means an unsupported metadata response. Correct the
input/tool context or retain the limitation; no repository build can turn a
read-only discovery failure into a passing inspection.

Enumeration is bounded to 20,000 entries and 128 directory levels. Manifest/lock
inputs are at most 1 MiB each and 8 MiB combined; metadata is at most 1 MiB, with
a 15-second subprocess limit. A bound failure requires reviewed scope/input
changes. Do not remove unrelated source just to satisfy a counter.

## Rebuild a review after changed input

A saved plan binds exact inputs and intent. If the config, Cargo manifests/lock,
license, legacy toolchain, discovered target paths or ownership state change,
regenerate and review a new plan. Retain its independently approved digest.
Never replace the approval at apply time with a digest read from unreviewed JSON.

Preview rejects non-UTF-8 or oversized text and views over 1 MiB. It returns no
truncated review. Inspect large customizations through the project's normal review
process and retain the unsupported preview scope. An exact preview does not lock
the filesystem; apply rechecks under its own transaction lock.

## Preserve conflicts and unfinished transactions

For `toolchain-conflict`, retain existing bytes and inspect both candidates.
Different unowned contents, legacy `rust-toolchain` and edits to owned bases need
deliberate resolution. Current main has no force/adopt command.

For `transaction` errors or `transaction-recovery-required`, follow
[the recovery runbook](apply.md#transaction-and-recovery). Keep the journal and
original approval. Do not delete `.armorer/apply.lock` to break contention: the
kernel releases a process lock on exit, and deleting a locked inode can create
two independent locks. Preserve intervening edits when rollback conflicts.

Let the evidence settle the conflict. **This is the way.**

## Carry a useful support record

Retain the reviewed Armorer source/version, host, command, exit status and static
error category/stage. Include a minimal credential-free manifest/config reproducer
and what you expected. Inspect output before sharing it; preserve exact local
evidence separately from any redacted report. Report ordinary defects through
[the repository issue tracker](https://github.com/brianluby/armorer/issues).

Discovery and verification have different boundaries. A scoped verifier's
authentication failure never enables historical mode or weaker trust. Retain
independently approved policy/root/version identities and follow
[the offline verifier guide](offline-verifier.md) rather than editing expected
identity to match a downloaded claim.

Keep required failures visible. **This is the way.**
