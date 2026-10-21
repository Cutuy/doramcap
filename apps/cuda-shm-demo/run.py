#!/usr/bin/env python3
"""Run and verify real Dora SPMC nodes; requires CUDA and Dora CLI 1.1.0."""
import argparse
import json
import os
from pathlib import Path
import re
import shlex
import signal
import subprocess
import tempfile

p = argparse.ArgumentParser(__doc__)
p.add_argument('--dora', default='dora')
p.add_argument('--consumers', type=int, default=3)
p.add_argument('--frames', type=int, default=12)
p.add_argument('--slots', type=int, default=2)
p.add_argument('--bytes', type=int, default=4096)
p.add_argument('--failure', choices=['stall', 'duplicate'])
p.add_argument('--sanitizer')
a = p.parse_args()
root = Path(__file__).resolve().parents[2]
binary = root / 'target/debug/cuda-shm-demo'
with tempfile.TemporaryDirectory(prefix='cuda-dora-flow-') as tmp:
    executable = str(binary)
    if a.sanitizer:
        wrapper = Path(tmp) / 'checked'
        wrapper.write_text('#!/bin/sh\nexec ' + shlex.join([a.sanitizer, '--tool', 'memcheck', '--error-exitcode', '99', str(binary)]) + ' "$@"\n')
        wrapper.chmod(0o700)
        executable = str(wrapper)
    nodes = [{'id': 'producer', 'path': executable,
              'args': f'producer --consumers {a.consumers} --frames {a.frames} --slots {a.slots} --bytes {a.bytes} --timeout {5 if a.failure else 40}',
              'inputs': {f'consumer{i}': {'source': f'consumer{i}/returns', 'queue_size': 128} for i in range(a.consumers)},
              'outputs': ['frames']}]
    for i in range(a.consumers):
        extra = f' --{a.failure}' if a.failure and i == 0 else ''
        nodes.append({'id': f'consumer{i}', 'path': executable,
                      'args': f'consumer --id {i}' + extra,
                      'inputs': {'frames': {'source': 'producer/frames', 'queue_size': 128}}, 'outputs': ['returns']})
    flow = Path(tmp) / 'flow.yml'
    flow.write_text(json.dumps({'nodes': nodes}))
    proc = subprocess.Popen([a.dora, 'run', str(flow)], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, start_new_session=True)
    try:
        output, _ = proc.communicate(timeout=65)
    except subprocess.TimeoutExpired:
        os.killpg(proc.pid, signal.SIGKILL)
        output, _ = proc.communicate()
        print(output)
        raise SystemExit('Dora did not terminate')
    output = re.sub(r'\x1b\[[0-9;]*m', '', output)
    print(output)
    if a.failure:
        expected = 'TIMEOUT: no unsafe reuse' if a.failure == 'stall' else 'duplicate return'
        assert proc.returncode != 0 and expected in output, 'expected deterministic rejection missing'
        if a.failure == 'stall':
            match = re.search(r'TIMEOUT: no unsafe reuse; published=(\d+).*reclaimed: (\d+)', output)
            assert match and int(match[1]) == a.slots and int(match[2]) == 0
    else:
        assert proc.returncode == 0, f'Dora exited {proc.returncode}'
        producer = [json.loads(line.split('PRODUCER_SUMMARY ', 1)[1]) for line in output.splitlines() if 'PRODUCER_SUMMARY ' in line]
        consumers = [json.loads(line.split('CONSUMER_SUMMARY ', 1)[1]) for line in output.splitlines() if 'CONSUMER_SUMMARY ' in line]
        assert len(producer) == 1 and len(consumers) == a.consumers
        assert producer[0]['stats']['reclaimed'] == a.frames
        assert producer[0]['stats']['published_pending'] > 0
        assert all(c['frames'] == a.frames and c['gpu_verified'] for c in consumers)
        assert all(c['max_generation'] > 1 for c in consumers)
    print('PASS:', a.failure or 'SPMC GPU payload + asynchronous publication + reuse')
