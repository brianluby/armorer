# Complete CycloneDX document validation

`verification::cyclonedx::OfflineSbomValidator` validates the complete CycloneDX
1.5 JSON document with the already selected CycloneDX CLI **0.33.1**, source
commit `b3cfa4b0edc356dad07e0b6e7ab6da0a94af0246`. Its official native binary must
match both an independently approved byte identity and the compiled distribution
pin before execution. A different caller-approved tool cannot override those
pins. Linux x86_64, Linux ARM64 and macOS ARM64 have explicit identities; all
other platforms fail closed. The three publisher asset digests/sizes were
independently matched against downloaded executable bytes. Those download
receipts alone do not establish execution on another platform.

This complements [selected Cargo graph reconciliation](cargo-graph-v2.md).
Invalid license, date/time and reference fields can leave component/node/edge
relationships consistent while violating the complete document schema. Both
checks are necessary. Schema validity does not authenticate producer claims or
prove the declared dependencies are complete.

## Independent approval and invocation

Supply the validator path and its approved `ByteIdentity` from an independently
reviewed policy/catalog channel. `open` copies the exact bounded regular native
binary to private read-only storage after matching the compiled pin. It never
executes an unqualified path, shell, consuming binary or repository helper.

`validate` requires an expected SBOM byte identity, rehashes a bounded regular
file into a private read-only snapshot, and parses duplicate-free JSON under the
shared node/depth limits. In a complete release consumer that identity must come
from an authenticated inventory, after its source/workflow/run context is checked.
Do not derive authenticity from a producer's unverified hash or this schema result.

The only native command is `validate --input-file <private snapshot>
--input-format json --input-version v1_5 --fail-on-errors`. Explicit format and
version prevent schema autodetection or compatibility fallback. The pinned source
[selects the requested version and rejects validation errors](https://github.com/CycloneDX/cyclonedx-cli/blob/b3cfa4b0edc356dad07e0b6e7ab6da0a94af0246/src/cyclonedx/Commands/ValidateCommand.cs).
Its CycloneDX.Core dependency is 12.1.2 at
`66d9850b39b32d9b0b42d5af2392af016d08572e`;
[the loader reads embedded BOM/SPDX/signature schemas](https://github.com/CycloneDX/cyclonedx-dotnet-library/blob/66d9850b39b32d9b0b42d5af2392af016d08572e/src/CycloneDX.Core/Json/Validator.cs)
with format validation enabled. Downloaded SBOM `$schema`, URLs and external
references do not select executable code or a schema-fetch command.

The child gets an empty environment with an isolated HOME and document temporary
directory, plus a .NET bundle-extraction directory private to the validator
workspace. Repeated validations reuse that qualified tool runtime cache while
each document request remains separate. The 30-second process limit is unchanged.
The child receives null stdin and no inherited credentials,
PATH, user configuration or startup hooks. Extraction of the qualified tool's own
embedded .NET runtime is permitted inside that private directory; consuming
artifacts are never extracted or executed. The document is capped at 17 MiB,
262,144 JSON value nodes and 64 nesting levels. Each native invocation is capped
at 30 seconds and 1 MiB per output stream. Failure returns a static stage error
without exposing raw invalid document content or child stderr. The successful
result and both snapshots are checked again before sealing `ValidatedSbom`.

Inputs require a stable filesystem. As with the signature adapter, a malicious
process with the same OS account is outside the private-snapshot isolation model.
The supported native distributions need their platform system libraries; missing
native runtime support fails rather than selecting a weaker validator.

## Validation and remaining acceptance

Run the qualified native gate explicitly:

```sh
python3 -I scripts/qualify-test-sbom-validator.py --output-directory /tmp/armorer-fixture-cyclonedx
ARMORER_TEST_CDX=/tmp/armorer-fixture-cyclonedx/cyclonedx cargo test --locked --test real_cyclonedx -- --ignored --test-threads=1
```

The directory must permit a new `cyclonedx` leaf; a conflicting file is never
overwritten. Ordinary tests leave this integration ignored. Hosted development
CI qualifies the native tool and runs the explicit gate on every supported native
platform; a missing or mismatched tool fails that step. Tests validate all three
shared graph/SBOM examples and reject malformed non-graph fields, wrong bytes,
versions, duplicate/trailing JSON and symlink inputs. Independent Rust tests also
reject a caller-selected shell or wrong executable bytes before execution.

`ValidatedSbom` has a private constructor and cannot be deserialized from JSON.
It records exact bytes, the complete parsed document and the native validator pin;
it is a schema proof, not a Sigstore or complete-release proof. Project 16 #8 still
requires inventory-first authentication, all asset/bundle verification, comparison
of the verified CycloneDX predicate with this exact published document and the
required selected graph, the trusted final-byte producer, genuine signed Armorer
rehearsal, private-backend positive qualification and explicit historical routing.
Human acceptance/merge and Build L2 acceptance remain separate gates.
