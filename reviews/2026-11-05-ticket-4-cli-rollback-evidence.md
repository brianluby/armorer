# Ticket #4 CLI rollback evidence

Date: 2026-11-05 UTC. Scope: Armorer project 16 ticket **#4** (database ID
**1307**) in this isolated worktree. This is local Unix regression evidence,
not ticket completion, hosted CI evidence, or a production release claim.

`tests/integrated_cli.rs` adds
`bootstrap_cli_write_failure_rolls_back_and_retry_succeeds_without_building`.
It copies the inert `examples/bootstrap-v2/library` fixture; adds a failing
`build.rs`, precreates `.github/workflows/bespoke.yml`, and writes an unrelated
sentinel. It creates the production CLI plan before changing
`.github/workflows` to mode `0500`. The saved plan's first two writes are
asserted to be `.armorer/ci-policy.toml` and then
`.github/workflows/armorer-build.yml`; the latter's real temporary-file write
therefore fails after the earlier managed write. A `Drop` guard restores the
directory mode even when an assertion panics.

The test invokes the compiled production binary for both `bootstrap apply`
commands. The blocked apply observed exit `1` and error code `io` with the OS
permission-denied error; it is not a plan/preimage/approval failure. It then
asserts that the first managed file, state, journal, both generated workflow
files, lock, and toolchain are absent, while the preexisting bespoke workflow
and unrelated sentinel retain their exact bytes. With permissions restored, the
same saved plan and digest observed `outcome: "applied"` and
`configured: true`. The build trap's `BUILD_EXECUTED` marker and `target`
directory remain absent throughout.

Commands and observed results:

```sh
rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo test --locked bootstrap_cli_write_failure_rolls_back_and_retry_succeeds_without_building --test integrated_cli -- --exact
# observed: 1 passed; blocked production CLI apply exited 1 with io/permission-denied,
# rollback preserved unrelated bytes, and restored-permission retry applied the same plan.

rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo fmt --check
# observed: passed
rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo clippy --locked --all-targets --all-features -- -D warnings
# observed: passed
rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo test --locked
# observed: passed: 78 unit, all integration suites, and 9 doctests; 9 documented native tests ignored
rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo build --locked
# observed: passed
```

Limitations: this Unix-only proof intentionally fails rather than skips when the
account can bypass directory-mode permissions. It exercises the production CLI
and filesystem replacement/rollback path, but does not model disk-full, quota,
provider, or crash failures. No public fault hook, production source change,
schema/catalog/pin/trust change, tracker write, provider write, merge, tag, or
publish occurred.
