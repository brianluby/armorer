"""Retain unprivileged native candidate bytes and build observations; establish no release trust."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import stat
import subprocess


def fixed_output(arguments, root):
    """Read bounded fixed native tool output without caller command or shell inputs."""
    result = subprocess.run(arguments, cwd=root, check=True, capture_output=True, timeout=30)
    if len(result.stdout) > 65536 or len(result.stderr) > 65536:
        raise ValueError('candidate tool output exceeds bound')
    return result.stdout.decode('utf-8').strip()


def identity(path, limit):
    """Hash a bounded regular leaf, checking its opened inode and final size."""
    inspected = path.lstat()
    if not stat.S_ISREG(inspected.st_mode) or not 0 < inspected.st_size <= limit:
        raise ValueError('candidate file type or size')
    digest = hashlib.sha256()
    size = 0
    with path.open('rb') as stream:
        opened = os.fstat(stream.fileno())
        if (opened.st_dev, opened.st_ino, opened.st_size) != (inspected.st_dev, inspected.st_ino, inspected.st_size):
            raise ValueError('candidate file changed')
        while block := stream.read(65536):
            size += len(block)
            if size > limit:
                raise ValueError('candidate file growth')
            digest.update(block)
    if size != inspected.st_size:
        raise ValueError('candidate file size changed')
    return {'sha256': digest.hexdigest(), 'size': size}


def main():
    """Stage exactly two native CI leaves under immutable source/run/attempt observations."""
    arguments = argparse.ArgumentParser(description=__doc__)
    arguments.add_argument('--output-directory', required=True, type=Path)
    args = arguments.parse_args()
    root = Path(__file__).resolve().parent.parent
    targets = {('Linux', 'x86_64'): 'x86_64-unknown-linux-gnu', ('Linux', 'aarch64'): 'aarch64-unknown-linux-gnu', ('Darwin', 'arm64'): 'aarch64-apple-darwin'}
    target = targets.get((platform.system(), platform.machine()))
    if target is None:
        raise ValueError('unsupported runtime candidate platform')
    if os.environ.get('GITHUB_REPOSITORY') != 'brianluby/armorer':
        raise ValueError('unexpected candidate repository')
    commit = fixed_output(['git', 'rev-parse', 'HEAD'], root)
    if not re.fullmatch('[0-9a-f]{40}', commit) or commit != os.environ.get('GITHUB_SHA'):
        raise ValueError('candidate source checkout mismatch')
    run = os.environ.get('GITHUB_RUN_ID', '')
    attempt = os.environ.get('GITHUB_RUN_ATTEMPT', '')
    if not re.fullmatch('[1-9][0-9]{0,19}', run) or not re.fullmatch('[1-9][0-9]{0,19}', attempt):
        raise ValueError('candidate run identity')
    compiler = fixed_output(['rustc', '--version'], root)
    if not compiler.startswith('rustc 1.95.0 '):
        raise ValueError('unqualified candidate compiler')
    before = identity(root/'target/release/armorer', 128*1024*1024)
    args.output_directory.mkdir(mode=0o700, exist_ok=False)
    executable = args.output_directory/'armorer'
    shutil.copyfile(root/'target/release/armorer', executable)
    if identity(executable, 128*1024*1024) != before:
        raise ValueError('candidate copy mismatch')
    executable.chmod(0o400)
    observation = {'schema_version': 1, 'kind': 'unprivileged-runtime-build-observation',
        'source': {'repository': 'brianluby/armorer', 'commit': commit, 'git_ref': os.environ.get('GITHUB_REF', '')},
        'compiler': '1.95.0', 'compiler_observation': compiler,
        'cargo_lock': identity(root/'Cargo.lock', 1048576), 'target': target,
        'run_id': int(run), 'run_attempt': int(attempt), 'event': os.environ.get('GITHUB_EVENT_NAME', ''),
        'executable': before, 'version': '0.1.0', 'authenticated_provenance': False,
        'signing_authorized': False, 'release_acceptance': False}
    metadata = args.output_directory/'runtime-build.json'
    metadata.write_text(json.dumps(observation, sort_keys=True, separators=(',', ':'))+'\n')
    metadata.chmod(0o400)


if __name__ == '__main__':
    main()
