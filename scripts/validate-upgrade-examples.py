"""Independent structural validation; not catalog/signature verification."""
from schema_checks import ROOT, coverage, document, positive, rejection, validator

checks = {name: validator(name + "-v1.json") for name in ["upgrade-plan", "upgrade-rollback"]}
count = 0
for profile in ["library", "cli", "service"]:
    directory = ROOT / "examples/upgrades" / profile
    for kind, filename in [("upgrade-plan", "upgrade-plan.json"), ("upgrade-rollback", "rollback-plan.json")]:
        positive(checks[kind], document(directory / filename), profile + " " + kind)
        count += 1
base = ROOT / "examples/upgrades/cli"
cases = [
    ("upgrade-plan", "upgrade-plan.json", "caller shell", lambda x: x.update(shell="caller code"), "additionalProperties", []),
    ("upgrade-plan", "upgrade-plan.json", "caller catalog URL", lambda x: x["target_catalog"].update(url="caller root"), "additionalProperties", ["target_catalog"]),
    ("upgrade-plan", "upgrade-plan.json", "unsupported decision", lambda x: x["changes"][0].update(decision="force"), "enum", ["changes", 0, "decision"]),
    ("upgrade-rollback", "rollback-plan.json", "unreviewed change", lambda x: x["changes"][0].update(unreviewed=True), "additionalProperties", ["changes", 0]),
    ("upgrade-rollback", "rollback-plan.json", "workspace shell", lambda x: x["original_upgrade"]["workspace"].update(shell="caller code"), "additionalProperties", ["original_upgrade", "workspace"]),
    ("upgrade-rollback", "rollback-plan.json", "downgrade type", lambda x: x.update(allow_downgrade="true"), "type", ["allow_downgrade"]),
]
for kind, filename, label, mutate, keyword, path in cases:
    rejection(checks[kind], document(base / filename), label, mutate, keyword, path)
coverage("upgrade", count, len(cases), 6, 6)
print(f"2 upgrade schemas; {count} positives; {len(cases)} structural rejections")
