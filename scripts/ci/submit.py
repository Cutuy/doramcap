#!/usr/bin/env python3
"""Snapshot a source checkout and enqueue ARM64 tests over SSH."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import shlex
import subprocess
import tarfile
import tempfile
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--source', type=Path, default=Path.cwd())
parser.add_argument('--host', default='doramcap-ci@100.64.0.20')
args = parser.parse_args()
source = args.source.resolve()
for required in ('examples/validate_native.py', 'crates/cgroup-bpf/examples/native_allocations.rs'):
    if not (source / required).is_file():
        parser.error(f'Source lacks {required}; use a checkout containing the native validation suite')
job = '/home/doramcap-ci/jobs/' + uuid.uuid4().hex

def git(*argv):
    return subprocess.check_output(['git', '-C', str(source), *argv])

# Include only build/test source paths; never tar the checkout or ignored files.
paths = sorted(set(git('ls-files', '-z', '--cached', '--others', '--exclude-standard').decode().split('\0')))
paths = [p for p in paths if p in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml')
         or p.startswith(('crates/', 'examples/'))]
manifest = {'head': git('rev-parse', 'HEAD').decode().strip(),
            'dirty': bool(git('status', '--porcelain')), 'files_sha256': {}}
with tempfile.TemporaryDirectory(prefix='doramcap-submit-') as tmp:
    archive = Path(tmp) / 'source.tar.gz'
    with tarfile.open(archive, 'w:gz') as tar:
        for relative in paths:
            path = source / relative
            if not path.exists():
                continue
            if path.is_symlink() or not path.is_file():
                raise SystemExit(f'Refusing non-regular source file: {relative}')
            data = path.read_bytes()
            manifest['files_sha256'][relative] = hashlib.sha256(data).hexdigest()
            info = tarfile.TarInfo(relative)
            info.size = len(data)
            info.mode = path.stat().st_mode & 0o777
            tar.addfile(info, io.BytesIO(data))
        runner = Path(__file__).with_name('run-worker-job.sh').read_bytes()
        manifest['runner_sha256'] = hashlib.sha256(runner).hexdigest()
        for name, data in [('run-worker-job.sh', runner),
                           ('SOURCE.json', json.dumps(manifest, indent=2).encode())]:
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mode = 0o644
            tar.addfile(info, io.BytesIO(data))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    subprocess.run(['ssh', args.host, 'mkdir -p ' + shlex.quote(job)], check=True)
    subprocess.run(['scp', str(archive), f'{args.host}:{job}/source.tar.gz'], check=True)
    command = (f'cd {shlex.quote(job)} && '
               f'echo "{digest}  source.tar.gz" | sha256sum -c - && '
               'tar -xzf source.tar.gz && '
               'pueue add --label doramcap-native -- bash run-worker-job.sh')
    subprocess.run(['ssh', args.host, command], check=True)
    print(f'Worker snapshot: {args.host}:{job}')
