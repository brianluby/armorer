#!/usr/bin/env python3
"""Regenerate the committed example packets that embed rendered catalog bytes.

Run this after any change under `catalogs/templates/` (and the matching golden
digests in `src/catalog.rs`). Usage, with a freshly built debug binary:

    python3 scripts/regenerate-example-packets.py <repository root> <armorer binary>

What it rewrites, in place:

* `examples/bootstrap-v2/<profile>/bootstrap-plan.json` from `bootstrap plan`.
  Only the generated caller workflow text, its digests and diffs, and the plan
  digest may differ from the committed packet; anything else is an error.
* `examples/bootstrap-catalog/.github/workflows/*.yml`, the literal rendered
  callers, taken from the freshly generated plans.
* `examples/upgrades/<profile>/{upgrade-plan,rollback-plan}.json`. Each scenario is
  reconstructed from its EXISTING packet (pre-upgrade bytes from `before_content`,
  ownership state from `state_preimages`, the catalog id, `--import-file` and
  downgrade flags the packet records) inside a temporary copy of the matching
  bootstrap example outside the repository. Old generated caller bytes in that
  reconstruction are replaced with the current rendering, and any digest embedded
  in a state preimage is recomputed. The cycle then runs plan, apply, exact replay
  (must report unchanged), rollback-plan and rollback-apply, and asserts that the
  tree returns to its pre-upgrade bytes, that no build ran (a `build.rs` trap is
  present in a verification pass) and that the scenario shape matches the old
  packet: same source format, direction, import files, rollback flag and the same
  decision for every path.

Packet fields that embed the current UTC review day (`policy_review_day`, and
through it the plan digests and the rollback packet's embedded upgrade copy)
legitimately change when the tool runs on a later UTC day. Running the tool twice
on the same day leaves the tree unchanged.

Standard library only, Python 3.11+.
"""

import hashlib
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

PROFILES = ("library", "cli", "service")
CALLERS = (".github/workflows/armorer-build.yml", ".github/workflows/armorer-ci.yml")
CALLER_FIELDS = ("after_sha256", "proposed_content", "unified_diff")
LOCK = ".armorer/apply.lock"
TRAP = 'fn main() { std::fs::write("BUILD_EXECUTED", "bad").unwrap(); panic!("a build ran"); }\n'
OLD_GROUP = re.compile(
    r"^(  group: armorer-(?:ci|build)-\$\{\{ github\.workflow \}\}-)\$\{\{ github\.ref \}\}$",
    re.MULTILINE,
)
NEW_GROUP = r"\1${{ github.event_name == 'pull_request' && github.ref || github.sha }}"
# Digests of caller bytes rendered by earlier templates; none may survive in a
# reconstructed state preimage, because the tool would then reject the old digest.
KNOWN_OLD_CALLER_DIGESTS = {
    "30ff9cbd05cd26659cf7f28274f236762ba2d3fc6518b57cfa7a0d47cc12b715",
    "edfb0ff48f128193445b0f35e2022913a2e0adb71007fb59d8d535cdf4f392c5",
    "d54919c9b41be91ffb665eb631a27aeed4ebb482339a64977a968798645f1ed1",
    "2478570952c760df003bd956521b126d936bfc28098797082b2ad30767c005e8",
}


def fail(message):
    raise SystemExit(f"error: {message}")


def sha256(text):
    return hashlib.sha256(text.encode()).hexdigest()


def current_caller_text(text):
    """Replace the previous concurrency group line with the current one."""
    return OLD_GROUP.sub(NEW_GROUP, text)


def refresh_state(node):
    """Rewrite generated caller text embedded in a state preimage, recomputing digests."""
    if isinstance(node, dict):
        content = node.get("content")
        if isinstance(content, str) and node.get("sha256") == sha256(content):
            fresh = current_caller_text(content)
            if fresh != content:
                node["content"] = fresh
                node["sha256"] = sha256(fresh)
        for value in node.values():
            refresh_state(value)
    elif isinstance(node, list):
        for value in node:
            refresh_state(value)


class Tool:
    def __init__(self, binary):
        self.binary = str(binary)

    def run(self, repo, *args):
        done = subprocess.run(
            [self.binary, "--repository", str(repo), *args], capture_output=True
        )
        if done.returncode != 0:
            fail(f"{' '.join(args)} exited {done.returncode}: {done.stderr.decode()}{done.stdout.decode()}")
        return done.stdout


def tree(root):
    return {
        str(p.relative_to(root)): p.read_bytes()
        for p in sorted(root.rglob("*"))
        if p.is_file() and str(p.relative_to(root)) != LOCK
    }


def masked_bootstrap(plan):
    plan = json.loads(json.dumps(plan))
    plan.pop("plan_sha256", None)
    for change in plan["changes"]:
        if change["path"] in CALLERS:
            for field in CALLER_FIELDS:
                change.pop(field, None)
    return plan


def regenerate_bootstrap_plans(root, tool):
    callers = {}
    for profile in PROFILES:
        directory = root / "examples/bootstrap-v2" / profile
        target = directory / "bootstrap-plan.json"
        old = json.loads(target.read_text())
        fresh_bytes = tool.run(
            directory, "bootstrap", "plan", "--policy", str(directory / "ci-policy.reviewed.toml")
        )
        fresh = json.loads(fresh_bytes)
        if masked_bootstrap(old) != masked_bootstrap(fresh):
            fail(f"{profile}: bootstrap plan differs beyond caller text, digests, diffs and plan digest")
        for change in fresh["changes"]:
            if change["path"] in CALLERS:
                callers.setdefault(change["path"], set()).add(change["proposed_content"])
        target.write_bytes(fresh_bytes)
        print(f"bootstrap-v2/{profile}: plan {fresh['plan_sha256']}")
    for path, texts in callers.items():
        if len(texts) != 1:
            fail(f"{path}: profiles render different caller bytes")
        literal = root / "examples/bootstrap-catalog" / path
        literal.write_text(next(iter(texts)), newline="")
        print(f"bootstrap-catalog/{path}: rewritten")


def upgrade_cycle(root, tool, profile, trap):
    """Run one full plan/apply/replay/rollback cycle; return the new packet bytes."""
    directory = root / "examples/upgrades" / profile
    old_up = json.loads((directory / "upgrade-plan.json").read_text())
    old_rb = json.loads((directory / "rollback-plan.json").read_text())
    with tempfile.TemporaryDirectory(prefix=f"armorer-regen-{profile}-") as scratch:
        scratch = pathlib.Path(scratch)
        repo = scratch / "project"
        shutil.copytree(root / "examples/bootstrap-v2" / profile, repo)
        if trap:
            (repo / "build.rs").write_text(TRAP)
        policy = repo / "ci-policy.reviewed.toml"
        if policy.read_text() != old_up["policy_content"]:
            fail(f"{profile}: reviewed policy differs from the packet's policy_content")
        for change in old_up["changes"]:
            before = change["before_content"]
            if before is not None:
                path = repo / change["path"]
                path.parent.mkdir(parents=True, exist_ok=True)
                text = current_caller_text(before) if change["path"] in CALLERS else before
                path.write_text(text, newline="")
        for relative, content in old_up["state_preimages"].items():
            if content is None:
                continue
            state = json.loads(content)
            refresh_state(state)
            rendered = json.dumps(state, indent=2) + ("\n" if content.endswith("\n") else "")
            leftovers = KNOWN_OLD_CALLER_DIGESTS & set(re.findall(r"[0-9a-f]{64}", rendered))
            if leftovers:
                fail(f"{profile}: state preimage {relative} still embeds an old caller digest")
            path = repo / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(rendered, newline="")
        before_tree = tree(repo)

        plan_args = ["upgrade", "plan", "--policy", str(policy), "--catalog", old_up["target_catalog"]["id"]]
        for imported in old_up["import_files"]:
            plan_args += ["--import-file", imported]
        if old_up["allow_downgrade"]:
            plan_args.append("--allow-downgrade")
        plan_bytes = tool.run(repo, *plan_args)
        plan = json.loads(plan_bytes)
        if tree(repo) != before_tree:
            fail(f"{profile}: upgrade plan changed consuming bytes")
        for key in ("source_format", "direction", "import_files", "allow_downgrade", "config_sha256", "source_catalog"):
            if plan[key] != old_up[key]:
                fail(f"{profile}: upgrade {key} changed: {old_up[key]!r} -> {plan[key]!r}")
        decisions = [(c["path"], c["decision"], c["imported"]) for c in plan["changes"]]
        if decisions != [(c["path"], c["decision"], c["imported"]) for c in old_up["changes"]]:
            fail(f"{profile}: per-path upgrade decisions changed")
        expected_inputs = dict(old_up["workspace"]["inputs"])
        if trap:
            expected_inputs["build.rs"] = "target-path-present"
        if plan["workspace"]["inputs"] != expected_inputs:
            fail(f"{profile}: workspace inputs changed")

        saved = scratch / "upgrade-plan.json"
        saved.write_bytes(plan_bytes)
        digest = plan["plan_sha256"]
        tool.run(repo, "upgrade", "apply", "--plan", str(saved), "--expect-plan-sha256", digest)
        replay = tool.run(repo, "upgrade", "apply", "--plan", str(saved), "--expect-plan-sha256", digest)
        if b"unchanged" not in replay:
            fail(f"{profile}: exact replay did not report unchanged: {replay.decode()[:300]}")

        rollback_args = [
            "upgrade", "rollback-plan", "--upgrade-plan", str(saved),
            "--expect-upgrade-plan-sha256", digest,
        ]
        if old_rb["allow_downgrade"]:
            rollback_args.append("--allow-downgrade")
        rollback_bytes = tool.run(repo, *rollback_args)
        rollback = json.loads(rollback_bytes)
        for key in ("allow_downgrade", "contract"):
            if rollback[key] != old_rb[key]:
                fail(f"{profile}: rollback {key} changed")
        rollback_shape = [(c["path"], c["after_sha256"] is None, c["before_content"] is None) for c in rollback["changes"]]
        if rollback_shape != [(c["path"], c["after_sha256"] is None, c["before_content"] is None) for c in old_rb["changes"]]:
            fail(f"{profile}: rollback operation inventory changed")
        saved_rollback = scratch / "rollback-plan.json"
        saved_rollback.write_bytes(rollback_bytes)
        tool.run(repo, "upgrade", "rollback-apply", "--plan", str(saved_rollback),
                 "--expect-plan-sha256", rollback["plan_sha256"])
        after_tree = tree(repo)
        if after_tree != before_tree:
            changed = sorted(k for k in set(after_tree) | set(before_tree) if after_tree.get(k) != before_tree.get(k))
            fail(f"{profile}: rollback did not restore the pre-upgrade bytes: {changed}")
        if (repo / "BUILD_EXECUTED").exists() or (repo / "target").exists():
            fail(f"{profile}: a build ran")
        return plan_bytes, rollback_bytes, plan, old_up


def regenerate_upgrade_packets(root, tool):
    for profile in PROFILES:
        upgrade_cycle(root, tool, profile, trap=True)  # verification pass with the execution trap
        plan_bytes, rollback_bytes, plan, old = upgrade_cycle(root, tool, profile, trap=False)
        directory = root / "examples/upgrades" / profile
        (directory / "upgrade-plan.json").write_bytes(plan_bytes)
        (directory / "rollback-plan.json").write_bytes(rollback_bytes)
        print(f"upgrades/{profile}: {old['source_format']} {old['direction']} plan {plan['plan_sha256']}")


def main():
    if len(sys.argv) != 3:
        fail("usage: regenerate-example-packets.py <repository root> <armorer binary>")
    root = pathlib.Path(sys.argv[1]).resolve()
    binary = pathlib.Path(sys.argv[2]).resolve()
    if not binary.is_file():
        fail(f"{binary} is not a file; build the debug binary first")
    for profile in PROFILES:
        if not (root / "examples/bootstrap-v2" / profile).is_dir():
            fail(f"{root} does not look like the armorer repository")
    tool = Tool(binary)
    regenerate_bootstrap_plans(root, tool)
    regenerate_upgrade_packets(root, tool)


if __name__ == "__main__":
    main()
