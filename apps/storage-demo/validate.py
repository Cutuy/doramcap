#!/usr/bin/env python3
"""Exercise the real demo HTTP API, direct writes, rotation and MCAP readback."""
import argparse
import json
from pathlib import Path
import select
import signal
import subprocess
import tempfile
import time
import urllib.request

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--bin-dir', type=Path, required=True)
p.add_argument('--monitor', type=Path)
p.add_argument('--backend', choices=['synthetic', 'bpf'], default='synthetic')
a = p.parse_args()
bins = a.bin_dir.resolve()
with tempfile.TemporaryDirectory(prefix='storage-demo-validation-') as directory:
    root = Path(directory) / 'session'
    proc = subprocess.Popen([str(bins / 'storage-demo'), '--monitor', str(a.monitor.resolve() if a.monitor else bins / 'system-monitor'),
                             '--start-paused', '--backend', a.backend, '--seconds', '2', '--port', '0', '--dir', str(root)], stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, text=True)
    live_stream = None
    try:
        assert select.select([proc.stdout], [], [], 15)[0], 'startup timeout'
        line = proc.stdout.readline()
        assert 'http://' in line, line
        url = line.split('http://')[1].split()[0]
        base = 'http://' + url
        def state():
            with urllib.request.urlopen(base + '/api/state', timeout=3) as response:
                return json.load(response)
        def wait(predicate):
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                value = state()
                assert value['status'] != 'error', value
                if predicate(value):
                    return value
                time.sleep(.05)
            raise AssertionError('state timeout: ' + str(value)[:800])
        def control(action, **options):
            req = urllib.request.Request(base + '/api/control',
                 json.dumps({'action': action, **options}).encode(), {'Content-Type': 'application/json'})
            with urllib.request.urlopen(req, timeout=3) as response:
                assert response.status == 202
        assert state()['status'] == 'idle'
        assert not list(root.glob('*.mcap'))
        control('restart')
        wait(lambda s: s.get('cpu', 0) >= 3)
        with urllib.request.urlopen(base + '/events', timeout=3) as stream:
            assert stream.readline().startswith(b'data:')
        control('burst')
        acknowledged = wait(lambda s: any(r['kind'] == '16 heartbeats' for r in s.get('receipts', [])))
        receipt = acknowledged['receipts'][-1]
        assert receipt['count'] == 16 and receipt['last'] - receipt['first'] == 15
        assert receipt['bytes'] > 0
        control('large')
        acknowledged = wait(lambda s: any(r['kind'] == '96 KiB probe' for r in s.get('receipts', [])))
        receipt = acknowledged['receipts'][-1]
        assert receipt['count'] == 1 and receipt['bytes'] >= 96 * 1024
        assert receipt['encodeMs'] >= 0
        control('flush')
        wait(lambda s: s.get('probes', 0) == 1 and s.get('heartbeats', 0) >= 16)
        control('rotate')
        result = wait(lambda s: s.get('lastSealed') is not None)
        sealed = result['lastSealed']
        kinds = [r['kind'] for r in sealed['records']]
        assert kinds[-3:] == ['DataEnd', 'Footer', 'Magic / EOF'], kinds
        assert sealed['size'] % 4096 == 50, sealed['size']
        assert result['ring']['completions'] > 0
        assert any(e['kind'] == 'CQE' for e in result['events'])
        with urllib.request.urlopen(base + '/files/0', timeout=3) as response:
            assert response.read() == (root / 'capture-0000.mcap').read_bytes()
        subprocess.run([str(bins / 'examples/validate_capture'), str(root / 'capture-0000.mcap')], check=True)
        expired = wait(lambda s: s.get('status') == 'sealed')
        previous_session = expired['session']
        assert (root / 'capture-0001.mcap').exists()
        unrelated = root / 'keep.txt'
        unrelated.write_text('preserve')
        control('restart')
        restarted = wait(lambda s: s.get('status') == 'live' and s.get('session') != previous_session and s.get('cpu', 0) >= 1)
        assert restarted['probes'] == 0 and restarted['lastSealed'] is None
        assert restarted['cpu'] < expired['cpu']
        assert not (root / 'capture-0001.mcap').exists()
        assert unrelated.read_text() == 'preserve'
        previous_session = restarted['session']
        control('restart')
        wait(lambda s: s.get('status') == 'live' and s.get('session') != previous_session)
        initial = state()['probes']
        control('probe_burst', count=4, intervalMs=0)
        measured = wait(lambda s: s.get('probes', 0) >= initial+4 and s.get('latency', {}).get('count', 0) > 0 and s.get('timing', {}).get('sample', {}).get('count', 0) > 0)
        assert measured['latency']['maxMs'] >= measured['latency']['p99Ms'] >= 0
        control('probe_burst', count=256, intervalMs=1000)
        wait(lambda s: s.get('burstRemaining', 0) > 0)
        control('cancel_burst')
        wait(lambda s: s.get('burstRemaining', -1) == 0)
        control('probe_burst', count=256, intervalMs=1000)
        wait(lambda s: s.get('burstRemaining', 0) > 0)
        control('stop')
        stopped = wait(lambda s: s.get('status') == 'sealed')
        assert stopped['burstRemaining'] == 0
        assert stopped['file']['sealed']
        assert stopped['file']['records'][-1]['kind'] == 'Magic / EOF'
        size = (root / stopped['file']['name']).stat().st_size
        time.sleep(.2)
        assert (root / stopped['file']['name']).stat().st_size == size
        assert state()['status'] == 'sealed'
        control('restart')
        wait(lambda s: s.get('status') == 'live' and s.get('session') != stopped['session'])
        print('Stop passed: sealed tail, stable file, server stays available, restart succeeds')
        print('Restart passed: expired and active sessions, counters reset, old captures removed, unrelated file retained')
        live_stream = urllib.request.urlopen(base + '/events', timeout=3)
        assert live_stream.readline().startswith(b'data:')
        print('Demo passed: live CPU + heartbeat + large probe, SSE, CQEs, rotation, tail and download')
    finally:
        proc.send_signal(signal.SIGINT)
        try:
            _, err = proc.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            _, err = proc.communicate()
            raise AssertionError('server failed to shut down with an active SSE client')
        if live_stream:
            live_stream.close()
        assert proc.returncode == 0, err
