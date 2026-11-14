# Review findings — group F_verification (19 findings)

─── src/verification/apple.rs:502-503 ───
[bug · medium] These in-process `check_validity` calls run with `kSecCSAllowNetworkAccess` (bit 16)
and have no deadline. Only the `codesign` subprocess is timeout-bounded (60s via `run_process`); the
Security.framework validation here can block on network ticket/cert lookup indefinitely. The
20-minute stage budget (`started.elapsed() <= ...`) is checked only *after* `native()` returns, so a
stalled network validation hangs the verifier rather than being cut off. Consider enforcing a
deadline around the whole native stage (e.g. run it on a bounded worker / watchdog) so network
stalls fail closed within the budget.

─── src/verification/apple.rs:103-103 ───
[performance · low] `archive` (up to ~1 GiB) is unused after `unpack` but, being a `Vec` with a
destructor, stays allocated until the end of the loop body — i.e. throughout the expensive
`native()` call, which itself re-reads up to 1 GiB from disk several times. That needlessly inflates
peak memory (archive + extracted binary + transient re-read buffers). Drop it right after
extraction.

              let binary = unpack(&archive, binary_name)?;
+             drop(archive);

─── src/verification/apple.rs:178-183 ───
[maintainability · low] This is byte-for-byte identical to the existing `ByteIdentity::from_bytes`
(src/trust/mod.rs), which is already `pub` and in scope via the `ByteIdentity` import. Reuse it
(`ByteIdentity::from_bytes(bytes)`) and delete this helper instead of maintaining a second copy of
the same digest+size construction.

─── src/verification/context.rs:287-288 ───
[security · medium] Cargo.lock is the only pinned input whose authenticated bytes are not retained:
config/lock/catalog/policy are digest-checked and parsed from the same read, but here the bytes are
hashed inside the `require!` and dropped. Any later consumer must re-read `Cargo.lock` from disk,
and the file can be swapped after this check (TOCTOU), so the trusted context no longer binds the
content actually used downstream. Read the bytes into a variable, verify the digest, and retain them
(or expose an accessor / require consumers to re-verify every read against
`inputs.cargo_lock_sha256`) so the authenticated snapshot is what gets used.

─── src/verification/context.rs:396-399 ───
[security · medium] Ambiguous composite key: `identifier()` allows consecutive hyphens in
deliverable ids and feature-set names, while targets are fixed `--`-free triples. Distinct artifacts
can therefore produce identical keys, e.g. (id=`x--aarch64-apple-darwin--y`,
target=`aarch64-apple-darwin`, fs=`z`) and (id=`x`, target=`aarch64-apple-darwin`,
fs=`y--aarch64-apple-darwin--z`) both yield `x--aarch64-apple-darwin--y--aarch64-apple-darwin--z`.
`expected_keys` then collapses two config-legal selections into one, the `selections.len() ==
expected_keys.len()` check approves a context covering only one of the two artifacts (the other
silently escapes evidence requirements), and the v3 `root_features` map cannot distinguish them.
Make the encoding injective — minimally reject `--` in `deliverable.id`/`deliverable.feature_set`
here (targets are fixed), or length-prefix/escape components — and apply the same fix to
`Selection::key()` so both sides agree.

─── src/verification/context.rs:441-442 ───
[security · low] macOS signing/notarization gating is hardcoded to one exact triple.
`config::TARGETS` is the authority for supported targets; if another Darwin triple (e.g.
`x86_64-apple-darwin`) is ever added there, this comparison silently classifies it as non-mac and
bypasses apple_team, Sign/Notarize steps, and AppleSigning coverage — a fail-open security gate.
Match the platform suffix (or use a shared `is_macos_target` helper next to `TARGETS`) so new Apple
targets fail closed.

-             let mac =
-                 selection.profile != Profile::Library && selection.target == "aarch64-apple-darwin";
+             let mac = selection.profile != Profile::Library
+                 && selection.target.ends_with("-apple-darwin");

─── src/verification/context.rs:158-159 ───
[maintainability · low] Doc says "rejecting adapters with no pins", but the Legacy branch's
`pins.iter().all(...)` is vacuously true for a pin-less adapter, unlike the NativeV2 branch's
explicit `!pins.is_empty()` guard. This is only safe today because `Catalog::validate`
(capability.rs) separately rejects empty pins and `open_version` requires the baseline adapters — an
invariant enforced in another module. Mirror the explicit non-empty guard here so this security
check doesn't silently degrade if that distant validation changes.

                      .all(|adapter| {
-                         adapter.pins.iter().all(|pin| {
+                         !adapter.pins.is_empty() && adapter.pins.iter().all(|pin| {

─── src/verification/cyclonedx.rs:155-155 ───
[maintainability · medium] All `run_process` failure modes are collapsed into a single misleading
diagnostic. `run_process` can fail due to spawn failure (e.g. exec/loader problem), the 30s timeout,
stdout/stderr exceeding the 1 MiB caps, or a nonzero exit (actual schema errors). Reporting every
one of these as `sbom-schema-validation-failed` tells operators the document is invalid when the
real cause may be infrastructure (a timeout on a large-but-valid 17 MiB SBOM, or a spawn failure),
leading to wrong remediation. This doesn't require echoing untrusted child output: distinguish the
failure classes with distinct fixed codes (e.g. `sbom-validator-execution-failed` /
`sbom-validator-timeout` vs `sbom-schema-validation-failed`) by inspecting the returned error before
discarding it, keeping the fail-closed behavior intact.

─── src/verification/graph.rs:403-406 ───
[bug · medium] `nodes[id]` panics on a missing key. Today the invariant holds because `compare_sbom`
only calls this after `index()` proved node/package bijection and full reachability, but the sibling
reachability walk in `index()` deliberately uses fallible `.get()` for exactly this traversal. If
the coverage invariant ever diverges (e.g. a future caller passes a filtered map, or `index()`
changes), this verification library panics on untrusted input instead of returning a typed error.
Make the walk fallible like `index()`: return `Result<BTreeSet<&'a str>>`, use
`.get(id).ok_or_else(|| crate::Error::Invalid("cargo-graph-node-missing".into()))?`, and propagate
with `?` at the `compare_sbom` call site.

              if required.insert(id) {
+                 let node = nodes
+                     .get(id)
+                     .ok_or_else(|| crate::Error::Invalid("cargo-graph-node-missing".into()))?;
                  pending.extend(
-                     nodes[id]
-                         .dependencies
+                     node.dependencies

─── src/verification/graph.rs:383-385 ───
[bug · medium] `native_linkage` is the only evidence collection here without a uniqueness check:
packages, nodes, edges, features, and contexts all reject duplicates, but repeated `package_id`
entries (up to MAX_PACKAGES) pass, allowing conflicting or inflated linkage evidence for the same
package to be accepted as valid. Deduplicate by `package_id` with a `BTreeSet`, consistent with the
other indexes.

+         let mut native_ids = BTreeSet::new();
          for native in &self.native_linkage {
              require(
-                 packages.contains_key(native.package_id.as_str())
+                 native_ids.insert(native.package_id.as_str())
+                     && packages.contains_key(native.package_id.as_str())

─── src/verification/graph.rs:144-147 ───
[maintainability · low] `sbom-string-field-missing` is reported both when the key is absent and when
it is present but not a string (e.g. a numeric `version`), which misdirects producer debugging.
Distinguish the two cases with a second opaque code — this preserves the no-echo policy while making
the diagnostic actionable.

-     value
-         .get(key)
-         .and_then(Value::as_str)
-         .ok_or_else(|| crate::Error::Invalid("sbom-string-field-missing".into()))
+     match value.get(key) {
+         None => Err(crate::Error::Invalid("sbom-string-field-missing".into())),
+         Some(v) => v
+             .as_str()
+             .ok_or_else(|| crate::Error::Invalid("sbom-string-field-invalid".into())),
+     }

─── src/verification/bundle.rs:39-42 ───
[documentation · low] The documented contract ("exact unverified bare transport bytes") does not
hold for the `load_download` path: there, `bytes` is the re-serialized `serde_json::to_vec(&value)`
normalization, not the bytes actually received on the line (whitespace/key order are rewritten,
wrapper metadata stripped). Current downstream consumers are self-consistent (snapshots are written
from these same retained bytes), so nothing breaks today — but in a security-sensitive crate, a
future caller relying on this doc for byte-identity against the received transport would be misled.
Either retain the original line bytes in `load_download`, or reword the doc to state that download
records are stored in canonicalized form.

─── src/verification/bundle.rs:71-73 ───
[maintainability · low] A base64 decode failure is mapped to `Error::Json`, whose display is "JSON
serialization failed" — an actively misleading diagnostic for an encoding fault in the DSSE payload
(it is neither JSON nor serialization). The crate's opaque fixed-code convention is fine, but this
case deserves its own static code, e.g. via the same mechanism `require` uses.

      let payload = STANDARD
          .decode(&bundle.dsse_envelope.payload)
-         .map_err(|_| Error::Json)?;
+         .map_err(|_| Error::Transaction("attestation-payload-encoding"))?;

─── src/verification/bundle.rs:73-73 ───
[maintainability · low] Correction to my previous suggestion on this line: use the same mechanism as
`trust::require`, i.e. `Error::Invalid` with a fixed static code (`Transaction` is reserved for
transaction-blocking failures). The core finding stands: mapping a base64 decode failure to
`Error::Json` ("JSON serialization failed") is a misleading diagnostic for a payload-encoding fault.

-         .map_err(|_| Error::Json)?;
+         .map_err(|_| Error::Invalid("attestation-payload-encoding".into()))?;

─── src/verification/historical.rs:75-75 ───
[bug · low] An empty `historical.assets` map passes every gate here: `policy.validate()`'s
`assets.values().all(...)` is vacuously true, `total == 0 <= MAX_TOTAL`, and `compare_directory`
succeeds against an empty directory. `verify` then returns a successful `HistoricalByteMatch` that
matched zero bytes. Given this module's fail-closed design, require a nonempty approved asset set so
an empty allowlist can never produce a vacuous "byte match" result.

-             historical.assets.len() <= MAX_FILES
+             !historical.assets.is_empty()
+                 && historical.assets.len() <= MAX_FILES

─── src/verification/release.rs:285-285 ───
[bug · low] The final full-rehash loop never samples `deadline(started)`, unlike the snapshot,
bundle, and selection loops. It re-reads and re-hashes up to `MAX_TOTAL` (4 GiB) across up to 8194
files, so on slow storage this loop alone can run far past the 20-minute `DEADLINE` after the last
check, breaking the stated "refuse partial work after the deadline" invariant. Add a per-iteration
deadline check here (it's cheap relative to the hashing).

          for (name, identity) in &identities {
+             deadline(started)?;

─── src/verification/release.rs:161-164 ───
[performance · low] O(n²) name lookups at maximum release scale: this per-bundle linear `.find` over
`inventory.assets`, and the per-identity `.find` for role caps in the final recheck loop, each scan
up to ~8192 assets — up to ~67M string comparisons per loop, eating into the fixed 20-minute
verification budget. Since names are unique after `inventory.validate_against`, build a name index
once (e.g. `let by_name: BTreeMap<&str, &Asset> = inventory.assets.iter().map(|a| (a.name.as_str(),
a)).collect();`) and use it for both the subject lookup here and the role-cap lookups in the final
loop (and optionally `asset_bytes`).

─── src/verification/sigstore.rs:682-683 ───
[bug · medium] The 30s deadline in `run_process_status` only breaks the polling loop; it is not a
hard bound on the call. Two things can block past it:

1. `stop()` does a blocking `wait()` after `kill()`, and both errors are ignored. If `kill()` fails,
or the child is in uninterruptible (D-state) I/O where SIGKILL is only delivered once it leaves that
state, `wait()` blocks indefinitely.
2. After the loop breaks (timeout/error), `std::thread::scope` still joins the two reader threads,
which are blocked in `capture()` until pipe EOF. Killing only the direct child closes its pipe ends,
but if a write-end stays open (e.g. inherited by a descendant) the readers never see EOF and the
join hangs.

For the pinned single-process `gh` this is unlikely, but the stated time bound is not actually
guaranteed, which is a DoS hazard in a verification gate. Consider making teardown non-blocking
(e.g. reap with a bounded/non-blocking wait and ensure reader threads cannot outlive the deadline)
so the timeout holds regardless of child behavior.

─── src/verification/runtime.rs:48-48 ───
[bug · low] The 20-minute stage budget is only polled inside `decode` (and once at its end). This
`io::snapshot` call copies up to `MAX_DISTRIBUTION` (~384 MiB) from the untrusted archive path with
no deadline checks, and the later `io::identity` rehash / `sync_all` are likewise uncovered.
`io::regular` rules out FIFOs/symlinks, but a slow or hung filesystem (e.g., network mount) can
still block `open()` indefinitely before the first `deadline(started)` ever runs, so the stage
timeout does not actually bound the whole stage. If the bound is a real requirement, make the copy
loop deadline-aware (pass `started`/a deadline into the snapshot); otherwise document the budget as
best-effort so callers don't rely on it for liveness.

