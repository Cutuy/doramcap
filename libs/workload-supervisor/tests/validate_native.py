#!/usr/bin/env python3
"""Run prebuilt Supervisor tests with native capabilities; never build as root."""
import argparse
import json
import os
from pathlib import Path
import pwd
import shutil
import signal
import sys
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
    """Check terminal stop and forced test cleanup with real monitor processes."""
    for stop in (signal.SIGINT, signal.SIGKILL):
        # Arrange: isolate the example's terminal group and owned cgroup.
        with tempfile.TemporaryDirectory(prefix='supervisor-sigint-') as temp:
            directory = Path(temp)
            cgroup = Path('/sys/fs/cgroup') / directory.name
            settings = dict(id='terminal', output_directory=str(directory / 'outputs'),
                            supervisord=supervisord, supervisorctl=supervisorctl,
                            monitor_binary=str(monitor.resolve()), readiness_seconds=15,
                            stop_seconds=3, cgroup_parent=str(cgroup))
            config = directory / 'run.pbtxt'
            config.write_text('\n'.join(f'{key}: {json.dumps(value)}'
                                        for key, value in settings.items()))
            cgroup.mkdir()
            child = None
            try:
                with (directory / 'events.log').open('w') as events:
                    child = subprocess.Popen(
                        [str(monitor.resolve().parent / 'examples/supervisor_monitor'),
                         str(config), '60'], stdout=subprocess.PIPE, stderr=events,
                        text=True, start_new_session=True)
                    deadline = time.monotonic() + 20
                    while '::Inspect Succeeded' not in (directory / 'events.log').read_text():
                        if child.poll() is not None or time.monotonic() >= deadline:
                            raise AssertionError('example did not reach recording readiness')
                        time.sleep(0.05)
                    # Act: cooperative terminal stop or abrupt manager death.
                    os.killpg(child.pid, stop)
                    output, _ = child.communicate(timeout=20)
                    # Assert: cooperative stop verifies each independent outcome.
                    if stop == signal.SIGINT:
                        assert child.returncode == 0, (child.returncode, output)
                        report = json.loads(output)
                        assert report['execution_error'] is None and not report['diagnostics'], report
                        assert all(report[key] for key in
                                   ('writers_stopped', 'recording_finalized', 'cleanup_verified')), report
                        assert not (cgroup / 'terminal').exists()
                    else:
                        assert child.returncode == -signal.SIGKILL, child.returncode
            finally:
                failure = sys.exception()
                try:
                    if child is not None:
                        if child.poll() is None:
                            child.kill()
                        child.wait()
                    (cgroup / 'cgroup.kill').write_text('1')
                    deadline = time.monotonic() + 5
                    while 'populated 0' not in (cgroup / 'cgroup.events').read_text().splitlines():
                        if time.monotonic() >= deadline:
                            raise TimeoutError('test cgroup remains populated')
                        time.sleep(0.05)
                    if (cgroup / 'terminal').exists():
                        (cgroup / 'terminal').rmdir()
                    cgroup.rmdir()
                except Exception as error:
                    if failure is None:
                        raise
                    failure.add_note(f'test cgroup cleanup failed: {error}')
            assert not cgroup.exists()
    print('PASS Supervisor terminal stop and killed-manager cleanup', flush=True)


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
