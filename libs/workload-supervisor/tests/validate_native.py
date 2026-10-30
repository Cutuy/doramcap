#!/usr/bin/env python3
"""Run prebuilt Supervisor tests with native capabilities; never build as root."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess


def validate(monitor):
    binary_dir = monitor.resolve().parent
    fixture = binary_dir / 'supervisor-test-fixture'
    supervisord = shutil.which('supervisord')
    supervisorctl = shutil.which('supervisorctl')
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
            return True
    raise RuntimeError('Build workload-supervisor --test native unprivileged before native validation')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--monitor', required=True, type=Path)
    args = parser.parse_args()
    raise SystemExit(0 if validate(args.monitor) else 1)
