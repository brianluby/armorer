#!/usr/bin/env python3
"""Stage one compiled-pin public signed Apple reference as inert data; never execute it."""
import argparse
import gzip
import hashlib
import io
import os
from pathlib import Path
import platform
import tarfile
import urllib.request

URL = "https://github.com/brianluby/momus-review/releases/download/v0.3.1/momus-aarch64-apple-darwin.tar.gz"
ARCHIVE_SIZE = 4192748
ARCHIVE_SHA256 = "93a3eee90f372f63c1abd027cc6a85f1bd1d1d505f83f6790211e136ba6863ea"
BINARY_SIZE = 11571456
BINARY_SHA256 = "5e7b3040ed2e772715b6fa19f6269b3b45f6705c769e29a6aceeb9ad7bd62caa"
MAX_TAR = 16 * 1024 * 1024
MEMBERS = {"momus-aarch64-apple-darwin", "momus-aarch64-apple-darwin/LICENSE",
           "momus-aarch64-apple-darwin/README.md", "momus-aarch64-apple-darwin/momus"}


def qualify(destination):
    """Rehash a fixed bounded release and write only its explicitly pinned executable to a fresh private leaf."""
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise ValueError("native Apple reference requires macOS ARM64")
    with urllib.request.urlopen(URL, timeout=60) as response:
        archive = response.read(ARCHIVE_SIZE + 1)
    if len(archive) != ARCHIVE_SIZE or hashlib.sha256(archive).hexdigest() != ARCHIVE_SHA256:
        raise ValueError("unapproved Apple reference archive bytes")
    with gzip.GzipFile(fileobj=io.BytesIO(archive)) as stream:
        expanded = stream.read(MAX_TAR + 1)
    if len(expanded) > MAX_TAR:
        raise ValueError("Apple reference archive exceeds expansion bound")
    with tarfile.open(fileobj=io.BytesIO(expanded), mode="r:") as stream:
        members = stream.getmembers()
        if len(members) != 4 or {member.name for member in members} != MEMBERS:
            raise ValueError("unexpected legacy Apple reference archive members")
        for member in members:
            if member.name == "momus-aarch64-apple-darwin":
                if not member.isdir():
                    raise ValueError("unexpected Apple reference directory")
            elif not member.isfile():
                raise ValueError("unsafe Apple reference member type")
        member = next(member for member in members if member.name == "momus-aarch64-apple-darwin/momus")
        if member.size != BINARY_SIZE:
            raise ValueError("unexpected Apple reference executable size")
        with stream.extractfile(member) as reader:
            binary = reader.read(BINARY_SIZE + 1)
    if len(binary) != BINARY_SIZE or hashlib.sha256(binary).hexdigest() != BINARY_SHA256:
        raise ValueError("unapproved Apple reference executable bytes")
    # This explicitly named legacy fixture reader grants no Armorer archive/release proof.
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.mkdir(mode=0o700)
    output = destination / "payload.macho"
    with output.open("xb") as writer:
        writer.write(binary)
    output.chmod(0o400)
    return output.resolve()


def main():
    """Export only the pin-matched inert path for the separately required native signature/ticket test."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument("--github-env", action="store_true")
    args = parser.parse_args()
    output = qualify(args.output_directory)
    if args.github_env:
        if "\n" in str(output) or "\r" in str(output):
            raise ValueError("unsafe Apple reference output path")
        with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as environment:
            environment.write("ARMORER_TEST_APPLE_REFERENCE=" + str(output) + "\n")
    print("Pin-matched inert Apple reference: " + str(output))


if __name__ == "__main__":
    main()
