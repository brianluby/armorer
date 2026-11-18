# ADR 0015: optional managed repository files

Status: implemented proposal for review (review item F2), 2026-11-18. Human
acceptance and hosted validation are separate gates; no release claim.

A repository that is secure from the start also needs a security policy, code
owners and dependency-update configuration. Armorer managed only its five CI and
build files. Adding `SECURITY.md`, `.github/CODEOWNERS` and
`.github/dependabot.yml` unconditionally would change every existing plan, and an
adopter with a hand-written `SECURITY.md` would hit a conflict and could not
bootstrap at all.

Each file is opt-in through a new optional `[repository_files]` table in
`armorer.toml`: `security_policy`, `dependabot` and a `code_owners` list. Every
switch is off by default, unknown fields are rejected, and the table is omitted
from every serialization at its defaults, so every existing configuration, plan,
state, journal, lock, receipt and committed example is byte-for-byte unchanged.
Owners are exactly `@user` or `@org/team`, at most 16, unique; errors never echo
a value. The security policy and Dependabot files are fixed templates with the
repository or workflow repository substituted; code owners are rendered in code
from the validated list.

Catalog entries are now unconditional or optional, in the one ADR 0013 table.
That table yields two sets: the maximal set, every path a catalog can render, and
the selected set for a configuration. A bootstrap plan lists exactly the selected
set for its intent; upgrade scope is the target's selected set plus owned paths
outside it. Documents that carry no configuration, bootstrap ownership and
journal count bounds, are bounded by the maximal set, and the license guard uses
the union of maximal sets. Upgrade ownership records the repository name and the
switches beside its configuration digest, only when a switch is on, so optional
bases are re-derived exactly. Bootstrap ownership records neither: its optional
bases are bounded and self-hashed like the policy file, and an upgrade accepts
them only in their rendered shape. ADR 0012 lets the pre-release `bootstrap-v2`
entry gain these entries in place, because no existing configuration renders them.

Turning a switch off after Armorer owns the file has no approved behavior yet.
It fails closed: bootstrap planning refuses with an explicit transaction error,
and an upgrade shows a blocking conflict. Nothing is deleted and ownership is never
silently released. Releasing, deleting or an explicit per-file decision is a
separate owner decision.

Consequences: an existing hand-written file at one of these paths conflicts until
the owner removes it or leaves the switch off; `armorer init` switches a file on
only where none exists. These files are not importable, admit no customization,
and `dependabot.yml` content is fixed and cannot carry extra ecosystems.
Code-owner review takes effect only when a ruleset requires it. See
[bootstrap](../bootstrap-v2.md#optional-repository-files) and
`tests/repository_files.rs`.
