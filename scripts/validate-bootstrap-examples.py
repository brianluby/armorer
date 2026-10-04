"""Structural checks; runtime tests separately establish exact plan semantics."""
from schema_checks import ROOT, coverage, document, positive, rejection, validator

plan = validator("bootstrap-plan-v2.json")
policy = validator("ci-policy-v1.json")
count = 0
for profile in ["library", "cli", "service"]:
    example = ROOT / "examples/bootstrap-v2" / profile
    positive(plan, document(example / "bootstrap-plan.json"), profile + " bootstrap plan")
    positive(policy, document(example / "ci-policy.reviewed.toml"), profile + " reviewed policy")
    count += 2
base = ROOT / "examples/bootstrap-v2/library"
cases = [
    (plan, "bootstrap-plan.json", "caller shell", lambda x: x.update(shell="caller code"), "additionalProperties", []),
    (plan, "bootstrap-plan.json", "owned type", lambda x: x["changes"][0].update(owned="true"), "type", ["changes", 0, "owned"]),
    (policy, "ci-policy.reviewed.toml", "unsupported duplicate policy", lambda x: x["bans"].update(multiple_versions="allow"), "enum", ["bans", "multiple_versions"]),
    (policy, "ci-policy.reviewed.toml", "advisory shell", lambda x: x["advisories"].update(shell="caller code"), "additionalProperties", ["advisories"]),
    (plan, "bootstrap-plan.json", "unknown package field", lambda x: x["workspace"]["packages"][0].update(caller="code"), "additionalProperties", ["workspace", "packages", 0]),
    (plan, "bootstrap-plan.json", "unknown target field", lambda x: x["workspace"]["packages"][0]["targets"][0].update(caller="code"), "additionalProperties", ["workspace", "packages", 0, "targets", 0]),
]
for check, filename, label, mutate, keyword, path in cases:
    rejection(check, document(base / filename), label, mutate, keyword, path)
coverage("bootstrap", count, len(cases), 6, 6)
print(f"2 bootstrap/policy schemas; {count} positives; {len(cases)} structural rejections")
