"""Independent structural checks; no signature or trust verification."""
from schema_checks import ROOT, coverage, document, positive, rejection, validator

checks = {path.name.removesuffix("-v1.json"): validator(path.name) for path in sorted((ROOT / "schemas").glob("*-v1.json"))}
paths = sorted(path for path in (ROOT / "examples/trust-v1").rglob("*") if path.is_file() and (path.suffix == ".json" or path.name in ("armorer.toml", "armorer.lock")))
count = 0
for path in paths:
    kind = {"armorer.toml": "config", "armorer.lock": "lock", "historical-verification-policy.json": "verification-policy"}.get(path.name, path.stem)
    if kind not in checks:
        raise ValueError(f"unmapped trust contract example {path}")
    positive(checks[kind], document(path), str(path))
    count += 1
# Discover every consuming intent example, including dogfood and profile-workspace.
for path in sorted((ROOT / "examples").rglob("armorer.toml")):
    if path not in paths:
        positive(checks["config"], document(path), str(path))
        count += 1
base = ROOT / "examples/trust-v1/linux-cli"
cases = [
    ("verification-policy", "custom predicate", lambda x: x["signers"][0].update(predicate="https://attacker.invalid/custom"), "enum", ["signers", 0, "predicate"]),
    ("release-inventory", "malformed asset digest", lambda x: x["assets"][0]["bytes"].update(sha256="wrong"), "pattern", ["assets", 0, "bytes", "sha256"]),
    ("release-inventory", "empty asset bytes", lambda x: x["assets"][0]["bytes"].update(size=0), "minimum", ["assets", 0, "bytes", "size"]),
    ("artifact-evidence", "caller command", lambda x: x.update(command="caller code"), "additionalProperties", []),
    ("capability-config", "unsupported version", lambda x: x.update(schema_version=2), "maximum", ["schema_version"]),
    ("catalog", "caller adapter URL", lambda x: x["adapters"][0].update(url="https://attacker.invalid/tool"), "additionalProperties", ["adapters", 0]),
    ("registry-receipt", "caller registry", lambda x: x.update(registry="caller-registry"), "enum", ["registry"]),
    ("evidence-requirements", "zero freshness", lambda x: x.update(max_age_seconds=0), "minimum", ["max_age_seconds"]),
    ("artifact-evidence", "zero tool freshness", lambda x: x["tools"][0].update(max_age_seconds=0), "minimum", ["tools", 0, "max_age_seconds"]),
]
for kind, label, mutate, keyword, location in cases:
    rejection(checks[kind], document(base / (kind + ".json")), label, mutate, keyword, location)
lock = document(base / "armorer.lock")
rejection(checks["lock"], lock, "invalid lock digest", lambda x: x.update(config_sha256="invalid"), "pattern", ["config_sha256"])
rejection(checks["lock"], lock, "invalid workflow commit", lambda x: x["workflows"].update(commit="main"), "pattern", ["workflows", "commit"])
rejection(checks["lock"], lock, "newline terminated digest", lambda x: x.update(config_sha256=x["config_sha256"] + "\n"), "pattern", ["config_sha256"])
rejection(checks["lock"], lock, "newline terminated commit", lambda x: x["workflows"].update(commit=x["workflows"]["commit"] + "\n"), "pattern", ["workflows", "commit"])
intent = document(base / "armorer.toml")
rejection(checks["config"], intent, "newline terminated repository", lambda x: x.update(repository=x["repository"] + "\n"), "pattern", ["repository"])
rejection(checks["config"], intent, "newline terminated identifier", lambda x: x["deliverables"][0].update(id=x["deliverables"][0]["id"] + "\n"), "pattern", ["deliverables", 0, "id"])
rejection(checks["lock"], lock, "overflowed version", lambda x: x.update(schema_version=2**32), "maximum", ["schema_version"])
inventory = document(base / "release-inventory.json")
rejection(checks["release-inventory"], inventory, "overflowed run id", lambda x: x["inputs"]["run"].update(id=2**64), "maximum", ["inputs", "run", "id"])
negative = len(cases) + 8
coverage("contract examples", count, negative, 60, 17)
print(f"{len(checks)} schemas checked; {count} positive examples; {negative} structural rejections")
