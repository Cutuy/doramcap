#!/usr/bin/env python3
"""Compare embedded snapshots with the same messages recorded through a real Dora flow."""
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
parser.add_argument('--backend', choices=('bpf',), default='bpf')
args = parser.parse_args()
bin_dir = args.bin_dir.resolve()
with tempfile.TemporaryDirectory(prefix='doramcap-recording-e2e-') as directory:
    root = Path(directory)
    embedded, received = root / 'embedded.mcap', root / 'received.mcap'
    source = Path(__file__).resolve().parents[3]
    monitor_config = source / 'apps/system-monitor/config/recording.pbtxt'
    recorder_config = root / 'recorder.pbtxt'
    recorder_config.write_text(f"""output_path: "{received}"
report_path: "{root / 'received.report.pbtxt'}"
input {{ input: "snapshots" topic: "doramcap/cpuacct" message_encoding: "protobuf" timestamp_key: "capture_unix_ns" sequence_key: "sequence" }}
""")
    # JSON is a YAML-compatible descriptor and safely quotes filesystem paths.
    flow = {'nodes': [
        {'id': 'monitor', 'path': str(bin_dir / 'system-monitor'),
         'args': f'--backend bpf --samples 24 --records 4096 --dora --record {embedded} --record-config {monitor_config}',
         'outputs': ['snapshots', 'dictionary']},
        {'id': 'recorder', 'path': str(bin_dir / 'mcap-recorder'), 'args': f'--config {recorder_config}',
         'inputs': {'snapshots': {'source': 'monitor/snapshots', 'queue_size': 64}}}]}
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
    subprocess.run([str(bin_dir / 'examples/compare_recordings'), str(embedded), str(received), 'doramcap/cpuacct', '24'], check=True)
