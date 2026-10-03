#!/usr/bin/env python3
"""Qualify only one compiled-pin native CycloneDX test distribution; never execute fetched bytes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
PINS = ROOT / "tests/fixtures/cyclonedx/validator-pins.json"
SOURCE = "b3cfa4b0edc356dad07e0b6e7ab6da0a94af0246"
BASE = "https://github.com/CycloneDX/cyclonedx-cli/releases/download/v0.33.1/"
LIMIT = 128 * 1024 * 1024


def qualify(destination):
    """Read exact native pins, rehash bounded official bytes and create only a new owned leaf."""
    document = json.loads(PINS.read_text())
    if document["version"] != "0.33.1" or document["source_commit"] != SOURCE:
        raise ValueError("unsupported schema validator source")
    systems = {"Linux": "linux", "Darwin": "macos"}
    architectures = {"x86_64": "x86_64", "aarch64": "aarch64", "arm64": "aarch64"}
    key = systems.get(platform.system(), "unsupported") + "-" + architectures.get(platform.machine(), "unsupported")
    names = {"linux-x86_64": "cyclonedx-linux-x64", "linux-aarch64": "cyclonedx-linux-arm64", "macos-aarch64": "cyclonedx-osx-arm64"}
    matches = [item for item in document["distributions"] if item["platform"] == key]
    if key not in names or len(matches) != 1:
        raise ValueError("unsupported native schema validator platform")
    pin = matches[0]
    url = BASE + names[key]
    if pin["name"] != names[key] or pin["url"] != url or not 0 < pin["bytes"]["size"] <= LIMIT:
        raise ValueError("unsupported schema validator distribution")
    with urllib.request.urlopen(url, timeout=30) as response:
        data = response.read(LIMIT + 1)
    if len(data) != pin["bytes"]["size"] or hashlib.sha256(data).hexdigest() != pin["bytes"]["sha256"]:
        raise ValueError("unapproved schema validator bytes")
    # The Rust integration test independently checks the compiled official pin before execution.
    destination.mkdir(parents=True, exist_ok=True)
    if destination.is_symlink():
        raise ValueError("schema validator output directory may not be a symlink")
    output = destination / "cyclonedx"
    with output.open("xb") as handle:
        handle.write(data)
    output.chmod(0o500)
    return output.resolve()


def main():
    """Optionally export only the qualified native path to the explicit hosted integration step."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument("--github-env", action="store_true")
    args = parser.parse_args()
    output = qualify(args.output_directory)
    if args.github_env:
        if "\n" in str(output) or "\r" in str(output):
            raise ValueError("unsafe schema validator output path")
        with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as environment:
            environment.write("ARMORER_TEST_CDX=" + str(output) + "\n")
    print("Qualified native CycloneDX CLI 0.33.1: " + str(output))


if __name__ == "__main__":
    main()
