#!/usr/bin/env python3
"""Real CUDA + Dora content regression matrix; fails if CUDA is unavailable."""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile

parser = argparse.ArgumentParser(__doc__)
parser.add_argument('--dora', default='dora')
parser.add_argument('--logs', type=Path)
args = parser.parse_args()
logs = args.logs or Path(tempfile.mkdtemp(prefix='cuda-content-e2e-'))
logs.mkdir(parents=True, exist_ok=True)
runner = Path(__file__).resolve().parents[1] / 'run.py'
cases = [
    ('one-byte', 1, 1, 0, True, None),
    ('partial-cuda-block', 257, 2, 1, True, None),
    ('cross-page', 4097, 2, 0xffffffff, True, None),
    ('large-tail', 65537, 2, 0xdeadbeef, True, None),
    ('gpu-only-async', 4097, 2, 0x98abcdef, False, None),
    ('gpu-rejects-corruption', 4097, 2, 0xdeadbeef, False, 'corrupt-gpu'),
    ('cpu-rejects-corruption', 4097, 2, 0xdeadbeef, True, 'corrupt-cpu'),
]
for name, size, slots, seed, checksum, failure in cases:
    # Arrange: producer and three independent consumer processes share fixed slots.
    command = [sys.executable, str(runner), '--dora', args.dora, '--frames', '8',
               '--consumers', '3', '--slots', str(slots), '--bytes', str(size), '--seed', str(seed)]
    if checksum:
        command.append('--checksum')
    if failure:
        command.extend(['--failure', failure])
    env = dict(os.environ)
    env.pop('CUDA_SHM_CHECKSUM', None)
    env.pop('CUDA_SHM_TRACE_DIR', None)
    # Act
    result = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=80, env=env)
    path = logs / f'{name}.log'
    path.write_text(result.stdout)
    # Assert: runner requires full verification/reuse or the specific corruption error.
    assert result.returncode == 0, f'{name} failed: {path}\n{result.stdout[-4000:]}'
    assert 'PASS:' in result.stdout, f'missing validation result: {path}'
    print(f'PASS {name}: seed={seed:#x}, bytes={size}, slots={slots}; {path}', flush=True)
