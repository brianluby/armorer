"""Independently check context JSON structure; never approve policy or execute consuming code."""
import copy
import json
from pathlib import Path
from jsonschema import Draft202012Validator, ValidationError

root = Path(__file__).resolve().parent.parent
schema = json.loads((root / 'schemas/verification-context-v1.json').read_text())
Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema)
context = json.loads((root / 'examples/verification-context-v1/synthetic-linux.json').read_text())
validator.validate(context)
negative = 0
for mutate in [
    lambda value: value.update(schema_version=2),
    lambda value: value.update(trigger='pull_request'),
    lambda value: value.update(command='caller shell input'),
    lambda value: value['inputs']['run'].update(attempt=0),
    lambda value: value['native_sbom_validator'].update(sha256='moving-ref'),
    lambda value: value['selections'][0].update(download_url='https://attacker.invalid/context'),
]:
    value = copy.deepcopy(context)
    mutate(value)
    try:
        validator.validate(value)
    except ValidationError:
        negative += 1
    else:
        raise AssertionError('invalid context structure accepted')
print(f'1 synthetic context schema example; {negative} structural rejections; no trust or signature claims')
