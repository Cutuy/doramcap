#!/usr/bin/env python3
"""Build unprivileged, package trusted native monitor, run opt-in Docker E2E.

Requires a local Linux Docker daemon, native BPF prerequisites, and Python 3.
--sudo uses the main checkout's protected credential only through sudo stdin.
Artifacts remain in --output (which must not already exist); the image is removed.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[3]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sudo', action='store_true')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--live-demo', action='store_true', help='serve the interactive orchestrator lab')
    parser.add_argument('--port', type=int, default=8092)
    args = parser.parse_args()
    output = args.output.absolute()
    output.mkdir(parents=True, exist_ok=False)
    credential = None
    if args.sudo:
        worktrees = subprocess.check_output(['git', 'worktree', 'list', '--porcelain'], cwd=REPO, text=True)
        main_checkout = Path(worktrees.splitlines()[0].removeprefix('worktree '))
        credential = main_checkout / '.sudo-password'
        if not credential.is_file() or credential.stat().st_mode & 0o777 != 0o600:
            raise SystemExit('Provision main checkout .sudo-password locally with mode 0600.')
        subprocess.run(['git', 'check-ignore', '-q', '.sudo-password'], cwd=main_checkout, check=True)

    def run(command, elevated=False, **kwargs):
        if elevated and credential:
            with credential.open('rb') as secret:
                return subprocess.run(['sudo', '-S', '-p', '', '--', *command], stdin=secret, check=True, **kwargs)
        return subprocess.run(command, check=True, **kwargs)

    def live(target, image):
        with tempfile.TemporaryDirectory(prefix='doramcap-live-') as private:
            socket = Path(private) / 'worker.sock'
            server = subprocess.Popen([str(target / 'orchestrator-demo'), str(socket), str(args.port)])
            worker = None
            try:
                for _ in range(100):
                    if socket.exists(): break
                    if server.poll() is not None: raise RuntimeError('Demo server exited')
                    time.sleep(0.05)
                else: raise RuntimeError('Demo server did not become ready')
                command = [str(target / 'worker'), str(socket), image, str(output)]
                if credential:
                    with credential.open('rb') as secret:
                        worker = subprocess.Popen(['sudo', '-S', '-p', '', '--', *command], stdin=secret)
                else:
                    worker = subprocess.Popen(command)
                print(f'Open http://127.0.0.1:{args.port}; Ctrl-C drains the active experiment and exits.', flush=True)
                try:
                    if worker.wait() != 0: raise RuntimeError('Docker worker failed')
                except KeyboardInterrupt:
                    server.terminate()
                    worker.wait(timeout=60)
            finally:
                server.terminate()
                server.wait(timeout=10)
                if worker is not None and worker.poll() is None:
                    # Closing the server disconnects control; the active workflow
                    # still awaits its ordinary cleanup before the worker exits.
                    worker.wait(timeout=60)

    # Cargo and image assembly are always unprivileged.
    run(['cargo', 'build', '--locked', '-p', 'system-monitor', '--features', 'bpf'], cwd=REPO)
    run(['cargo', 'build', '--locked', '-p', 'orchestrator', '--example', 'docker_monitor'], cwd=REPO)
    if args.live_demo:
        run(['cargo', 'build', '--locked', '-p', 'orchestrator-demo', '--bins'], cwd=REPO)
        run(['npm', 'ci', '--prefix', 'apps/orchestrator-demo/frontend'], cwd=REPO)
        run(['npm', 'run', 'build', '--prefix', 'apps/orchestrator-demo/frontend'], cwd=REPO)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version', '1'], cwd=REPO))
    target = Path(metadata['target_directory']) / 'debug'
    binary = target / 'system-monitor'
    image = f'doramcap-monitor-e2e:{os.getpid()}-{time.time_ns()}'
    run(['docker', 'version'], elevated=True, stdout=(output / 'docker-version.txt').open('w'))
    # Limit this harness to local Docker: mounts and PID checks refer to this host.
    info = run(['docker', 'context', 'inspect'], elevated=True, capture_output=True, text=True)
    endpoint = json.loads(info.stdout)[0]['Endpoints']['docker']['Host']
    if not endpoint.startswith('unix://') or os.environ.get('DOCKER_HOST') or os.environ.get('DOCKER_CONTEXT'):
        raise SystemExit('Use the default local Unix-socket Docker context with no Docker environment overrides.')
    with tempfile.TemporaryDirectory(prefix='doramcap-monitor-image-') as staging:
        staging = Path(staging)
        rootfs = staging / 'rootfs'
        rootfs.mkdir()
        shutil.copy2(binary, rootfs / 'system-monitor')
        ldd = subprocess.check_output(['ldd', str(binary)], text=True)
        (output / 'libraries.txt').write_text(ldd)
        for line in ldd.splitlines():
            match = re.search(r'(?:=>\s+)?(/\S+)', line)
            if match:
                source = Path(match.group(1))
                # Libraries resolved through a local build-toolchain path still need
                # to live on the container loader's default search path.
                destination = rootfs / 'lib' / source.name
                if 'ld-linux' in source.name:
                    destination = rootfs / source.relative_to('/')
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, destination)
        (rootfs / 'recordings').mkdir()
        (staging / 'Dockerfile').write_text('FROM scratch\nCOPY rootfs /\nENTRYPOINT ["/system-monitor"]\n')
        built = False
        try:
            run(['docker', 'build', '--network=none', '-t', image, str(staging)], elevated=True)
            built = True
            manifest = run(['docker', 'image', 'inspect', image], elevated=True, capture_output=True, text=True)
            (output / 'image.json').write_text(manifest.stdout)
            if args.live_demo:
                live(target, image)
            else:
                run([str(target / 'examples/docker_monitor'), image, str(output)], elevated=True)
        finally:
            if built:
                run(['docker', 'image', 'rm', image], elevated=True)
    print(f'Validated native Docker lifecycle. Evidence: {output}')


if __name__ == '__main__':
    main()
