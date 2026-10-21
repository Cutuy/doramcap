#!/usr/bin/env python3
"""Exercise real Dora transport, browser controls, hysteresis and optional sinks."""
import argparse
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import tempfile
import threading
import time
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--dora', required=True)
parser.add_argument('--bin-dir', type=Path, required=True)
parser.add_argument('--sinks', action='store_true', help='Require an all-feature watchdog build')
args = parser.parse_args()
bins = args.bin_dir.resolve()
received = []
class Collector(BaseHTTPRequestHandler):
    def do_POST(self):
        payload = self.rfile.read(int(self.headers['Content-Length']))
        if self.path == '/v1/logs':
            received.append(len(payload))
        self.send_response(200)
        self.send_header('Content-Type', 'application/x-protobuf')
        self.send_header('Content-Length', '0')
        self.end_headers()
    def log_message(self, *_):
        pass
collector = ThreadingHTTPServer(('127.0.0.1', 0), Collector)
threading.Thread(target=collector.serve_forever, daemon=True).start()
with socket.socket() as sock:
    sock.bind(('127.0.0.1', 0))
    port = sock.getsockname()[1]
base = f'http://127.0.0.1:{port}'
with tempfile.TemporaryDirectory(prefix='fault-dora-') as directory:
    root = Path(directory)
    config = root / 'watchdog.pbtxt'
    optional = ''
    if args.sinks:
        optional = f'''mcap {{enabled: true path: {json.dumps(str(root/'faults.mcap'))} flush_ms: 50}}
        otel {{enabled: true endpoint: "http://127.0.0.1:{collector.server_port}/v1/logs" timeout_ms: 500 service_name: "fault-demo"}}'''
    config.write_text('''app {id: "watchdog" config {[type.googleapis.com/doramcap.watchdog.v1.WatchdogConfig] {
    inputs: "source_a" inputs: "source_b" source_timeout_ms: 1000 snapshot_interval_ms: 100
    ''' + optional + '}}}')
    flow = {'nodes': [
        *[{'id': f'source_{name}', 'path': str(bins/'fault-demo-source'), 'args': name,
           'inputs': {'controls': 'dashboard/controls'}, 'outputs': ['reports']} for name in ['a', 'b']],
        {'id': 'watchdog', 'path': str(bins/'fault-watchdog'), 'args': str(config),
         'inputs': {'source_a': 'source_a/reports', 'source_b': 'source_b/reports'}, 'outputs': ['faults']},
        {'id': 'dashboard', 'path': str(bins/'fault-demo-dashboard'),
         'env': {'FAULT_DEMO_LISTEN': f'127.0.0.1:{port}'},
         'inputs': {'faults': 'watchdog/faults'}, 'outputs': ['controls']}]}
    descriptor = root/'flow.yml'
    descriptor.write_text(json.dumps(flow))
    def state():
        with urllib.request.urlopen(base+'/api/state', timeout=1) as response:
            return json.load(response)
    def wait(predicate, timeout=10):
        deadline = time.monotonic()+timeout
        last = None
        while time.monotonic() < deadline:
            try:
                last = state()
                if predicate(last):
                    return last
            except (OSError, ValueError):
                pass
            time.sleep(.025)
        raise AssertionError(f'condition timed out: {last}')
    def condition(value, source='a'):
        return next((c for c in value['conditions'] if c['component']=='demo' and c['resource']==source and c['code']=='operation'), {})
    def control(mode):
        request = urllib.request.Request(base+'/api/control', data=json.dumps({'source':'a','mode':mode}).encode(), headers={'Content-Type':'application/json'})
        with urllib.request.urlopen(request, timeout=2) as response:
            assert response.status == 202
    with (root/'dora.log').open('w+') as log:
        proc = subprocess.Popen([args.dora, 'run', str(descriptor), '--stop-after', '25s'], cwd=root, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            wait(lambda v: condition(v).get('health')=='HEALTH_HEALTHY')
            started = time.monotonic()
            control('fail')
            wait(lambda v: condition(v).get('health')=='HEALTH_ACTIVE')
            latency = time.monotonic()-started
            assert latency < 2, latency
            control('healthy')
            wait(lambda v: condition(v).get('health')=='HEALTH_RECOVERING')
            wait(lambda v: condition(v).get('health')=='HEALTH_HEALTHY')
            control('pause')
            wait(lambda v: condition(v).get('freshness')=='FRESHNESS_STALE')
            assert condition(state(), 'b')['freshness']=='FRESHNESS_FRESH'
            control('flood')
            wait(lambda v: condition(v).get('health')=='HEALTH_ACTIVE')
            control('healthy')
            wait(lambda v: condition(v).get('health')=='HEALTH_HEALTHY')
            control('event')
            wait(lambda v: any(c['resource']=='a' and c['code']=='event' and int(c['failures'] or '0')>0 for c in v['conditions']))
            proc.wait(timeout=30)
            if proc.returncode:
                raise AssertionError(f'Dora exit {proc.returncode}')
            if args.sinks:
                assert received, 'no OTLP requests'
                subprocess.run([str(bins/'examples/fault_readback'),str(root/'faults.mcap')],check=True)
            print(json.dumps({'dora':'passed','control_to_active_ms':round(latency*1000,1),'otlp_requests':len(received),'mcap':args.sinks}))
        except BaseException:
            os.killpg(proc.pid, signal.SIGTERM) if proc.poll() is None else None
            try:
                proc.wait(timeout=8)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait()
            log.seek(0)
            print(log.read()[-12000:])
            raise
collector.shutdown()
