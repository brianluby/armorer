"""Independent structural checks; runtime tests establish exact plan semantics."""
import copy
import json
from pathlib import Path
import tomllib
from jsonschema import Draft202012Validator, ValidationError

root = Path(__file__).resolve().parent.parent
plan_schema = json.loads((root / "schemas/bootstrap-plan-v2.json").read_text())
policy_schema = json.loads((root / "schemas/ci-policy-v1.json").read_text())
Draft202012Validator.check_schema(plan_schema)
Draft202012Validator.check_schema(policy_schema)
plan = Draft202012Validator(plan_schema)
policy = Draft202012Validator(policy_schema)
positive = 0
for profile in ["library", "cli", "service"]:
    example = root / "examples/bootstrap-v2" / profile
    plan.validate(json.loads((example / "bootstrap-plan.json").read_text()))
    policy.validate(tomllib.loads((example / "ci-policy.reviewed.toml").read_text()))
    positive += 2
negative = 0
for validator, value, mutate in [
    (plan, json.loads((root / "examples/bootstrap-v2/library/bootstrap-plan.json").read_text()), lambda x: x.update(shell="caller code")),
    (plan, json.loads((root / "examples/bootstrap-v2/library/bootstrap-plan.json").read_text()), lambda x: x["changes"][0].update(owned="true")),
    (policy, tomllib.loads((root / "examples/bootstrap-v2/library/ci-policy.reviewed.toml").read_text()), lambda x: x["bans"].update(multiple_versions="allow")),
    (policy, tomllib.loads((root / "examples/bootstrap-v2/library/ci-policy.reviewed.toml").read_text()), lambda x: x["advisories"].update(shell="caller code")),
]:
    value = copy.deepcopy(value)
    mutate(value)
    try:
        validator.validate(value)
    except ValidationError:
        negative += 1
    else:
        raise AssertionError("invalid bootstrap structural example accepted")
print(f"2 bootstrap/policy schemas; {positive} positives; {negative} structural rejections")
