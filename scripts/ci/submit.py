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
import time
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--source', type=Path, default=Path.cwd())
parser.add_argument('--host', default='doramcap-ci@100.64.0.20')
args = parser.parse_args()
source = args.source.resolve()
for required in ('apps/system-monitor/tests/validate_native.py', 'libs/resources/cgroup-bpf/examples/native_allocations.rs'):
    if not (source / required).is_file():
        parser.error(f'Source lacks {required}; use a checkout containing the native validation suite')
job = '/home/doramcap-ci/jobs/' + uuid.uuid4().hex
snapshot_mtime = time.time()
ssh_options = ['-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10',
               '-o', 'ServerAliveInterval=10', '-o', 'ServerAliveCountMax=3']

def git(*argv):
    return subprocess.check_output(['git', '-C', str(source), *argv])

# Include only build/test source paths; never tar the checkout or ignored files.
paths = sorted(set(git('ls-files', '-z', '--cached', '--others', '--exclude-standard').decode().split('\0')))
paths = [p for p in paths if p in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.node-version')
         or p.startswith(('libs/', 'apps/', 'console/server/native/', 'console/proto/', 'console/contract/',
                           'console/server/src/', 'console/server/tests/', 'console/server/scripts/',
                           'console/frontend/src/', 'console/frontend/scripts/'))
         or p in ('scripts/ci/check_boundaries.py', 'scripts/ci/test_boundaries.py', 'scripts/ci/boundaries.json', 'scripts/ci/test-node.mts',
                  'console/server/package.json', 'console/server/package-lock.json',
                  'console/server/tsconfig.json', 'console/server/tsconfig.build.json',
                  'console/frontend/package.json', 'console/frontend/package-lock.json',
                  'console/frontend/tsconfig.json', 'console/frontend/index.html')]
manifest = {'head': git('rev-parse', 'HEAD').decode().strip(),
            'dirty': bool(git('status', '--porcelain')), 'files_sha256': {}, 'files_mode': {}}
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
            manifest['files_mode'][relative] = path.stat().st_mode & 0o777
            info = tarfile.TarInfo(relative)
            info.mtime = snapshot_mtime
            info.size = len(data)
            info.mode = path.stat().st_mode & 0o777
            tar.addfile(info, io.BytesIO(data))
        # Older root-owned launchers invoke examples/validate_native.py. Keep their
        # source snapshot entry point until those launchers are upgraded.
        compatibility = b"import runpy\nfrom pathlib import Path\nrunpy.run_path(str(Path(__file__).resolve().parents[1] / 'apps/system-monitor/tests/validate_native.py'), run_name='__main__')\n"
        info = tarfile.TarInfo('examples/validate_native.py')
        info.mtime = snapshot_mtime
        info.size = len(compatibility)
        info.mode = 0o644
        tar.addfile(info, io.BytesIO(compatibility))
        manifest['files_sha256'][info.name] = hashlib.sha256(compatibility).hexdigest()
        manifest['files_mode'][info.name] = info.mode
        runner = Path(__file__).with_name('run-worker-job.sh').read_bytes()
        manifest['runner_sha256'] = hashlib.sha256(runner).hexdigest()
        prepare = Path(__file__).with_name('prepare-build.py').read_bytes()
        manifest['prepare_sha256'] = hashlib.sha256(prepare).hexdigest()
        for name, data in [('run-worker-job.sh', runner), ('prepare-build.py', prepare),
                           ('SOURCE.json', json.dumps(manifest, indent=2).encode())]:
            info = tarfile.TarInfo(name)
            info.mtime = snapshot_mtime
            info.size = len(data)
            info.mode = 0o644
            tar.addfile(info, io.BytesIO(data))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    subprocess.run(['ssh', *ssh_options, args.host, 'mkdir -p ' + shlex.quote(job)], check=True)
    subprocess.run(['scp', *ssh_options, str(archive), f'{args.host}:{job}/source.tar.gz'], check=True)
    command = (f'cd {shlex.quote(job)} && '
               f'echo "{digest}  source.tar.gz" | sha256sum -c - && '
               'tar -xzpf source.tar.gz && '
               'pueue add --label doramcap-native -- bash run-worker-job.sh')
    subprocess.run(['ssh', *ssh_options, args.host, command], check=True)
    print(f'Worker snapshot: {args.host}:{job}')
