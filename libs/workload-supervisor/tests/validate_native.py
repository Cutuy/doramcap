#!/usr/bin/env python3
"""Run prebuilt Supervisor tests with native capabilities; never build as root."""
import argparse
import json
import os
from pathlib import Path
import pwd
import shutil
import signal
import tempfile
import time
import subprocess


def runtime_executable(name, monitor):
    system = shutil.which(name)
    if system:
        return system
    owner_home = Path(pwd.getpwuid(monitor.stat().st_uid).pw_dir)
    private = owner_home / '.local/share/doramcap-supervisor/bin' / name
    return str(private) if private.is_file() and os.access(private, os.X_OK) else None


def validate_terminal_stop(monitor, supervisord, supervisorctl):
    """Exercise terminal SIGINT delivery to the example's foreground process group."""
    # Arrange: keep the test runner outside the example's process group.
    example = monitor.resolve().parent / 'examples/supervisor_monitor'
    with tempfile.TemporaryDirectory(prefix='supervisor-sigint-') as temp:
        directory = Path(temp)
        cgroup = Path('/sys/fs/cgroup') / directory.name
        cgroup.mkdir()
        config = directory / 'run.pbtxt'
        settings = dict(id='terminal', output_directory=str(directory / 'outputs'),
                        supervisord=supervisord, supervisorctl=supervisorctl,
                        monitor_binary=str(monitor.resolve()), readiness_seconds=15,
                        stop_seconds=3, cgroup_parent=str(cgroup))
        config.write_text('\n'.join(f'{key}: {json.dumps(value)}'
                                    for key, value in settings.items()))
        try:
            with (directory / 'events.log').open('w') as events:
                child = subprocess.Popen([str(example), str(config), '60'],
                                         stdout=subprocess.PIPE, stderr=events,
                                         text=True, start_new_session=True)
                try:
                    deadline = time.monotonic() + 20
                    while True:
                        if '::Inspect Succeeded' in (directory / 'events.log').read_text():
                            break
                        if child.poll() is not None or time.monotonic() >= deadline:
                            raise AssertionError('example did not reach recording readiness')
                        time.sleep(0.05)
                    # Act: terminal Ctrl-C targets the entire foreground group.
                    os.killpg(child.pid, signal.SIGINT)
                    output, _ = child.communicate(timeout=20)
                    # Assert: cooperative stop preserves every independent outcome.
                    assert child.returncode == 0, (child.returncode, output)
                    report = json.loads(output)
                    assert report['execution_error'] is None, report
                    assert report['writers_stopped'], report
                    assert report['recording_finalized'], report
                    assert report['cleanup_verified'], report
                    assert not report['diagnostics'], report
                    assert not (cgroup / 'terminal').exists()
                    print('PASS Supervisor terminal Ctrl-C recording and cleanup', flush=True)
                finally:
                    if child.poll() is None:
                        child.kill()
                    child.wait()
        finally:
            cgroup.rmdir()


def validate(monitor):
    binary_dir = monitor.resolve().parent
    fixture = binary_dir / 'supervisor-test-fixture'
    supervisord = runtime_executable('supervisord', monitor)
    supervisorctl = runtime_executable('supervisorctl', monitor)
    if not supervisord or not supervisorctl:
        print('SKIP Supervisor native scenario: provision supervisord and supervisorctl', flush=True)
        return False
    if not fixture.is_file():
        raise RuntimeError('Build tests/fixture.inc unprivileged as supervisor-test-fixture beside system-monitor')
    candidates = sorted((binary_dir / 'deps').glob('native-*'), key=lambda p: p.stat().st_mtime, reverse=True)
    for candidate in candidates:
        if not candidate.is_file() or not os.access(candidate, os.X_OK):
            continue
        listed = subprocess.check_output([str(candidate), '--list'], text=True)
        if 'real_recordings_spaces_and_concurrent_runs:' in listed:
            environment = dict(os.environ, DORAMCAP_NATIVE_MONITOR=str(monitor.resolve()),
                               DORAMCAP_TEST_FIXTURE=str(fixture),
                               DORAMCAP_SUPERVISORD=supervisord, DORAMCAP_SUPERVISORCTL=supervisorctl)
            subprocess.run([str(candidate), '--ignored', '--nocapture', '--test-threads=1'],
                           env=environment, check=True, timeout=120)
            validate_terminal_stop(monitor, supervisord, supervisorctl)
            return True
    raise RuntimeError('Build workload-supervisor --test native unprivileged before native validation')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--monitor', required=True, type=Path)
    args = parser.parse_args()
    raise SystemExit(0 if validate(args.monitor) else 1)
