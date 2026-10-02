#!/usr/bin/env python3
"""Fetch one exact native gh for genuine fixture CI. Never execute fetched bytes.
All archives and the checksum manifest are committed digest pins. Only the named
regular executable leaf is read; archive trees are never extracted.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import tarfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PINS = ROOT / "tests/fixtures/sigstore/verifier-pins.json"
BASE_URL = "https://github.com/cli/cli/releases/download/v2.102.0/"
MAX_ARCHIVE = 32 * 1024 * 1024


def fetch(name, limit):
    """Download one fixed release path with a byte cap; never execute returned bytes."""
    with urllib.request.urlopen(BASE_URL + name, timeout=30) as response:
        data = response.read(limit + 1)
    if len(data) > limit:
        raise ValueError("official verifier distribution exceeds bound")
    return data


def qualify(destination):
    """Authenticate manifest, archive and executable, then create a new owned output."""
    pins = json.loads(PINS.read_text())
    if pins["version"] != "2.102.0" or pins["source_commit"] != "fc4b137cdef0a6bd28fd461b7cf9c84a5812a8cd":
        raise ValueError("unsupported verifier source/version")
    architecture = {"x86_64": "amd64", "aarch64": "arm64", "arm64": "arm64"}.get(platform.machine().lower())
    key = platform.system().lower() + "-" + str(architecture)
    matches = [pin for pin in pins["pins"] if pin["platform"] == key]
    if len(matches) != 1:
        raise ValueError("unsupported native verifier platform")
    pin = matches[0]
    archive_name = {
        "linux-amd64": "gh_2.102.0_linux_amd64.tar.gz",
        "linux-arm64": "gh_2.102.0_linux_arm64.tar.gz",
        "darwin-arm64": "gh_2.102.0_macOS_arm64.zip",
    }[key]
    if pin["archive"]["name"] != archive_name:
        raise ValueError("unexpected verifier archive")
    manifest = fetch("gh_2.102.0_checksums.txt", 1048576)
    if hashlib.sha256(manifest).hexdigest() != pins["manifest_sha256"]:
        raise ValueError("unapproved verifier checksum manifest")
    archive = fetch(archive_name, MAX_ARCHIVE)
    if len(archive) != pin["archive"]["size"] or hashlib.sha256(archive).hexdigest() != pin["archive"]["sha256"]:
        raise ValueError("unapproved verifier archive bytes")
    entry = pin["archive"]["sha256"] + "  " + archive_name
    if manifest.decode("ascii").splitlines().count(entry) != 1:
        raise ValueError("official manifest does not bind archive")
    if archive_name.endswith(".zip"):
        with zipfile.ZipFile(io.BytesIO(archive)) as container:
            entries = [item for item in container.infolist() if item.filename == pin["leaf"]]
            if len(entries) != 1:
                raise ValueError("ambiguous executable leaf")
            item = entries[0]
            if item.is_dir() or ((item.external_attr >> 16) & 0o170000) == 0o120000 or item.file_size != pin["executable"]["size"]:
                raise ValueError("unsupported executable leaf")
            executable = container.read(item)
    else:
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as container:
            entries = [item for item in container.getmembers() if item.name == pin["leaf"]]
            if len(entries) != 1 or not entries[0].isfile() or entries[0].size != pin["executable"]["size"]:
                raise ValueError("ambiguous or unsupported executable leaf")
            with container.extractfile(entries[0]) as stream:
                executable = stream.read(pin["executable"]["size"] + 1)
    if len(executable) != pin["executable"]["size"] or hashlib.sha256(executable).hexdigest() != pin["executable"]["sha256"]:
        raise ValueError("unapproved executable bytes")
    destination.mkdir(parents=True, exist_ok=True)
    output = destination / "gh"
    # A conflicting existing file is an error; do not replace an unrelated tool.
    with output.open("xb") as stream:
        stream.write(executable)
    output.chmod(0o500)
    receipt = {"source_commit": pins["source_commit"], "version": pins["version"], "platform": key,
               "manifest_sha256": pins["manifest_sha256"], "archive": pin["archive"], "executable": pin["executable"]}
    (destination / "qualification.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return output.resolve()


def main():
    """Qualify a native test tool and optionally export only its path to hosted CI."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument("--github-env", action="store_true", help="Append qualified path to the current CI environment file")
    arguments = parser.parse_args()
    output = qualify(arguments.output_directory)
    if arguments.github_env:
        if "\n" in str(output) or "\r" in str(output):
            raise ValueError("invalid qualified executable path")
        with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as environment:
            environment.write("ARMORER_TEST_GH=" + str(output) + "\n")
    print("Qualified native GitHub CLI 2.102.0: " + str(output))


if __name__ == "__main__":
    main()
