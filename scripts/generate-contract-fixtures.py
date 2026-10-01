"""Deterministic synthetic contract fixtures; none are authenticated release evidence."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "examples" / "trust-v1"
NOW = 1790870400


def identity(data):
    if isinstance(data, str):
        data = data.encode()
    return {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}


def write(path, value):
    path.write_bytes((json.dumps(value, indent=2) + "\n").encode("utf-8"))


def review():
    return {"owner": "fixture-reviewer", "rationale": "Synthetic contract example only",
            "reviewed_at": NOW - 100, "expires_at": NOW + 86400,
            "record": identity("synthetic review record")}


def workflow():
    return {"repository": "fixture/workflows", "path": ".github/workflows/release.yml", "commit": "b" * 40}


def build(name, profile, target):
    directory = ROOT / name
    directory.mkdir(parents=True, exist_ok=True)
    binary = "app" if profile != "library" else None
    toml = ('schema_version = 1\nrepository = "fixture/project"\ntoolchain = "1.95.0"\n'
            '[[deliverables]]\nid = "app"\nprofile = "' + profile + '"\npackage = "app"\n'
            + ('binary = "app"\n' if binary else '') + 'targets = ["' + target + '"]\nfeature_set = "minimal"\n'
            '[feature_sets.minimal]\ndefault_features = false\nfeatures = []\n'
            '[policy]\nlicense_file = "LICENSE"\nattestations = "required"\n')
    (directory / 'armorer.toml').write_bytes(toml.encode('utf-8'))
    config_digest = identity(toml)['sha256']
    lock = ('schema_version = 1\nconfig_sha256 = "' + config_digest + '"\nruntime_version = "0.1.0"\n'
            '[workflows]\nrepository = "fixture/workflows"\ncommit = "' + 'b' * 40 + '"\n'
            '[tools.cargo-cyclonedx]\nversion = "0.5.9"\nsha256 = "' + identity('synthetic cyclonedx tool')['sha256'] + '"\n')
    (directory / 'armorer.lock').write_bytes(lock.encode('utf-8'))
    source = {"repository": "fixture/project", "commit": "a" * 40, "git_ref": "refs/tags/v1.0.0"}
    inputs = {"source": source, "config_sha256": config_digest, "lock_sha256": identity(lock)['sha256'],
              "cargo_lock_sha256": identity("synthetic Cargo.lock")['sha256'],
              "runtime": identity("synthetic runtime"), "runtime_version": "0.1.0",
              "run": {"id": 12, "attempt": 1, "workflow": workflow()}}
    provenance = "https://slsa.dev/provenance/v1"
    sbom = "https://cyclonedx.org/bom"
    signers = [{"predicate": sbom if scope == "sbom-predicate" else provenance,
                "workflow": workflow(), "scope": scope} for scope in
               ['final-artifact', 'cargo-sbom-file', 'sbom-predicate', 'evidence-file', 'inventory']]
    policy = {"mode": "release", "schema_version": 1, "repository": "fixture/project", "sources": [source], "signers": signers,
              "roots": {"backend": "sigstore-public-good", "trusted_root": identity("synthetic roots"),
                        "verifier": identity("synthetic verifier"), "verifier_version": "2.102.0", "review": review()},
              "deny_self_hosted_runners": True, "supplemental_assets": {"diagnostic": False},
              "apple_team": "ABCDEFGHIJ" if target == "aarch64-apple-darwin" and binary else None,
              "historical": [], "review": review()}
    key = 'app--' + target + '--minimal'
    final = key + ('.crate' if profile == 'library' else '.tar.gz')
    assets = []
    def asset(name, role, subjects=None, predicate=None, predicate_asset=None):
        assets.append({"name": name, "role": role, "bytes": identity('fixture:' + name),
                       "subjects": subjects or [], "predicate": predicate, "predicate_asset": predicate_asset})
    asset(final, 'distributable')
    records = [('cargo-sbom', '.cdx.json'), ('cargo-graph', '.cargo-graph.json'),
               ('build-evidence', '.build.json'), ('package-evidence', '.package.json')]
    mac = target == 'aarch64-apple-darwin' and binary
    if mac:
        records.append(('transformation-evidence', '.apple.json'))
    for role, suffix in records:
        n = key + suffix
        asset(n, role, [final])
        asset(n + '.provenance.sigstore.json', 'attestation-bundle', [n], provenance)
    asset(final + '.provenance.sigstore.json', 'attestation-bundle', [final], provenance)
    asset(final + '.sbom.sigstore.json', 'attestation-bundle', [final], sbom, key + '.cdx.json')
    inventory = {"schema_version": 1, "inputs": inputs, "assets": assets}
    selection = {"deliverable_id": "app", "profile": profile, "package": "app", "package_version": "1.0.0",
                 "binary": binary, "target": target, "feature_set": "minimal", "default_features": False,
                 "features": [], "toolchain": "1.95.0"}
    tool = {"name": "cargo-cyclonedx", "kind": "tool", "version": "0.5.9",
            "bytes": identity("synthetic cyclonedx tool"), "authentication_record": identity("synthetic tool authentication"),
            "observed_at": NOW - 80, "max_age_seconds": None}
    final_bytes = assets[0]['bytes']
    coverage = {"capability": "cargo-sbom", "scope": "compiled-cargo-target-and-host-build-dependencies",
                "tested_subject": final_bytes, "omissions": ["native/system/downloaded dependencies are not fully inventoried"],
                "outcome": "passed", "enforcement": "enforced", "exception_ids": []}
    kinds = ['build', 'sign', 'notarize', 'package'] if mac else ['build', 'package']
    steps = []
    previous = None
    for i, kind in enumerate(kinds):
        output = final_bytes if kind == 'package' else identity('synthetic ' + ('sign' if kind == 'notarize' else kind))
        steps.append({"kind": kind, "inputs": [previous] if previous else [identity('synthetic source snapshot')],
                      "output": output, "run": inputs['run'], "started_at": NOW - 60 + i * 10,
                      "finished_at": NOW - 55 + i * 10, "outcome": "passed", "platform_evidence": [identity("synthetic platform reference: " + kind)]})
        previous = output
    apple = None
    if mac:
        apple = {"team_id": "ABCDEFGHIJ", "certificate_sha256": "d" * 64, "hardened_runtime": True,
                 "secure_timestamp": True, "notarization_submission_id": "fixture-notary-id", "notarization_outcome": "passed",
                 "notarization_log": identity("synthetic notary log"), "ticket_mode": "online-standalone-mach-o"}
    catalog_identity = identity('synthetic reviewed catalog')
    evidence = {"schema_version": 1, "inputs": inputs, "selection": selection, "catalog": catalog_identity,
                "runner_label": {'x86_64-unknown-linux-gnu': 'ubuntu-24.04', 'aarch64-unknown-linux-gnu': 'ubuntu-24.04-arm',
                                 'aarch64-apple-darwin': 'macos-15'}[target], "runner_image": "synthetic-hosted-image",
                "recorded_at": NOW, "tools": [tool], "coverage": [coverage], "exceptions": [], "steps": steps,
                "apple_assertions": apple}
    requirements = {"schema_version": 1, "inputs": inputs, "selection": selection, "catalog": catalog_identity,
                    "max_age_seconds": 3600, "steps": {kind: workflow() for kind in kinds}, "tools": [tool],
                    "required_coverage": [coverage], "allowed_exceptions": [], "apple_team": policy['apple_team'], "review": review()}
    baseline = ['platform-attestations', 'immutable-releases', 'protected-tags', 'protected-publish-environment',
                'cargo-sbom', 'dependency-policy']
    if mac:
        baseline += ['apple-signing', 'protected-signing-environment']
    capabilities = {"schema_version": 1, "config_sha256": config_digest, "catalog": catalog_identity,
                    "decisions": dict.fromkeys(baseline, 'required'), "review": review()}
    catalog = {"schema_version": 1, "previous_catalog": None, "adapters": [{"id": "cargo-cyclonedx", "capability": "cargo-sbom",
               "config_versions": [1], "runtime_versions": ['0.1.0'], "workflow": workflow(),
               "pins": [{"name": tool['name'], "version": tool['version'], "distribution": tool['bytes'],
                         "authentication_record": tool['authentication_record']}], "review": review()}]}
    catalog_identity = identity(json.dumps(catalog, indent=2) + '\n')
    capabilities['catalog'] = catalog_identity
    evidence['catalog'] = catalog_identity
    requirements['catalog'] = catalog_identity
    observation = {"schema_version": 1, "capability": "immutable-releases", "availability": "available", "enforcement": "enforced",
                   "observed_at": NOW, "evidence": identity('synthetic setting observation'), "limitations": []}
    receipt = {"schema_version": 1, "inputs": inputs, "tag": 'v1.0.0', "release_id": 123,
               "inventory": identity(json.dumps(inventory, indent=2) + '\n'), "policy": identity(json.dumps(policy, indent=2) + '\n'),
               "state": 'owned-draft', "draft_download_receipt": None, "approval": None, "immutable_setting": None,
               "published_at": None, "release_attestation": None, "provenance_verification": None, "conflict": None}
    lifecycle = {"schema_version": 1, "inputs": inputs, "stage": 'configured', "policy": receipt['policy'], "recorded_at": NOW,
                 "evidence": [identity('synthetic configuration receipt')], "limitations": ['No hosted or authenticated evidence']}
    crates = [{"name": "core", "version": "1.0.0", "archive": identity('synthetic core crate'), "dependencies": []},
              {"name": "app", "version": "1.0.0", "archive": identity('synthetic app crate'), "dependencies": ['core']}]
    publish_set = {"schema_version": 1, "inputs": inputs, "registry": 'crates-io', "crates": crates}
    # Rust struct and JSON map serialization order differ. Compute the publish-set
    # identity over compact typed Rust JSON in the fixture export/test helper.
    registry_receipt = {"schema_version": 1, "publish_set": identity(json.dumps(publish_set, separators=(',', ':'))),
                        "registry": 'crates-io', "recorded_at": NOW,
                        "crates": [{"name": c['name'], "version": c['version'], "state": 'prepared', "registry_bytes": None,
                                    "index_sha256": None, "observation": None, "conflict": None} for c in crates]}
    for filename, obj in [('verification-policy', policy), ('release-inventory', inventory), ('artifact-evidence', evidence),
                          ('evidence-requirements', requirements), ('capability-config', capabilities), ('catalog', catalog),
                          ('capability-observation', observation), ('github-receipt', receipt), ('lifecycle-record', lifecycle),
                          ('publish-set', publish_set), ('registry-receipt', registry_receipt)]:
        write(directory / (filename + '.json'), obj)
    return policy


for fixture, profile, target in [('linux-cli', 'cli', 'x86_64-unknown-linux-gnu'), ('library', 'library', 'x86_64-unknown-linux-gnu'),
                                  ('workspace-service', 'service', 'aarch64-unknown-linux-gnu'), ('macos-final', 'cli', 'aarch64-apple-darwin')]:
    policy = build(fixture, profile, target)
    if fixture == 'linux-cli':
        historical = {"source": {"repository": 'fixture/project', "commit": 'e' * 40, "git_ref": 'refs/tags/v0.1.0'},
                      "assets": {"legacy.tar.gz": identity('synthetic historical artifact')}, "review": review(),
                      "limitations": ['Exact reviewed bytes only; no provenance authentication or SLSA level']}
        policy['historical'] = [historical]
        ROOT.mkdir(parents=True, exist_ok=True)
        write(ROOT / 'historical-verification-policy.json', policy)
