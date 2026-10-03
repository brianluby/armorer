# Native Apple release verification v1

This candidate extends `verify-release` with mandatory macOS checks for every
CLI/service `aarch64-apple-darwin` selection. The existing private
`AuthenticatedReleaseFiles` result must first authenticate the inventory, exact
assets, Sigstore bundles, source/signer expectations, SBOM predicates and Cargo
graphs against the independently approved `TrustedReleaseContext`.
`VerifiedAppleRelease` accepts those private results, not a producer JSON receipt
or a caller-supplied verification callback. Failure of any required Apple payload
returns no complete Apple proof or successful CLI result.

## Byte and platform boundary

After whole-file authentication, the reader accepts one bounded gzip member and
one regular USTAR executable leaf named by the independently approved selection.
It rejects links, paths, device files, PAX/GNU extensions, extra members, trailer
junk, invalid CRC/checksum, unexpected names, nonzero ownership/time, wrong modes
and nonzero padding. Compressed and executable bytes are capped at 1 GiB. The
reader never extracts offered paths or executes the executable. It writes only a
fixed `payload.macho` file in a private temporary directory, with owner-read-only
permissions, and rehashes it before and after native verification.

The executable identity must match both the signed and notarized step outputs;
the gzip archive must match the package output. The frozen v1 evidence chain and
required workflow/run/input/catalog bindings are independently revalidated.

On macOS, Security.framework statically validates the same private executable
against the Apple generic anchor, the Developer ID Application certificate OID
and the independently approved ten-character team ID. A valid ad-hoc signature
does not meet this requirement. All architectures and strict validation are
requested. The payload reader additionally requires a thin ARM64 executable and
one SHA-256 CodeDirectory with hardened runtime set, ad-hoc signing unset, and
the same team identifier. Signing blob tables are bounded, unique and
nonoverlapping. Alternate CodeDirectories and unrecognized signing slots are
explicitly unsupported by this version.

The pinned native CMS binding verifies the detached CodeDirectory signature and
the current X.509 signer chain. The actual leaf certificate DER SHA-256 must match
the authenticated producer evidence. Apple’s CMS authenticated TSA timestamp is
required; the signer’s ordinary signing-time attribute cannot satisfy this gate.
The timestamp must fall within the authenticated signing step and cannot be in
the future. Timestamped executables whose signer certificate has since expired
are currently rejected by this conservative current-certificate CMS check.
The separately documented exact historical byte-verification command preserves
its original semantics and does not acquire a native Apple release claim.

The final native requirement includes `notarized`, which requires the platform
to find a notarization ticket for the actual CodeDirectory hash. Its result is
reported as `system-ticket-verified-cache-or-network`. It is not proof of a fresh
online response, the producer’s notarization submission UUID or the origin of a
retained producer log. The authenticated log remains producer evidence; the
native ticket is a separate check over the actual executable. No notarization
submission, credential lookup/import, keychain modification, signing or release
publication is implemented by this consumer.

## Supported operation and limitations

Use the existing `verify-release` arguments and an independently reviewed context
digest. Successful output includes `apple_verification` and `apple_payloads`.
`not-required` means the complete approved set contains no Apple executable;
macOS library `.crate` files do not require Developer ID signing. Linux can
verify those library/Linux sets. A set containing an Apple executable requires
macOS and cannot downgrade to file authentication on Linux.

This version supports the frozen `online-standalone-mach-o` ticket mode in
`.tar.gz` packages. It does not claim a stapled ticket on a plain executable or
tarball, and rejects `stapled-package` rather than projecting `.app`, `.pkg` or
`.dmg` semantics onto a tarball. Ticket lookup can use the system cache or the
network; an absent/unavailable required ticket fails. The trusted consumer host,
its Security.framework, system certificate store, and private local filesystem
are platform assumptions. The native APIs can perform their own network I/O;
the twenty-minute stage expiry is checked on return and is not a hard interrupt
of an in-flight platform API call. No same-account concurrent mutation is
supported during the private snapshot verification.

## Qualification and acceptance

Ordinary tests cover checked gzip trailers, unsafe archive forms, ambiguous
Mach-O/signature offsets, wrong team, missing runtime/CMS and requirement-input
injection. Parser fixtures contain fake CMS and are explicitly not a signed
positive. A separate macOS test compiles only an owned constant C fixture, signs
it ad-hoc with hardened runtime, verifies that ordinary codesign accepts it and
requires the real native Developer ID gate to reject it. The fixture is never
executed and uses no Developer ID identity or notarization submission:

```console
cargo test --locked real_hardened_adhoc_signature_cannot_satisfy_developer_id -- --ignored --test-threads=1
```

A separately pinned public Momus v0.3.1 executable supplies a genuine native
reference positive. Its final archive's provenance was independently verified
against source `d64edff1b47982a408ffca4a551d4d83cdc06740`, tag `v0.3.1` and reusable
signer `290feb94f125b7525302a46e1a37ff5eaefd3465` with the retained offline root.
The reference pins archive/executable digests, public signer certificate DER,
team `DVH6X33J83` and authenticated timestamp. macOS CI downloads only the fixed
archive, rehashes both identities, writes an inert private snapshot and requires
the actual native backend to accept it. Wrong team, wrong certificate and
tampered signed bytes must fail, with no execution. The legacy reference archive
contains README/license files; its explicit fixture-only reader does not change
Armorer's production single-leaf archive parser or construct a complete release
proof. The fixture does not constitute Momus adoption or an Armorer producer
rehearsal. Fixture source review and hosted macOS 15 qualification remain pending.

Hosted macOS 15 qualification, protected Apple producer finalization, final-byte
signing and independent verification of the complete own Armorer release,
independent source review and human acceptance remain required. This
candidate alone does not complete tickets #8/#9, authorize publication, or
establish SLSA Build L2. Existing v1 schemas and historical receipts are preserved.

References: [Apple static code validation](https://developer.apple.com/documentation/security/secstaticcodecheckvalidity(_:_:_:)),
[Apple notarization requirements](https://developer.apple.com/documentation/security/resolving-common-notarization-issues),
[Apple ticket lookup implementation](https://github.com/apple-oss-distributions/Security/blob/main/OSX/libsecurity_codesigning/lib/notarization.cpp),
[pinned Rust Security.framework binding](https://docs.rs/security-framework/3.7.0/security_framework/).
