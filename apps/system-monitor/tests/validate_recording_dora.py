#!/usr/bin/env python3
"""Compare embedded recording with messages received through a real Dora flow."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--dora', required=True)
parser.add_argument('--bin-dir', type=Path, required=True)
args = parser.parse_args()
bin_dir = args.bin_dir.resolve()
with tempfile.TemporaryDirectory(prefix='doramcap-recording-e2e-') as directory:
    root = Path(directory)
    embedded, received = root / 'embedded.mcap', root / 'received.mcap'
    # JSON is a YAML-compatible descriptor and safely quotes filesystem paths.
    flow = {'nodes': [
        {'id': 'monitor', 'path': str(bin_dir / 'system-monitor'),
         'args': f'--backend bpf --samples 24 --records 4096 --dora --record {embedded}',
         'outputs': ['snapshots', 'dictionary']},
        {'id': 'recorder', 'path': str(bin_dir / 'mcap-recorder'), 'args': str(received),
         'inputs': {'snapshots': {'source': 'monitor/snapshots', 'queue_size': 64},
                    'dictionary': {'source': 'monitor/dictionary', 'queue_size': 64}}}]}
    descriptor = root / 'flow.yml'
    descriptor.write_text(json.dumps(flow))
    proc = subprocess.Popen([args.dora, 'run', str(descriptor)], cwd=root,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            text=True, start_new_session=True)
    try:
        output, _ = proc.communicate(timeout=45)
    except subprocess.TimeoutExpired:
        os.killpg(proc.pid, signal.SIGTERM)
        try:
            output, _ = proc.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL)
            output, _ = proc.communicate()
        print(output)
        raise SystemExit('Dora validation timed out')
    if proc.returncode:
        print(output)
        raise SystemExit(proc.returncode)
    subprocess.run([str(bin_dir / 'examples/compare_recordings'), str(embedded), str(received)], check=True)
