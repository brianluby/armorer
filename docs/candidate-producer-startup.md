# Seal the process before bringing the keys

This is a retained implementation walkthrough at the immutable source below.
The source/workflow integrations were subsequently merged. Use the
[current source map](development-status.md) and [acceptance ledger](v01-acceptance.md)
for command availability and remaining gates. The candidate label identifies
this guide's original qualification boundary.

Candidate guide: workflow source
[`4cdf5d786aebb637c7b41afe53c4d19a8be08ffa`](https://github.com/brianluby/armorer-workflows/tree/4cdf5d786aebb637c7b41afe53c4d19a8be08ffa).
Main still embeds workflow `772ca83e386c883c88cc3b936d69f8cb3216c91e`.
This experimental producer identity boundary is not wired into a protected
release job. Its output grants no signing or publication authority.

A startup hook runs before a JavaScript guard. Close that path first.
**This is the way.**

## Start through the fixed boundary

The old `producer_oidc_v1.mjs` and `mapped_producer_v1.mjs` authentication exports
now reject with `oidc-startup-boundary-required` and
`producer-context-startup-boundary-required`. Their record exports reject any
proof. Deleting `NODE_OPTIONS` inside a running process cannot undo a preload
that has already read a credential or changed HTTP behavior.

The new `.github/actions/producer-launcher-v1` composite action invokes absolute
isolated Python: `/usr/bin/python3 -I` on the supported Linux runners and
`/opt/homebrew/bin/python3 -I` on macOS ARM64. Trusted step configuration overrides
the declared startup variables before Python starts. The entire privileged job
must have independently enforced startup boundaries before any earlier action or
executable receives credentials. Python remains a hosted-image dependency;
this does not establish interpreter reproducibility or isolation from hostile
processes sharing the account.

Internal `producer_oidc_worker_v1.mjs` and `mapped_producer_worker_v1.mjs` modules
are called only by the fixed fresh worker. Importing them into caller-controlled
credentialed Node is unsupported. The later signing controller still needs an
accepted integration; there is no supported Node-first fallback.

## Bring independently approved intent and exact bytes

| Action input | Required authority |
| --- | --- |
| `node` | Prepared executable matching the exact compiled native size and SHA-256 |
| `intent` | Independently reviewed bounded JSON file containing the fixed request envelope and complete issuer or mapped intent |
| `approved-intent-sha256` | Independent approval of that exact file; deriving authority from offered token/artifact contents is invalid |

The envelope has exactly `schema_version: 1`, `operation: "oidc"` or `"mapped"`,
and an `intent` object. The full field contracts live in the candidate's
[OIDC guide](https://github.com/brianluby/armorer-workflows/blob/4cdf5d786aebb637c7b41afe53c4d19a8be08ffa/docs/producer-oidc-context-v1.md)
and [mapped guide](https://github.com/brianluby/armorer-workflows/blob/4cdf5d786aebb637c7b41afe53c4d19a8be08ffa/docs/mapped-producer-context-v1.md).
An empty intent is invalid. The launcher checks the approved digest and envelope
before copying Node or reading credentials. Internal workers validate the complete
intent before their provider reads; the envelope check is not that full validation.
Caller inputs expose no command, custom script, token value, keyset or provider URL.

Prepare the fixed public Node **24.21.0** in a credential-free stage using
`prepare_node(destination)` from `armorer_runtime/producer_launcher_v1.py`.
Preparation accepts one fixed official archive, checks compiled size/hash and
writes only its checked `bin/node` leaf. Other archive entries are not extracted
or executed. The privileged launcher installs no tools: it checks a no-follow
native leaf, copies it into private scratch, then rehashes the copy before
credential access. The source's
[platform pin table](https://github.com/brianluby/armorer-workflows/blob/4cdf5d786aebb637c7b41afe53c4d19a8be08ffa/docs/producer-startup-v1.md)
contains the three proposed delivery identities. Public checksum agreement does
not authenticate an upstream release signature or accept a production catalog.

Check the input. Check the copy. **This is the way.**

## Pass only the child's declared credentials

The fresh Node child receives fixed `PATH` and locale, private `HOME`/`TMPDIR`,
`ACTIONS_ID_TOKEN_REQUEST_URL` and `ACTIONS_ID_TOKEN_REQUEST_TOKEN`.
A mapped operation also receives `ARMORER_WORKFLOW_READ_TOKEN`; an issuer-only
operation does not. Values stay in the bounded child environment. They do not
appear in arguments, request JSON or audit output.

The constructed environment excludes Node startup overrides, extra CAs, TLS
bypass, proxies, Python/loader inputs, Apple material and ambient GH credentials.
The mapped worker's separate native reader keeps its own exact read-token
boundary. The artifact-writer observer's runtime credentials are a separate
[interface](credential-boundaries.md#match-the-candidate-helpers-exact-read-interface).

| Launcher bound | Limit |
| --- | ---: |
| Approved request and child stdin | 2 MiB |
| Child stdout | 4 MiB |
| Child stderr | 1 MiB; discarded from returned errors |
| Worker deadline | 600 seconds |
| Cooperative cleanup wait | Up to 90 additional seconds before forced group termination |

The mapped worker retains its own 240-second native deadline and 75-second
emergency grace. Cancellation stops fresh work and gives it time to reap its
separately owned native children. The outer launcher kills remaining members of
its group even after the leader exits. These controls do not contain a hostile
process that escapes its group or sharing-account adversaries.

## Keep an observation separate from a permit

The launcher returns a fixed audit envelope with the operation and public
observation, `startup_environment_isolated: true`, and all three flags
`production_node_catalog_accepted`, `signing_authorized` and
`publication_authorized` set to **false**. The internal original proof is subject
to its [issuer/mapped lifetime](candidate-producer-evidence.md#use-original-identity-proofs-within-their-lifetimes)
and dies with the fresh worker. Copied audit JSON cannot recreate it or authorize
a later protected step. Complete controller/private-proof integration is still
required.

The [dated review](../reviews/2026-10-03-workflow-producer-startup.md) separates
local native startup controls, the earlier hosted skip and the fixture-wiring
successor. Fixtures use synthetic credentials and an ephemeral RSA issuer.
Accepted Node catalog/signatures, live protected OIDC, effective approval,
complete own signed inventory, Apple finalization and publication/recovery remain
separate gates. No SLSA level follows from this identity observation.

Keep the keys inside the boundary. Earn the remaining proof. **This is the way.**
