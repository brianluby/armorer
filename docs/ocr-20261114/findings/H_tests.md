# Review findings — group H_tests (70 findings)

─── tests/apply.rs:1-1 ───
[test · medium] This test file is not gated to Unix, but `armorer::apply::apply` unconditionally
returns `Err("durable transactions require a supported Unix host")` on non-Unix hosts
(src/apply.rs). Every test here that calls `apply(...).unwrap()` (or asserts a specific apply error)
will therefore fail on Windows/macOS-less CI targets — only the symlink test is `#[cfg(unix)]`. Gate
the whole file with `#![cfg(unix)]` (or gate each apply-dependent test) so the suite matches the
platform contract of the code under test.

+ #![cfg(unix)]
+
  use armorer::{

─── tests/apply.rs:85-91 ───
[test · medium] The mtime-equality check can pass spuriously: several filesystems have 1–2s mtime
granularity, so a non-idempotent rewrite of `rust-toolchain.toml` within the same timestamp tick
still compares equal. Unlike `state.json`, the toolchain file's bytes are never re-checked after
replay, so even a content-changing rewrite could slip through. Add a deterministic byte-level
assertion after the second apply (keep the mtime check as a secondary signal; note that an
identical-content rewrite in the same tick remains unprovable via mtime alone).

+     assert_eq!(
+         bytes(root.path(), "rust-toolchain.toml"),
+         plan.changes[0].proposed_content.as_bytes()
+     );
      assert_eq!(
          fs::metadata(root.path().join("rust-toolchain.toml"))
              .unwrap()
              .modified()
              .unwrap(),
          modified
      );

─── tests/apply.rs:202-203 ───
[test · low] Two robustness gaps: (1) Contention is simulated with two fds in the *same* process.
That works for Linux `flock`, but on filesystems that emulate it with per-process POSIX locks (e.g.
some NFS mounts) the second lock succeeds and this test fails/misbehaves; holding the lock from a
spawned child process would exercise real cross-process exclusion. (2) Matching the prose substring
"another Armorer" couples the test to error wording — a reworded message breaks the test even though
behavior is correct. Prefer a typed busy/lock-contention error variant (or at least a stable error
code) over string matching.

─── tests/apply.rs:236-237 ───
[test · low] Symlink-escape coverage is limited to final-component symlinks (`.armorer` and
`rust-toolchain.toml`). A regression that follows symlinks in *intermediate* path components (e.g. a
symlinked parent of a managed file) or that opens a symlinked approved-plan file would not be caught
here. Add cases with a symlinked intermediate directory and a symlinked plan file passed to
`load_plan`/the CLI, asserting the outside target is neither read-adopted nor written.

─── tests/apply.rs:279-279 ───
[test · medium] Both forged-state scenarios assert only `is_err()`, so an unrelated early failure
(digest validation, input drift, etc.) would mask a regression in the intent/ownership
re-verification this test is meant to protect — e.g. if the `same_inputs` re-check were removed but
the fresh-digest check still rejected the forged plan, this test would keep passing. Pin the
rejection reason (assert the error message/variant: "approved inputs changed" for the forged plan,
"committed plan receipt conflicts with managed files" for the emptied `managed` map), and after each
rejected apply assert `state.json` bytes are unchanged so a failed apply can't silently mutate
ownership state.

─── tests/apply.rs:295-298 ───
[test · low] The injection only produces a genuine duplicate key because the fixture's `BTreeMap`
happens to serialize `"LICENSE"` first with a non-null value; nothing asserts the replacement
occurred or that a doubled key exists. If the key set or serialization changes (empty map →
trailing-comma invalid JSON; absent `LICENSE` → merely an extra field), `load_plan` would reject for
parse/unknown-field reasons and this test would silently stop exercising duplicate-key detection.
Pin the injected duplicate, e.g. `assert!(raw.contains("\"LICENSE\":null,\"LICENSE\":"));` before
writing the file.

      let raw = serde_json::to_string(&plan).unwrap().replace(
          "\"input_preimages\":{",
          "\"input_preimages\":{\"LICENSE\":null,",
      );
+     assert!(raw.contains("\"LICENSE\":null,\"LICENSE\":"));

─── tests/apply.rs:241-241 ───
[test · medium] Despite the name, this test never verifies that approval is *required*: it only runs
the happy path with `--expect-plan-sha256` supplied. If the CLI regression made the flag optional
(silently auto-approving the plan file), this suite would still pass — the digest gate is only
tested at the `load_plan` library level, not at the CLI argument-wiring level. Add a negative
invocation (omit `--expect-plan-sha256`, and/or pass a wrong digest) asserting a non-zero exit and
no applied files. Also include stderr in the failure diagnostic below — CLI errors are usually
written there, so a failing run currently prints an empty/unhelpful message.

─── tests/bootstrap.rs:50-62 ───
[test · medium] The no-write oracle follows symlinks and opens special files: `path.is_dir()`
follows directory symlinks (walking outside the fixture and misattributing escaped mutations), and
`fs::read` on a FIFO blocks forever. This is why the symlink/FIFO tests can't use `snapshot` at all
and fall back to weaker checks (outside-dir count / `.armorer` absence), missing partial writes
inside the fixture root. Use `entry.file_type()` (does not follow links) and record symlinks/special
entries without opening them, so every test can use the full byte-equality oracle.

-             let path = entry.unwrap().path();
-             if path.is_dir() {
+             let entry = entry.unwrap();
+             let path = entry.path();
+             let kind = entry.file_type().unwrap();
+             let key = path.strip_prefix(root).unwrap().to_str().unwrap().to_owned();
+             if kind.is_dir() {
                  walk(root, &path, files);
+             } else if kind.is_symlink() {
+                 files.insert(key, fs::read_link(&path).unwrap().to_string_lossy().into_owned().into_bytes());
+             } else if kind.is_file() {
+                 files.insert(key, fs::read(&path).unwrap());
              } else {
-                 files.insert(
-                     path.strip_prefix(root)
-                         .unwrap()
-                         .to_str()
-                         .unwrap()
-                         .to_owned(),
-                     fs::read(path).unwrap(),
-                 );
+                 files.insert(key, b"<special>".to_vec());
              }

─── tests/bootstrap.rs:377-378 ───
[test · medium] This test unconditionally spawns the external GNU `patch` binary; on Windows or
minimal CI images without it, `spawn().unwrap()` panics with NotFound instead of the test being
skipped. The other external-tool tests (`symlink`, `mkfifo`) are `#[cfg(unix)]`-gated — gate this
one the same way (or skip when `patch` is unavailable / apply the diffs with an in-crate applier).

+ #[cfg(unix)]
  #[test]
  fn independently_applied_complete_diffs_reconstruct_every_target() {

─── tests/bootstrap.rs:159-159 ───
[maintainability · low] Security assertions are positionally coupled to catalog/TARGETS ordering:
`changes[index]` here, `changes[1]` in the forgery test, `changes[4]` in the replay test, plus
hardcoded counts (5 files, 18 tools). A catalog reorder silently retargets the
conflict/ownership/forgery checks at different files while the tests still pass. Select changes by
path (as `new_bootstrap_defaults_to_the_v2_successor_authority` already does with `.find(|c| c.path
== ...)`) or assert the expected path at each index so a reorder fails loudly.

─── tests/bootstrap.rs:135-135 ───
[test · low] mtime equality does not prove no rewrite: on filesystems with coarse timestamp
granularity (1–2 s ticks, e.g. FAT/some network FS), a rewrite within the same tick leaves
`modified()` unchanged, so this assertion can pass even if the second `apply` rewrote the files. The
byte snapshot above is the only real evidence here. For a stronger idempotence proof, compare file
identity (inode on unix) or mark the targets read-only so any rewrite attempt errors.

─── tests/ci_policy.rs:27-28 ───
[test · low] When a case in this loop fails, neither `unwrap_err()` nor the bare `assert!`
identifies which mutated policy caused the failure, making CI failures hard to diagnose. Attach the
input to the failure messages (e.g. `expect_err(&format!(...))` and an assertion message). The same
applies to the `bad`-input loops in the other tests, which currently use a bare
`assert!(parse_at(bad.as_bytes(), today).is_err());` with no context.

-         let error = parse_at(invalid.as_bytes(), today).unwrap_err();
-         assert!(!error.to_string().contains("DO-NOT-ECHO"));
+         let error = parse_at(invalid.as_bytes(), today)
+             .expect_err(&format!("policy should be rejected: {invalid}"));
+         assert!(
+             !error.to_string().contains("DO-NOT-ECHO"),
+             "error echoed untrusted input: {error}"
+         );

─── tests/ci_policy.rs:44-44 ───
[test · low] This case is intended to verify rejection of non-zero-padded dates, but `2026-2-03`
(Feb 3, 2026) is also before `today` (2026-10-02), so it would be rejected as expired even if format
validation were entirely missing — the assertion does not isolate the intended check. Use an
unpadded date that is otherwise valid and inside the accepted expiry window (the test accepts
"2026-12-31"), e.g. "2026-12-3", so only the missing zero-padding can cause rejection.

-         "2026-2-03",
+         "2026-12-3",

─── tests/ci_policy.rs:102-104 ───
[test · low] This loop injects `custom_hook = "DO-NOT-ECHO"` into `[bans]` but, unlike the first
test, never asserts that the resulting error message avoids echoing the untrusted input — the
non-echo guarantee is only checked for the top-level unknown-key path. Add the same
`!error.to_string().contains("DO-NOT-ECHO")` assertion here, and include the failing input in the
message so a failure identifies which of the five mutations broke.

          good.replace("[bans]", "[bans]\ncustom_hook = \"DO-NOT-ECHO\""),
      ] {
-         assert!(parse_at(bad.as_bytes(), today).is_err());
+         let error = parse_at(bad.as_bytes(), today)
+             .expect_err(&format!("policy should be rejected: {bad}"));
+         assert!(!error.to_string().contains("DO-NOT-ECHO"), "{error}");

─── tests/catalog.rs:105-105 ───
[test · medium] This hardening test only exercises the v1 catalog (`reviewed()` /
`WORKFLOW_COMMIT`). The v2 catalog (`reviewed_v2()`, pinned to `WORKFLOW_COMMIT_V2`) is what the CLI
test asserts against, yet its rendered callers never go through these permission/forbidden-key
checks — a regression in the v2 pin or a v2-specific renderer change would only be caught by the
byte-comparison fixture test, which can be "fixed" by updating the committed fixtures in the same
change. Run the same assertions for both catalogs, parameterized by their expected commit.

─── tests/catalog.rs:107-107 ───
[test · low] The test name promises "only reviewed full pins", but this is a presence check: an
extra unpinned reference (e.g. a second `uses: actions/checkout@main` line) would still pass, since
the pinned line remains and none of the forbidden substrings match. Also, the forbidden list misses
`permissions: write-all`, which grants write access without matching `"contents: write"`. Assert
exactly one `uses:` occurrence per workflow and add `"write-all"` to the forbidden list.

          assert!(value.contains(&format!("@{WORKFLOW_COMMIT}\n")));
+         assert_eq!(value.matches("uses:").count(), 1);

─── tests/publication_cli.rs:2-2 ───
[bug · medium] This unconditional `std::os::unix` import breaks compilation of the whole test crate
on non-Unix targets. Sibling tests gate this import (and Unix-only tests) with `#[cfg(unix)]` (see
`tests/integrated_cli.rs`). Since this test also writes `/bin/sh` scripts and sets Unix permission
bits, add `#[cfg(unix)]` to the test function as well.

- use std::{fs, os::unix::fs::PermissionsExt, process::Command};
+ use std::{fs, process::Command};
+ #[cfg(unix)]
+ use std::os::unix::fs::PermissionsExt;

─── tests/publication_cli.rs:16-16 ───
[test · low] Hardcoding `/usr/bin/touch` weakens the execution detector on Unix systems where that
path doesn't exist (NixOS, some minimal images): the marker can never appear, so
`assert!(!marker.exists())` passes vacuously even if a regression runs the tool before context
authentication (the context would still fail afterwards with the expected message). Use
PATH-resolved `touch` like the sibling tests (`tests/release_context.rs`, `tests/historical.rs`).

-             format!("#!/bin/sh\n/usr/bin/touch '{}'\n", marker.display()),
+             format!("#!/bin/sh\ntouch '{}'\n", marker.display()),

─── tests/publication_cli.rs:60-60 ───
[test · low] A bare `unwrap()` here panics with only a serde error when the CLI emits non-JSON
output, hiding which of the five actions failed and what the CLI actually printed. Include the
action and captured output in the failure message (the `cli()` helper in `tests/integrated_cli.rs`
does this) so regressions in this security-ordering guarantee are diagnosable.

-         let diagnostic: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
+         let diagnostic: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
+             panic!(
+                 "{action}: invalid diagnostic JSON ({error}); stdout: {}; stderr: {}",
+                 String::from_utf8_lossy(&output.stdout),
+                 String::from_utf8_lossy(&output.stderr)
+             )
+         });

─── tests/real_cyclonedx.rs:25-29 ───
[test · medium] This blanket `is_err()` accepts a failure from any gate, so the negative cases don't
prove what the test claims. In `validate`, errors can come from the byte snapshot, the `specVersion
== "1.5"` pre-check (`unsupported-sbom-document-version`), the duplicate-key JSON parser, or the
native tool (`sbom-schema-validation-failed`). Concretely: the `specVersion: "1.6"`, duplicate-key,
and `"{} {}"` inputs never reach the native tool at all, and the three metadata mutations
(licenses/timestamp/externalReferences) — the whole point of "distinguish malformed metadata from
graph consistency" — would still pass this assertion if a regression made the native step fail for
the wrong reason or be skipped. Pass an expected error code into the helper and match it, e.g.
`Err(armorer::Error::Invalid(code)) if code == expected_code`, using
`"sbom-schema-validation-failed"` for the three field mutations (as the positive test already does
for `verification-input-type-or-size`).

─── tests/integrated_cli.rs:50-56 ───
[test · low] The `cli` helper makes contract failures hard to diagnose: (1) the status assertion
prints only stderr, so when the child is killed by a signal (`status.code()` is `None`, stderr often
empty) the failure message is blank; (2) `serde_json::from_slice(...).unwrap()` panics with only a
parse error and never shows the raw stdout that failed to parse. Include `output.status` (its
`Display` reports signals) and the raw stdout in both failure paths.

      assert_eq!(
          output.status.code(),
          Some(status),
-         "{}",
-         String::from_utf8_lossy(&output.stderr)
+         "status: {}\nstderr: {}\nstdout: {}",
+         output.status,
+         String::from_utf8_lossy(&output.stderr),
+         String::from_utf8_lossy(&output.stdout)
      );
-     serde_json::from_slice(&output.stdout).unwrap()
+     serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
+         panic!(
+             "CLI stdout was not valid JSON ({error}): {}",
+             String::from_utf8_lossy(&output.stdout)
+         )
+     })

─── tests/integrated_cli.rs:260-260 ───
[test · medium] This fault injection is environment-dependent: mode `0o500` does not block writes
when the test runs as root (common in CI containers) or on filesystems that ignore Unix mode bits,
so the exit-status assertion below fails spuriously even though the CLI is correct (the assertion
message even anticipates this). Additionally, the later `"Permission denied" / "Operation not
permitted"` substring match depends on English-locale `strerror` output. Probe whether the mode
actually blocks writes and skip visibly when the environment bypasses permission checks, so the test
only runs where the intended failure path is real.

      fs::set_permissions(&workflows, fs::Permissions::from_mode(0o500)).unwrap();
+     let probe = workflows.join(".permission-probe");
+     if fs::write(&probe, b"probe").is_ok() {
+         fs::remove_file(&probe).unwrap();
+         drop(restore);
+         eprintln!("skipping: account/filesystem bypasses directory mode bits (root?)");
+         return;
+     }

─── tests/integrated_cli.rs:284-294 ───
[test · medium] Rollback completeness is verified against a hardcoded list of paths, so leftovers at
any unlisted path (temp files, renamed backups, artifacts added as the managed set evolves) would
pass these absence assertions. `before` already captures the full pre-apply byte snapshot —
comparing `snapshot(root.path())` against `before` proves complete restoration, subsumes this list,
and stays correct as managed artifacts change. (Note the snapshot still can't see empty directories;
see the `snapshot` helper comment.)

-     for path in [
-         ".armorer/ci-policy.toml",
-         ".armorer/bootstrap-state-v2.json",
-         ".armorer/bootstrap-journal-v2.json",
-         ".github/workflows/armorer-build.yml",
-         ".github/workflows/armorer-ci.yml",
-         "armorer.lock",
-         "rust-toolchain.toml",
-     ] {
-         assert!(!root.path().join(path).exists(), "{path}");
-     }
+     assert_eq!(snapshot(root.path()), before);

─── tests/integrated_cli.rs:247-251 ───
[test · low] Pinning the blocked workflow to exactly `changes[1]` couples this test to the plan's
precise ordering. The scenario only requires that the blocked write happens *after* at least one
successful managed write; a benign reordering (e.g., inserting another writable change before the
workflow) breaks the test even though rollback behavior is unchanged — and if these assertions were
ever relaxed, the failure could occur on the very first write, silently losing the partial-apply
rollback coverage. Keep the guarantee that the first change is writable, but locate the blocked
workflow's position dynamically instead of hardcoding index 1.

-     assert_eq!(plan["changes"][0]["path"], ".armorer/ci-policy.toml");
-     assert_eq!(
-         plan["changes"][1]["path"],
-         ".github/workflows/armorer-build.yml"
+     let changes = plan["changes"].as_array().unwrap();
+     assert_eq!(changes[0]["path"], ".armorer/ci-policy.toml");
+     let blocked = changes
+         .iter()
+         .position(|change| change["path"] == ".github/workflows/armorer-build.yml")
+         .expect("plan must contain the managed build workflow");
+     assert!(
+         blocked > 0,
+         "the blocked workflow write must follow a successful managed write"
      );

─── tests/integrated_cli.rs:31-37 ───
[bug · medium] `Path::is_dir()` follows symlinks. If the code under test ever leaves a symlink
behind (e.g., an interrupted atomic replacement — exactly the kind of side effect this harness
exists to detect), `snapshot` recurses outside the tempdir or loops forever on a symlink cycle,
hanging/aborting the harness instead of reporting the offending write. Dispatch on non-following
metadata (`fs::symlink_metadata` or `DirEntry::file_type`) and record symlinks as their own entries.
The same issue exists in `copy` (`entry.path().is_dir()` + `fs::copy` follow links). Also note the
files-only byte map cannot observe empty-directory creation/removal or permission changes; consider
recording directory entries too if those guarantees matter.

-         if path.is_dir() {
+         let metadata = fs::symlink_metadata(&path).unwrap();
+         if metadata.file_type().is_symlink() {
+             files.insert(
+                 relative,
+                 fs::read_link(&path)
+                     .unwrap()
+                     .to_string_lossy()
+                     .into_owned()
+                     .into_bytes(),
+             );
+         } else if metadata.is_dir() {
              for (name, bytes) in snapshot(&path) {
                  files.insert(format!("{relative}/{name}"), bytes);
              }
          } else {
              files.insert(relative, fs::read(path).unwrap());
          }

─── tests/integrated_cli.rs:217-220 ───
[test · low] `.ok()` collapses every read error into `None`, conflating "file absent" with "file
exists but is unreadable" (permissions, I/O errors). If `Cargo.lock` was absent in `before` but is
later created unreadable, this assertion passes and masks an unintended side effect. Distinguish
`NotFound` from other errors so real I/O failures fail the test.

-         assert_eq!(
-             fs::read(root.path().join("Cargo.lock")).ok(),
-             before.get("Cargo.lock").cloned()
-         );
+         let lock = match fs::read(root.path().join("Cargo.lock")) {
+             Ok(bytes) => Some(bytes),
+             Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
+             Err(error) => panic!("reading Cargo.lock failed: {error}"),
+         };
+         assert_eq!(lock, before.get("Cargo.lock").cloned());

─── tests/preview.rs:32-42 ───
[test · medium] The snapshot only records file contents: `is_dir()` and `fs::read()` both follow
symlinks, and directories, file types, and permission bits are never captured. As a result, the
suite's read-only guarantees (`assert_eq!(snapshot(...), before)`) can pass even if preview/apply
creates an empty directory (e.g. `.armorer/`), changes file modes, or swaps a symlink for a regular
file with identical bytes. Following directory symlinks also risks unbounded recursion on cycles.
Record the entry kind via `entry.file_type()` (which does not follow links), include directory
entries and symlink targets (`fs::read_link`), and capture permission bits so "no mutation"
assertions are actually meaningful.

          for entry in fs::read_dir(path).unwrap() {
-             let path = entry.unwrap().path();
-             if path.is_dir() {
-                 walk(root, &path, output);
-             } else {
+             let entry = entry.unwrap();
+             let path = entry.path();
+             let file_type = entry.file_type().unwrap();
+             let key = path.strip_prefix(root).unwrap().to_string_lossy().into_owned();
+             if file_type.is_symlink() {
                  output.insert(
-                     path.strip_prefix(root).unwrap().to_string_lossy().into(),
-                     fs::read(path).unwrap(),
+                     key,
+                     fs::read_link(&path).unwrap().to_string_lossy().into_owned().into_bytes(),
                  );
+             } else if file_type.is_dir() {
+                 output.insert(format!("{key}/"), Vec::new());
+                 walk(root, &path, output);
+             } else {
+                 output.insert(key, fs::read(&path).unwrap());
              }
          }

─── tests/preview.rs:233-234 ───
[test · medium] The leak assertion scans only stdout, but error diagnostics are typically written to
stderr, so a `DO-NOT-ECHO` leak on stderr would pass. Also, unlike every other test in this file,
this one never snapshots the consumer before/after the failed preview, so a preview that replaced
the symlink with a regular file or created `.armorer/` would go undetected. Check both output
streams and verify the link itself is still a symlink (plus snapshot equality once the snapshot
helper records file types).

      assert_eq!(output.status.code(), Some(1));
      assert!(!String::from_utf8_lossy(&output.stdout).contains("DO-NOT-ECHO"));
+     assert!(!String::from_utf8_lossy(&output.stderr).contains("DO-NOT-ECHO"));
+     assert!(
+         fs::symlink_metadata(root.path().join("rust-toolchain.toml"))
+             .unwrap()
+             .file_type()
+             .is_symlink()
+     );

─── tests/preview.rs:186-189 ───
[test · medium] `unwrap_or_default()` silently converts any read error (not just absence) into empty
content, and for `rust-toolchain.toml` / `rust-toolchain` — which the fixture never creates — this
loop creates a new file rather than mutating an existing input, so the intended "changed saved-plan
input" scenario isn't what gets exercised for those cases. The bare `is_err()` also cannot
distinguish the intended staleness rejection from an unrelated parse/validation failure caused by
the appended text (e.g. an invalid legacy toolchain channel), so the test can pass for the wrong
reason. Handle `NotFound` explicitly and assert the error is specifically the staleness error (its
message contains "stale").

-         let old = fs::read_to_string(&path).unwrap_or_default();
-         fs::write(path, format!("{old}\n# changed\n")).unwrap();
+         let old = match fs::read_to_string(&path) {
+             Ok(text) => text,
+             Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
+             Err(error) => panic!("{input}: {error}"),
+         };
+         fs::write(&path, format!("{old}\n# changed\n")).unwrap();
          let before = snapshot(root.path());
-         assert!(preview_plan(root.path(), &plan).is_err(), "{input}");
+         let error = preview_plan(root.path(), &plan).unwrap_err();
+         assert!(error.to_string().contains("stale"), "{input}: {error}");

─── tests/preview.rs:294-299 ───
[test · low] This test hard-depends on an external GNU-compatible `patch` being on PATH. If the
binary is missing, `.output().unwrap()` panics with an opaque I/O error, and BSD/macOS `patch`
handles CRLF and "\ No newline at end of file" hunks differently, producing environment failures
rather than a signal about the product's diffs. Probe for GNU patch first (e.g. run `patch
--version` and check for "GNU") and skip with an explicit message when unavailable, or otherwise
document/gate the toolchain requirement.

─── tests/preview.rs:61-62 ───
[test · low] The exactness assertions index `files[0]` / `changes[0]` without asserting the
collection length or selecting the entry by path (this pattern repeats in the conflict, ownership,
legacy-toolchain, and independent-patch tests). `Preview.files` is a `Vec` derived from
`plan.changes`, so if another managed file is ever added or ordering changes, these assertions
silently target the wrong entry — or panic out of bounds — instead of failing with a meaningful
message. Assert the expected length (e.g. `assert_eq!(result.files.len(), 1)`) or look entries up
via `result.files.iter().find(|f| f.path == "rust-toolchain.toml")`.

─── tests/cargo_graph_v2.rs:136-141 ───
[test · medium] These tamper-rejection loops assert only `is_err()`, so a mutation rejected for an
*unrelated* reason still passes and a regression in the targeted check goes unnoticed. Example:
`/nodes/0/features` is index-based — if fixture ordering drifts, the pointer hits a different node
and the error may come from `invalid-cargo-feature-inventory` or an identity check instead of the
intended `cargo-graph-requested-feature-missing`. The feature tests later in this file already pin
exact error strings; do the same here by pairing each pointer with its expected code
(`cargo-graph-input-mismatch`, `cargo-graph-selection-mismatch`, `cargo-graph-coverage-mismatch`,
etc.), and apply the same to the SBOM mutation loop and the completeness loop below.

─── tests/cargo_graph_v2.rs:613-617 ───
[test · medium] This test runs unconditionally (not `#[ignore]`d) but is Unix-only: `.env_clear()`
strips `SystemRoot`/`TEMP` that Windows processes need, `PATH` is hardcoded to `/usr/bin:/bin`, and
`cargo.with_file_name("rustc")` produces `rustc` without `.exe` on Windows. It will fail on any
non-Unix CI leg. Gate the test with `#[cfg(unix)]` (or set up the environment portably: preserve
required system vars and use the platform executable suffix for `RUSTC`).

─── tests/cargo_graph_v2.rs:313-313 ───
[test · low] The suite's theme is that the producer cannot choose its own consumer context, yet here
the expected `source_commit` (and `native_target`) are taken verbatim from the producer-supplied
receipt with no independent cross-check — a coherently forged receipt+graph pair passes.
`runtime_commit` is at least cross-checked against `lock.workflows.commit`; give `source_commit` the
same treatment (pin it as a fixture constant like `context()` does, or verify it against another
retained artifact). Also note this interop path is `#[ignore]`d, so it never runs in default CI —
make sure some pipeline actually sets `ARMORER_TEST_GRAPH_RECEIPTS`.

─── tests/cargo_graph_v2.rs:252-252 ───
[maintainability · low] `remove(0)` assumes the root's first dependency edge is the runtime `dep`
edge (true for today's fixture). The same test already resolves nodes and SBOM entries by id/ref —
resolve this edge by `package == "dep"` too, so a fixture reorder can't silently retarget the
scenario. Same pattern applies to `graph.nodes[0].features = actual;` in
`real_cargo_resolution_distinguishes_requested_flags_from_resolved_features`, which should look up
the root node by id like the other feature tests do.

-     let normal = graph.nodes[root].dependencies.remove(0);
+     let dep_edge = graph.nodes[root]
+         .dependencies
+         .iter()
+         .position(|e| e.package == "dep")
+         .unwrap();
+     let normal = graph.nodes[root].dependencies.remove(dep_edge);

─── tests/cargo_graph_v2.rs:128-128 ───
[test · low] The substitution matrix exercises every `Selection` field except `package` and
`feature_set`. Production binds them via whole-struct equality (`self.selection == *selection`), but
that binding is only regression-tested field-by-field here — add the two missing mutations so a
future switch to per-field comparison can't silently drop them.

          ("/selection/toolchain", json!("1.94.0")),
+         ("/selection/package", json!("other")),
+         ("/selection/feature_set", json!("minimal")),

─── tests/inspection.rs:316-320 ───
[test · medium] Vacuous assertion: `bytes` is an owned `Vec<u8>` captured from the earlier
`load_lock` call and never re-read, so comparing it to `lock.as_bytes()` after rewriting the file is
trivially true. It cannot detect a re-read/TOCTOU regression, so the stated 'loaded bytes remain
bound to the validated lock' guarantee is never exercised. To actually test binding, re-run the
pipeline (e.g. `inspect`) after the rewrite and assert it hashes/uses the originally validated
bytes, or drop this misleading assertion.

─── tests/inspection.rs:425-428 ───
[test · medium] Environment-sensitive panic: this `.unwrap()` on the PATH search aborts the test on
any host without rustup on PATH (and `var_os("PATH").unwrap()` panics if PATH is unset). Prefer
skipping gracefully (e.g. early `return` / `eprintln!` + return when rustup isn't found) so the
suite doesn't fail on rustup-less CI images.

─── tests/inspection.rs:578-584 ───
[test · medium] Portability/flakiness: hardcoding `/usr/bin/mkfifo` breaks on systems where mkfifo
lives elsewhere (e.g. `/bin/mkfifo`); resolve it via PATH or use a libc/nix mkfifo. Also the fixed
2s wall-clock kill deadline can false-positive under CI load. Consider a more generous/adaptive
timeout.

─── tests/inspection.rs:603-603 ───
[test · low] Weak assertion: checking only exit code 1 lets any unrelated failure satisfy the
'non-blocking FIFO rejection' guarantee. Capture stdout and assert the specific error code (as the
other CLI tests do) so this proves the FIFO was rejected for the right reason.

─── tests/inspection.rs:174-182 ───
[test · medium] On non-unix hosts the provider script is never made executable (the chmod is
`#[cfg(unix)]`) and a `#!/bin/sh` script isn't a runnable Windows executable, so `!marker.exists()`
passes trivially without proving rustc-wrapper/credential-provider exclusion. Gate the whole test
with `#[cfg(unix)]` (or make the provider genuinely executable cross-platform) so it doesn't give
false confidence on Windows.

─── tests/inspection.rs:397-400 ───
[test · medium] Silent no-op risk: if the replace target drifts (whitespace/ordering), `replace`
returns the manifest unchanged and `is_ok()` still passes, so the feature-cycle path is never
actually exercised. Assert the mutation took effect (e.g. `assert_ne!(manifest, mutated)` or assert
the plan reflects the cycle) before asserting success.

─── tests/inspection.rs:493-493 ───
[test · low] Same silent no-op risk: `is_ok()` alone passes even if the `[package]` replace fails to
match, leaving anchored-pattern/opaque-metadata coverage untested. Assert the replacement actually
changed the manifest before asserting success.

─── tests/inspection.rs:411-411 ───
[test · low] Environment-dependent: the `cargo-discovery` code here relies on rustup-managed
resolution of `99.99.99` (with auto-install disabled). On a host without rustup, `trusted_rustup`
fails first and returns the same `cargo-discovery` code for an unrelated reason (`rustup-path`), so
the test passes without actually exercising the missing-toolchain path. Consider asserting the error
message identifies the missing toolchain (not `rustup-path`) to make the guarantee meaningful.

─── tests/real_sigstore.rs:263-268 ───
[test · high] This security gate asserts only `is_err()` for every identity-mismatch case (and the
same pattern is used for all other negative checks: the tamper loop, artifact flips, `GithubPrivate`
backend, approval-digest mismatch, wrong root, expired review, verifier substitution, and the mixed
download). Any unrelated failure — e.g. a native `gh` exec/runtime error or an
`OfflineVerifier::open` failure for a different reason — would satisfy the assertion without proving
the intended enforcement path fired, silently weakening this required CI gate. Match the specific
`Error::Invalid(code)` per case, as the first check already does with
`"verification-input-type-or-size"`, and include the actual error in the panic message. At minimum,
assert the error class:

-         assert!(
-             wrong_verifier
+         let err = wrong_verifier
-                 .verify(&artifact, &expected_bytes, &bundle, &wrong)
+             .verify(&artifact, &expected_bytes, &bundle, &wrong)
-                 .is_err(),
-             "{case}"
+             .expect_err(case);
+         assert!(
+             matches!(err, armorer::Error::Invalid(_)),
+             "{case}: unexpected error class: {err:?}"
          );

─── tests/real_sigstore.rs:174-174 ───
[test · low] Replacing the first base64 character with 'A' is not guaranteed to mutate anything: if
the fixture signature already starts with 'A', the bundle stays byte-identical and this "tamper"
case becomes an unmutated positive verification (the test then fails spuriously on any fixture
rotation). Also `&sig[1..]` panics on an empty string. The same pattern is repeated for the
certificate `rawBytes` below. Decode, flip a bit, and re-encode so the mutation is guaranteed
regardless of fixture content:

-     signature["dsseEnvelope"]["signatures"][0]["sig"] = format!("A{}", &sig[1..]).into();
+     let mut sig_bytes = STANDARD.decode(sig).unwrap();
+     sig_bytes[0] ^= 1;
+     signature["dsseEnvelope"]["signatures"][0]["sig"] = STANDARD.encode(sig_bytes).into();

─── tests/trust_contracts.rs:601-602 ───
[test · medium] The test name promises unknown-field rejection, but no payload with an unknown field
is ever written — only duplicate keys, oversize, traversal and symlink cases are exercised. Add an
unknown-field case built from a valid record (so it can't fail for missing required fields instead),
e.g. serialize `policy()`, inject an extra key, and assert `load_json::<VerificationPolicy>` rejects
it. Otherwise the claimed `deny_unknown_fields` contract is untested.

      std::fs::write(&path, br#"{"outer":{"required":true,"required":false}}"#).unwrap();
      assert!(armorer::trust::load_json::<serde_json::Value>(temp.path(), "contract.json").is_err());
+     let mut value = serde_json::to_value(policy()).unwrap();
+     value["unexpected_field"] = true.into();
+     std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
+     assert!(armorer::trust::load_json::<VerificationPolicy>(temp.path(), "contract.json").is_err());

─── tests/trust_contracts.rs:171-173 ───
[test · medium] These mutations (traversal name, `INVENTORY_BUNDLE_NAME`, and the self-referencing
subject above) all alter asset[0]'s name/subjects, which already fails the expected-asset-set
comparison — the same rejection the `"extra.tar.gz"` rename case proves. So the bare `is_err()`
assertions pass even if the path-safety, reserved-name, or cycle checks were deleted; they don't
isolate the security property the test name claims. Pin the rejection reason via the error code, as
`producer_cannot_refresh_independently_approved_tool_observation_time` already does with
`unwrap_err().to_string()`.

─── tests/trust_contracts.rs:587-587 ───
[maintainability · low] On subprocess failure the assertion discards `output.stderr`, so a
schema-generation crash reports only "schema {kind}" with no diagnostic. Include stderr in the
failure message.

-         assert!(output.status.success(), "schema {kind}");
+         assert!(
+             output.status.success(),
+             "schema {kind}: {}",
+             String::from_utf8_lossy(&output.stderr)
+         );

─── tests/release_context.rs:205-208 ───
[test · high] Vacuous on unsupported hosts: `TrustedReleaseContext::open` always fails at the
`qualified_native_validator()?` gate (src/verification/context.rs:323) when the host is unsupported,
and `fixture()` deliberately embeds a bogus validator identity in that case. Since this test only
asserts `is_err()` and is not `cfg_attr`-gated like the other semantic tests, it passes on
unsupported hosts even if every config/lock/catalog/policy substitution check were deleted. Add the
same `#[cfg_attr(..., ignore)]` gate used by the other semantic tests, and/or assert the specific
authority-mismatch error code so an unrelated rejection path cannot satisfy the assertion.

─── tests/release_context.rs:381-381 ───
[test · medium] Same unsupported-host vacuity as the substitution test: this test is not
`cfg_attr`-gated and asserts only `is_err()`. On hosts where `qualified_native_validator()` fails,
`open` errors at the native-validator gate regardless of symlink handling, so a regression in
symlink rejection would go undetected there. Gate the test like the other semantic tests or assert
the specific symlink/type rejection error code for each case.

─── tests/release_context.rs:165-167 ───
[test · medium] The unsupported-host branch accepts any error, so a broken fixture (bad JSON,
missing file, wrong digest) would still pass here and silently mask loss of the intended coverage.
Assert that the error is specifically the unsupported-native-validator rejection rather than bare
`is_err()`.

─── tests/release_context.rs:1076-1083 ───
[test · medium] On unsupported hosts no assertion is made about the error message at all — any
exit-1 JSON error passes, including one produced by an unrelated early-exit path. Add an `else`
branch asserting the expected unsupported-host/native-validator rejection message so the check is
never silently skipped (the file's own doc comment on `supported()` promises "never skip a test
silently").

─── tests/release_context.rs:47-47 ───
[test · low] `now()` is sampled repeatedly (twice here, once per tool in the `observed_at` map,
again in the expiry/observation mutations) and each sample is signed into the approved digest, so
related timestamps can straddle a second boundary and the fixtures are non-reproducible. Capture one
instant at the top of `fixture()` (`let now = now();`) and derive
`reviewed_at`/`expires_at`/`observed_at` from it; pass the same base into the mutation tests.
Margins are currently wide enough to avoid flakes, but a single clock read removes the risk and
makes digests deterministic per run.

─── tests/release_context.rs:598-599 ───
[test · medium] These slices hardcode the exact layout of `synthetic-runtime.tar` (1024-byte stride,
128-byte members at +512). The container-mutation test's fixed offsets
(148/156/257/512/640/1536/3500) and the `len() == 128` assertion share the same coupling. If the
binary fixture drifts, this panics on slice bounds or — worse — the mutation test flips an
unintended byte and the archive-safety assertions silently test the wrong field. Add an explicit
layout sanity check before slicing (e.g., `assert!(archive.len() >= targets.len() * 1024 + 1024)`
plus a ustar-magic/member-size check), or derive offsets from the tar headers, so drift fails with a
clear diagnostic.

─── tests/release_context.rs:305-305 ───
[maintainability · low] The wildcard arm silently absorbs any index: bumping `0..14` without adding
an arm re-applies the `apple_team` mutation instead of failing, and adding arm `13` without bumping
the range never runs it. Make the last case explicit and end with `_ => unreachable!()` (the v3 test
in this file already uses that pattern) so range/arm drift fails loudly.

-             _ => selected["evidence_requirements"]["apple_team"] = "ABCDE12345".into(),
+             13 => selected["evidence_requirements"]["apple_team"] = "ABCDE12345".into(),
+             _ => unreachable!(),

─── tests/release_context.rs:761-761 ───
[maintainability · low] Same wildcard-arm hazard as the baseline loop: an out-of-sync `0..15` range
silently re-runs the `build_workflow` mutation or drops a new case. Use an explicit `14 =>` arm plus
`_ => unreachable!()`.

-             _ => catalog["runtime"]["build_workflow"]["commit"] = "e".repeat(40).into(),
+             14 => catalog["runtime"]["build_workflow"]["commit"] = "e".repeat(40).into(),
+             _ => unreachable!(),

─── tests/release_context.rs:860-860 ───
[maintainability · low] Same wildcard-arm hazard in the `0..12` archive-mutation loop: range/arm
drift silently re-applies the `archive[1536] ^= 1` mutation. Use an explicit `11 =>` arm plus `_ =>
unreachable!()`.

-             _ => archive[1536] ^= 1,
+             11 => archive[1536] ^= 1,
+             _ => unreachable!(),

─── tests/historical.rs:252-260 ───
[test · medium] Vacuous size-cap assertion: the 1 MiB+1 file is checked against `"a".repeat(64)`,
which can never match its content. If the size gate in `io::read_bounded`/`regular` were removed,
this would still fail with `unapproved-historical-policy` and the test stays green. Use the file's
real digest so only the size cap can reject it, and assert the specific code (`regular()` emits
`verification-input-type-or-size` for `len > max`, before the digest comparison). Likewise, the
`../escape` assertion just above accepts any error — pin it to the asset-name rejection (e.g.
`invalid-historical-asset-name` from policy validation).

-     assert!(
-         HistoricalByteMatch::verify(
+     let digest = ByteIdentity::from_bytes(&fs::read(&path).unwrap()).sha256;
+     let error = HistoricalByteMatch::verify(
-             &directory.path().join("download"),
+         &directory.path().join("download"),
-             &path,
+         &path,
-             &"a".repeat(64),
+         &digest,
-             &source
+         &source
-         )
+     )
-         .is_err()
+     .err()
+     .unwrap();
+     assert!(
+         error.to_string().contains("verification-input-type-or-size"),
+         "{error}"
      );

─── tests/historical.rs:89-91 ───
[test · medium] These markers can never be observed here: `touch executed` and any cargo build
resolve against the *process* working directory (the crate root under `cargo test`), not the tempdir
— and `target/` legitimately exists at the crate root anyway. So actual execution/extraction would
go undetected while these assertions always pass. Exercise the scenario in a child process with an
explicit `.current_dir(directory.path())` (the CLI test already spawns one) so markers land where
they are checked, or assert against the real CWD for `executed`.

─── tests/historical.rs:149-149 ───
[test · medium] Security negatives accept any error: a broken source-pinning guard could hide behind
an unrelated earlier failure (fixture drift, review expiry, digest handling). The implementation
distinguishes `historical-release-not-allowed` (source mismatch) from
`historical-release-mode-required` (non-release ref) — assert those, as the attestation/total-byte
tests already do. Same applies to the bare `is_err()` checks for `modern`, `collision`, and
`rehearsal` below.

-         assert!(verify(directory.path(), &policy, &wrong).is_err());
+         let error = verify(directory.path(), &policy, &wrong).err().unwrap();
+         let message = error.to_string();
+         assert!(
+             message.contains("historical-release-not-allowed")
+                 || message.contains("historical-release-mode-required"),
+             "case {case}: {error}"
+         );

─── tests/historical.rs:195-197 ───
[test · medium] Bare `is_err()` lets a broken byte comparison hide behind set/IO errors. Pin each
case to its intended failure: tampered bytes → `historical-asset-byte-mismatch`; extra file →
`historical-download-file-set-mismatch`; extra directory →
`historical-download-entry-type-or-count`; missing file → a NotFound IO error. At minimum assert the
tamper case's byte-mismatch code — that is the core guarantee of this test.

─── tests/historical.rs:301-301 ───
[test · medium] Any error passes, so a broken symlink/special-file guard can hide behind an
unrelated rejection. Note case 3's socket is also an unapproved extra entry, so a plain file-set
mismatch would mask broken special-file handling. Assert per-case codes: linked policy/asset →
`verification-input-type-or-size` (`regular()` uses `symlink_metadata`), linked download dir →
`historical-download-directory-type`, socket → `historical-download-entry-type-or-count`.

─── tests/historical.rs:369-372 ───
[test · low] Pin the rejection reason: the scan loop maps non-UTF-8 names to
`historical-download-name-invalid`; a bare `is_err()` would also pass if the file were rejected
merely as an extra set member.

          b"invalid name",
      )
      .unwrap();
-     assert!(verify(directory.path(), &policy, &source).is_err());
+     let error = verify(directory.path(), &policy, &source).err().unwrap();
+     assert!(
+         error.to_string().contains("historical-download-name-invalid"),
+         "{error}"
+     );

─── tests/historical.rs:113-116 ───
[test · medium] These inputs also fail `VerificationPolicy` schema validation (`{}` is missing every
required field), so `is_err()` passes even with a lenient parser — serde_json accepts duplicate keys
by default. This does not prove trailing-data/duplicate-key rejection. Start from an otherwise-valid
approved policy (e.g. the fixture), append trailing data or duplicate an existing key, recompute the
digest, and assert failure — then only the strict-parse gate can reject it.

─── tests/historical.rs:334-334 ───
[test · medium] Two gaps: (1) the test name claims "never auto-selects policy", but every invocation
passes `--policy` and `--expect-policy-sha256` explicitly — add a run omitting them and assert
nonzero exit so the required-argument contract is actually exercised. (2) On the failure path this
unconditional stdout-JSON unwrap panics with no exit-code/stderr context if the CLI ever reports
errors differently; include status and stderr in the panic message.

-         let result: Value = serde_json::from_slice(&output.stdout).unwrap();
+         let result: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
+             panic!(
+                 "non-JSON stdout ({error}); status: {:?}; stderr: {}",
+                 output.status,
+                 String::from_utf8_lossy(&output.stderr)
+             )
+         });

─── tests/upgrade.rs:393-396 ───
[test · medium] This raw string mutation is fragile and can pass for the wrong reason. For a fresh
fixture the plan's `state_preimages` map may be empty, serializing as `"state_preimages":{}`; the
replacement then yields `{".armorer/state.json":null,}` — a trailing comma, i.e. malformed JSON.
`load_plan` fails at decode with a syntax error, so the final `is_err()` never exercises
null-preimage rejection at all. If the key already exists in the map, you instead get duplicate JSON
keys (serde_json silently keeps the last). Build the mutation via `serde_json::Value` so the
document stays well-formed, and assert the preimage map's prior shape so the test deterministically
exercises strict reconstruction/digest validation rather than an accidental parse failure.

-     let raw = serde_json::to_string(&p).unwrap().replace(
-         "\"state_preimages\":{",
-         "\"state_preimages\":{\".armorer/state.json\":null,",
-     );
+     let mut value = serde_json::to_value(&p).unwrap();
+     let preimages = value["state_preimages"].as_object_mut().unwrap();
+     assert!(!preimages.contains_key(".armorer/state.json"));
+     preimages.insert(".armorer/state.json".into(), serde_json::Value::Null);
+     let raw = serde_json::to_string(&value).unwrap();

─── tests/upgrade.rs:872-875 ───
[test · medium] The plan `p` is computed before the symlink is created, so these negative assertions
may pass for the wrong reason: `upgrade::apply` re-validates freshness against the current
inventory, and replacing `armorer.lock` / `armorer-ci.yml` with a symlink already invalidates the
pre-symlink plan — it would fail even with no symlink defense at all. Likewise `recover` can fail
simply because no valid journal exists. The outside-file and `is_symlink()` checks help, but a
staleness-rejected apply never writes anything anyway, so the symlink overwrite guard is not
actually proven here. Assert the failure cause (e.g. error text identifying the symlink rejection)
for `inspect`, `apply`, and `recover` so the test can't silently degrade into a generic staleness
check.

─── tests/upgrade.rs:592-592 ───
[test · low] mtime-equality is a weak no-op signal: on filesystems with coarse timestamp granularity
(1–2s ticks, common on CI volumes), an implementation that rewrites identical bytes during the
"unchanged" replay within the same tick passes this assertion, giving false confidence that no
rewrite occurred. Also, `Metadata::modified()` is unsupported on some platforms and will panic via
`unwrap()`. Ensure at least one filesystem timestamp tick elapses between the first apply and the
replay (or compare a stronger signal such as ctime/inode where available) so a spurious rewrite is
reliably detected.

─── tests/upgrade.rs:965-969 ───
[test · low] This test hard-depends on an external GNU `patch` (`--batch` and `-E` are GNU-specific;
BSD patch differs, Windows/minimal containers usually lack it). `spawn().unwrap()` panics with an
opaque "No such file or directory" when the binary is missing — use `.expect("GNU patch is required
for independent diff reconstruction")` or skip gracefully. Additionally, `patch` reports failures on
stderr, but the assertion message prints only stdout, making real diff-reconstruction failures hard
to diagnose; include `result.stderr` in the message.

              assert!(
                  result.status.success(),
-                 "{path}: {}",
-                 String::from_utf8_lossy(&result.stdout)
+                 "{path}: {}{}",
+                 String::from_utf8_lossy(&result.stdout),
+                 String::from_utf8_lossy(&result.stderr)
              );

