#!/usr/bin/env python3
"""Verify a submitted snapshot and synchronize a dedicated, disposable source cache.

Uses rsync for content comparison, permission updates, and deletion. Identical
files keep their destination mtimes; changed files receive fresh mtimes. The
caller must hold the worker lock through synchronization, building, and testing.
Raises on malformed manifests, unexpected files, symlinks, or digest mismatches.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def files(directory, ignored=()):
    result = set()
    for path in directory.rglob('*'):
        if path.is_symlink():
            raise ValueError(f'Symlink in source tree: {path}')
        if path.is_dir():
            continue
        if not path.is_file():
            raise ValueError(f'Non-regular source file: {path}')
        relative = path.relative_to(directory).as_posix()
        if relative not in ignored:
            result.add(relative)
    return result


def verify(directory, expected, modes, ignored=()):
    actual = files(directory, ignored)
    if actual != set(expected):
        raise ValueError(f'Source inventory mismatch: missing={set(expected) - actual}, '
                         f'extra={actual - set(expected)}')
    for name, checksum in expected.items():
        if digest(directory / name) != checksum:
            raise ValueError(f'Source checksum mismatch: {name}')
        if name in modes and (directory / name).stat().st_mode & 0o777 != modes[name]:
            raise ValueError(f'Source permission mismatch: {name}')


def prepare(snapshot, destination):
    if snapshot.is_symlink() or destination.is_symlink():
        raise ValueError('Snapshot and cache must be real directories')
    snapshot = snapshot.resolve()
    destination = destination.resolve()
    if snapshot == destination or snapshot in destination.parents or destination in snapshot.parents:
        raise ValueError('Snapshot and cache must be separate, non-overlapping directories')
    manifest = json.loads((snapshot / 'SOURCE.json').read_text())
    expected = dict(manifest['files_sha256'])
    expected.update({
        'run-worker-job.sh': manifest['runner_sha256'],
        'prepare-build.py': manifest['prepare_sha256'],
        'SOURCE.json': digest(snapshot / 'SOURCE.json'),
    })
    for name in expected:
        path = Path(name)
        if path.is_absolute() or '..' in path.parts or path.as_posix() != name:
            raise ValueError(f'Invalid manifest path: {name}')
    modes = manifest['files_mode']
    if set(modes) != set(manifest['files_sha256']):
        raise ValueError('Source permission inventory mismatch')
    verify(snapshot, expected, modes, ignored={'source.tar.gz'})
    destination.mkdir(parents=True, exist_ok=True)
    # Refuse symlinks before allowing rsync to delete stale generated/source files.
    files(destination)
    subprocess.run([
        'rsync', '-rcp', '--delete-excluded', '--exclude=/source.tar.gz',
        '--itemize-changes', '--', str(snapshot) + '/', str(destination) + '/',
    ], check=True)
    verify(destination, expected, modes)
    # Native Python tests may write bytecode as root. Keep its containing
    # directories CI-owned so the next unprivileged rsync can remove it.
    for name in expected:
        if name.endswith('.py'):
            (destination / name).parent.joinpath('__pycache__').mkdir(exist_ok=True)
    print(f'Verified {len(expected)} files from {snapshot} in {destination}', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('snapshot', type=Path)
    parser.add_argument('destination', type=Path, help='dedicated disposable source cache')
    args = parser.parse_args()
    prepare(args.snapshot, args.destination)
