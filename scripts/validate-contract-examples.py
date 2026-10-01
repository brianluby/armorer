"""Independent structural schema checks; no signature or trust verification."""
import copy
import json
from pathlib import Path
import tomllib
from jsonschema import Draft202012Validator, ValidationError

root = Path(__file__).resolve().parent.parent
validators = {}
for path in sorted((root / 'schemas').glob('*-v1.json')):
    schema = json.loads(path.read_text())
    Draft202012Validator.check_schema(schema)
    validators[path.name.removesuffix('-v1.json')] = Draft202012Validator(schema)
count = 0
for directory in ['linux-cli', 'library', 'workspace-service', 'macos-final']:
    example = root / 'examples/trust-v1' / directory
    for path in sorted(example.glob('*.json')):
        validators[path.stem].validate(json.loads(path.read_text()))
        count += 1
    for name, kind in [('armorer.toml', 'config'), ('armorer.lock', 'lock')]:
        validators[kind].validate(tomllib.loads((example / name).read_text()))
        count += 1
validators['verification-policy'].validate(json.loads((root / 'examples/trust-v1/historical-verification-policy.json').read_text()))
count += 1
validators['config'].validate(tomllib.loads((root / 'examples/trust-v1/workspace/armorer.toml').read_text()))
count += 1

base = root / 'examples/trust-v1/linux-cli'
negative = 0
for kind, mutate in [
    ('verification-policy', lambda x: x['signers'][0].update(predicate='https://attacker.invalid/custom')),
    ('release-inventory', lambda x: x['assets'][0]['bytes'].update(sha256='wrong')),
    ('release-inventory', lambda x: x['assets'][0]['bytes'].update(size=0)),
    ('artifact-evidence', lambda x: x.update(command='caller code')),
    ('capability-config', lambda x: x.update(schema_version=2)),
    ('catalog', lambda x: x['adapters'][0].update(url='https://attacker.invalid/tool')),
    ('registry-receipt', lambda x: x.update(registry='caller-registry')),
]:
    value = copy.deepcopy(json.loads((base / (kind + '.json')).read_text()))
    mutate(value)
    try:
        validators[kind].validate(value)
    except ValidationError:
        negative += 1
    else:
        raise AssertionError('invalid structural example accepted: ' + kind)
print(f'{len(validators)} schemas checked; {count} positive examples; {negative} structural rejections')
