# Explicit runtime and native tool identity migration

The `native-catalog-v2`, `runtime-distribution-v1` and `verification-context-v2`
schemas form an explicit migration for #5/#8/#9/#11. Catalog v1, context v1,
config/lock v1, inventory/evidence v1 and Cargo graph v2 retain their previous
bytes and meaning. No failure selects another version automatically.

`TrustedReleaseContext::open` reads `armorer-verification-context.json` and
requires catalog v1. `TrustedReleaseContext::open_native_v2` reads
`armorer-verification-context-v2.json`; its outer version is 2, its `release`
field contains the frozen v1 expectation shape, and its independent
`runtime_source` approves the runtime repository, immutable commit and ref.
Both APIs check the independently approved context digest before JSON,
then exact config, lock, Cargo.lock, catalog and policy bytes. The complete
`AuthenticatedReleaseFiles::verify` consumer accepts either private context;
its inventory-first signature, whole SBOM, graph and evidence checks stay active.
These APIs are library operations. The CLI exports schemas; it has no complete
release verification or runtime approval command.

A v2 native pin retains the downloaded archive in `distribution`. The v1 lock's
tool digest continues to identify those archive bytes. Its tagged `material`
is either `exact` (raw action/schema/database/policy/helper bytes) or
`native-member` (explicit target, safe flat member name and executable bytes).
Native members require kind `tool`; raw native distributions require member
bytes to equal distribution bytes. Compressed archives may have different
identities. Evidence compares the selected executed member, its kind, version
and authentication record; it rejects foreign targets. Each baseline adapter
must have applicable pins, and every applicable baseline pin is required.
Database observations retain their explicit freshness requirement. V1 keeps
requiring every baseline pin and distribution-byte equality.

The common runtime is one uncompressed canonical USTAR archive with exactly
three members, ordered Linux x86, Linux ARM, macOS ARM:

```
armorer--x86_64-unknown-linux-gnu
armorer--aarch64-unknown-linux-gnu
armorer--aarch64-apple-darwin
```

The manifest is outside the archive, inside the approved catalog. This avoids
a self-hash cycle. It binds the actual archive, every member, runtime version,
independently approved source, exact compiler 1.95.0, runtime Cargo.lock,
same-source build workflow and retained authentication record. Workflow helper
code remains separately bound by the immutable workflow commit.

`ApprovedRuntimeFiles::open` borrows the private v2 context. It snapshots and
rehashes the whole approved archive before decoding. All three members must
match their approved bytes and native header architecture before the selected
host member becomes executable. Members are capped at 128 MiB; total archive
is capped at three such members plus 20 KiB. Streaming uses 64 KiB buffers and
checks a 20-minute stage deadline between regular-file reads. Other host
members remain inert and are never extracted. The loader invokes no command.
`native_executable()` rechecks current context validity and retained bytes;
fixed reviewed invocations belong to a separate adapter.

The accepted USTAR subset uses flat names, mode 0500, numeric uid/gid/mtime zero,
empty owner/link/prefix/device fields, regular type `0`, POSIX magic/version,
canonical octal sizes and checksum, zero member padding, two zero end blocks
and exact 10240-byte record padding. Any alternate header, duplicate/reordered
member, link, directory, PAX/GNU extension, nonzero tail or trailing data fails.
The format subset is tested against an independently created CPython USTAR
fixture; see [the pinned primary implementation](https://github.com/python/cpython/blob/v3.14.8/Lib/tarfile.py).
Native header checks establish architecture compatibility, not that an
unapproved executable is safe. Approved digests and source review remain essential.
Stable private storage and a trusted local account remain assumptions; mode
0500 cannot prevent its owner or a privileged account changing a file.

Development CI retains a release-mode candidate on each of the three native
hosts after existing Rust, genuine signature, complete SBOM, inventory and
schema gates. The fixed artifact contains `armorer` and `runtime-build.json`;
metadata identifies checkout source, compiler, Cargo.lock, target and
run/attempt. Upload uses the immutable reviewed upload-artifact pin, archive
mode, no overwrite and 14-day retention. It requests no OIDC/signing/admin
permissions. These unprivileged observations are producer claims. Independent
provider qualification must bind exact PR head/merge ancestry and source tree,
successful workflow/run/attempt, repository, artifact set/digests/sizes,
member bytes/architectures and compiler/lock observations before assembling
and separately approving a candidate runtime spec.

The explicitly ignored native test accepts a separately approved spec digest
and independent source commit/ref; it loads actual qualified three-host bytes
and invokes only native `--version` in an empty environment. Its selection
authorities remain synthetic test fixtures. This demonstrates candidate
loading, not a signed complete release or production catalog approval.

Production catalog/root acceptance, upstream tool authentication, protected
credential isolation, qualified platform report semantics, final-byte producer,
own signed complete inventory rehearsal, Apple finalization and both pilot
rehearsals remain required. Authentication-record syntax or a CI artifact
cannot satisfy those gates. This migration enables reviewed candidate work;
it does not authorize signing, publication or claim SLSA Build L2.
