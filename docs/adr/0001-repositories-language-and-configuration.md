# ADR 0001: repositories, language and configuration

Status: accepted design, 2026-09-30. Implementation is staged; current contracts remain experimental before v0.1.

Context: adopters need stable trusted workflows and safe local migrations without maintaining a large copied release pipeline. The two components evolve at different rates and have different trust boundaries.

Decision: use brianluby/armorer for a Rust CLI/shared runtime and brianluby/armorer-workflows for separately versioned SHA-pinned reusable workflows. Use armorer.toml intent, armorer.lock resolutions, tracked generated bases and independently reviewed consumer policy. Require tested compatibility across versions.

Alternatives: a monorepo simplifies atomic changes but ties workflow trust/versioning to CLI delivery; copying templates avoids cross-repository calls but multiplies drift; Python/Go are viable CLI languages but Rust best matches distribution and domain requirements. Shell lacks robust migration/input validation.

Consequences: two release/version streams and a compatibility catalog must be maintained. Rust compile/distribution and bootstrap trust need explicit handling. Adopters receive small caller workflows and reviewable catalog updates.
