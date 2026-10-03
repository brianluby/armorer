# Stage the whole schema set

Snapshot: 2026-10-03 America/Los_Angeles / UTC.
[Draft #23](https://github.com/brianluby/armorer/pull/23) is
`715a6ece2e90c826b2a647419177515413f2c7bb`, based on #21 at
`56d7380a5534583b7973b5eedc55a1223b46b60e`. It targets
`codex/v01-source-integration`; main remains
`96457ee418439dde097339cfcd374c08f2cc98ad`.

Inspect the command. Keep the old schemas on failure. **This is the way.**

## Repair and direct control

The [earlier contributor finding](2026-10-02-pr21-resolved-features.md#confirmed-contributor-guide-finding)
remains valid at its recorded head and at #21's current head. The literal `+`
becomes its own loop entry, and `armorer schema +` fails with exit 2 before the
final replacement loop. #23 removes that character, corrects the output count
from 23 to 24, and adds the requested creed. Only `CONTRIBUTING.md` changes.
No new actionable defect was found in that seven-line documentation diff.
This is the author's validation record, not independent reviewer approval.

Both literal shell procedures were extracted from the contributor page and run
in the isolated candidate checkout with Rust 1.95.0 and offline dependencies.
The documented `target/debug/armorer` path resolved through an owned temporary
`target` symlink to the cache containing the binary built from that checkout.
The schema command retained the documented fixed `--target-dir target` argument.
The temporary symlink was removed after validation.

| Procedure | Exit | Committed schemas after execution | Staging |
| --- | ---: | --- | --- |
| Original #21 instructions | 2, invalid schema kind `+` | All 24 unchanged by SHA-256 comparison | Removed by trap |
| Corrected #23 instructions | 0 | All 24 exports byte-identical to committed files | Removed by trap |

The fixed procedure actually runs every exporter. A separate CI schema loop alone
would not establish that this contributor command works. These controls establish
generation and cleanup; they do not establish an atomic multi-file replacement
if a later `mv` fails.

## Qualification and integration boundary

Formatting, strict all-target/all-feature Clippy, the locked build and all
**221 local tests** pass. Seven native tests remain explicitly ignored locally.
The first sandboxed attempt could not bind a temporary Unix socket (`EPERM`);
the complete suite passed after that sandbox restriction was lifted.

[Development run 37110189255](https://github.com/brianluby/armorer/actions/runs/37110189255)
passes Linux x64, Linux ARM64 and macOS 15 at the exact #23 head. The configured
native signature controls, committed-schema comparisons, independent example
validation and workflow audit pass. Native Apple checks apply to the macOS job.

The verified fix is proposed in a separate draft. #21 still contains the original
typo until an authorized integration incorporates it. Main's contributor guide
correctly lists its own 14 schemas and does not gain candidate commands from this
repair. This receipt grants no merge, protected signing, complete own-release
rehearsal or publication acceptance.

Keep the fix with its source. **This is the way.**
