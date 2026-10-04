"""Credential-free regressions for the structural gates and inert staging helpers."""
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from schema_checks import ROOT, coverage, rejection, validator


def load_script(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / (name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ReviewGates(unittest.TestCase):
    def test_unrelated_validation_failure_cannot_satisfy_mutation(self):
        check = validator("lock-v1.json")
        with self.assertRaisesRegex(ValueError, "control"):
            rejection(check, {}, "invalid baseline", lambda v: v.update(config_sha256="wrong"), "pattern", ["config_sha256"])

    def test_coverage_does_not_use_removable_assert(self):
        with self.assertRaisesRegex(ValueError, "coverage"):
            coverage("empty run", 0, 0, 3, 12)

    def test_optimized_empty_graph_run_fails(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            (root / "scripts").mkdir()
            (root / "schemas").mkdir()
            for name in ["schema_checks.py", "validate-graph-v2.py"]:
                shutil.copyfile(ROOT / "scripts" / name, root / "scripts" / name)
            shutil.copyfile(ROOT / "schemas/cargo-graph-v2.json", root / "schemas/cargo-graph-v2.json")
            result = subprocess.run([sys.executable, "-O", str(root / "scripts/validate-graph-v2.py")], capture_output=True, text=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("fixture set changed", result.stderr)

    def test_unsigned_formats_have_standard_bounds(self):
        def inspect(value, path):
            if isinstance(value, dict):
                if value.get("format") in ("uint32", "uint64"):
                    bits = 32 if value["format"] == "uint32" else 64
                    self.assertIn("maximum", value, path)
                    self.assertLessEqual(value["maximum"], 2**bits-1, path)
                for key, child in value.items():
                    inspect(child, path + "/" + key)
            elif isinstance(value, list):
                for index, child in enumerate(value):
                    inspect(child, path + "/" + str(index))
        for path in sorted((ROOT / "schemas").glob("*.json")):
            inspect(json.loads(path.read_text(encoding="utf-8")), path.name)

    def test_frozen_native_catalog_bytes_still_match_both_contexts(self):
        root = ROOT / "examples/runtime-native-v2"
        data = (root / "catalog.json").read_bytes()
        expected = {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}
        for name in ["context.json", "context-v3.json"]:
            self.assertEqual(json.loads((root / name).read_bytes())["release"]["catalog"], expected)

    @unittest.skipUnless(os.name == "posix", "Unix staging permissions")
    def test_qualified_output_is_private_before_bytes_are_written_and_never_clobbered(self):
        module = load_script("qualify-test-sbom-validator")
        data = b"inert test data; never executed"
        pin = {"platform": "linux-x86_64", "name": "cyclonedx-linux-x64", "url": module.BASE + "cyclonedx-linux-x64", "bytes": {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}}
        pins = json.dumps({"version": "0.33.1", "source_commit": module.SOURCE, "distributions": [pin]})
        actual_fdopen = os.fdopen
        modes = []
        def observe(fd, *args, **kwargs):
            modes.append(stat.S_IMODE(os.fstat(fd).st_mode))
            return actual_fdopen(fd, *args, **kwargs)
        with tempfile.TemporaryDirectory() as path, patch.object(Path, "read_text", return_value=pins), patch.object(module.platform, "system", return_value="Linux"), patch.object(module.platform, "machine", return_value="x86_64"), patch.object(module.urllib.request, "urlopen", side_effect=lambda *args, **kwargs: io.BytesIO(data)), patch.object(module.os, "fdopen", side_effect=observe):
            destination = Path(path) / "qualified"
            output = module.qualify(destination)
            self.assertEqual(modes, [0o600])
            self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o500)
            self.assertEqual(output.read_bytes(), data)
            with self.assertRaises(FileExistsError):
                module.qualify(destination)
            self.assertEqual(output.read_bytes(), data)


if __name__ == "__main__":
    unittest.main()
