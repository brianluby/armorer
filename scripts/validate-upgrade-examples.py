"""Independent structural validation; not catalog/signature verification."""
import copy
import json
from pathlib import Path
from jsonschema import Draft202012Validator, ValidationError

root = Path(__file__).resolve().parent.parent
validators = {}
for name in ["upgrade-plan", "upgrade-rollback"]:
    schema = json.loads((root / f"schemas/{name}-v1.json").read_text())
    Draft202012Validator.check_schema(schema)
    validators[name] = Draft202012Validator(schema)
positive = 0
for profile in ["library", "cli", "service"]:
    directory = root / "examples/upgrades" / profile
    for name, filename in [("upgrade-plan", "upgrade-plan.json"), ("upgrade-rollback", "rollback-plan.json")]:
        validators[name].validate(json.loads((directory / filename).read_text()))
        positive += 1
base = root / "examples/upgrades/cli"
negative = 0
for kind, filename, mutate in [
    ("upgrade-plan", "upgrade-plan.json", lambda x: x.update(shell="caller code")),
    ("upgrade-plan", "upgrade-plan.json", lambda x: x["target_catalog"].update(url="caller root")),
    ("upgrade-plan", "upgrade-plan.json", lambda x: x["changes"][0].update(decision="force")),
    ("upgrade-rollback", "rollback-plan.json", lambda x: x["changes"][0].update(unreviewed=True)),
    ("upgrade-rollback", "rollback-plan.json", lambda x: x["original_upgrade"]["workspace"].update(shell="caller code")),
    ("upgrade-rollback", "rollback-plan.json", lambda x: x.update(allow_downgrade="true")),
]:
    value = copy.deepcopy(json.loads((base / filename).read_text()))
    mutate(value)
    try:
        validators[kind].validate(value)
    except ValidationError:
        negative += 1
    else:
        raise AssertionError(f"invalid upgrade structural example accepted: {kind}")
print(f"2 upgrade schemas; {positive} positives; {negative} structural rejections")
