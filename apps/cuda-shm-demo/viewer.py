#!/usr/bin/env python3
"""Local live CUDA/Dora experiment viewer. Trace artifacts remain in --output."""
import argparse
from collections import deque
import struct
from generated import viewer_pb2 as pb
from google.protobuf.message import DecodeError
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import os
import subprocess
import sys
import threading
import time
from urllib.parse import urlparse, parse_qs
import uuid

ROOT = Path(__file__).resolve().parent


def split_frames(data):
    """Retain incomplete writes; reject corrupt lengths before buffering a payload."""
    frames, offset = [], 0
    while len(data) - offset >= 4:
        size = struct.unpack_from('<I', data, offset)[0]
        if not 0 < size <= 4 * 1024 * 1024:
            raise ValueError('invalid activity batch size')
        if len(data) - offset < 4 + size:
            break
        frames.append(data[offset + 4:offset + 4 + size])
        offset += 4 + size
    return frames, data[offset:]


def validate(config):
    if not isinstance(config, dict):
        raise ValueError("configuration must be an object")
    out = {}
    for name, default, low, high in [('frames', 120, 4, 2000), ('slots', 2, 1, 16),
                                     ('consumers', 3, 1, 8), ('bytes', 4097, 1, 1048576)]:
        value = config.get(name, default)
        if type(value) is not int or not low <= value <= high:
            raise ValueError(f'{name} must be an integer in {low}..{high}')
        out[name] = value
    if out['frames'] <= out['slots']:
        raise ValueError('frames must exceed slots to demonstrate reuse')
    out['failure'] = config.get('failure', '')
    if out['failure'] not in ('', 'stall', 'duplicate'):
        raise ValueError('invalid failure mode')
    out['checksum'] = config.get('checksum', False)
    if type(out['checksum']) is not bool:
        raise ValueError('checksum must be boolean')
    return out


class Experiment:
    def __init__(self, dora, output):
        self.dora, self.output = dora, output
        self.lock = threading.Lock()
        self.rows = deque()
        self.history_bytes = 0
        self.serial = 0
        self.state = 'idle'
        self.run_id = ''
        self.log = ''
        self.config = {}
        self.directory = None
        self.proc = None

    def start(self, config):
        config = validate(config)
        with self.lock:
            if self.state == 'running':
                raise ValueError('an experiment is already running')
            self.run_id = uuid.uuid4().hex
            self.directory = self.output / self.run_id
            self.directory.mkdir(parents=True)
            self.rows.clear()
            self.history_bytes = 0
            self.serial = 0
            self.log = ''
            self.config = config
            self.state = 'running'
            threading.Thread(target=self.worker, daemon=True).start()

    def worker(self):
        positions, partial = {}, {}
        def ingest():
            for path in self.directory.glob('*.activity.pb'):
                with path.open('rb') as file:
                    file.seek(positions.get(path, 0))
                    chunk = file.read()
                    positions[path] = file.tell()
                frames, partial[path] = split_frames(partial.get(path, b'') + chunk)
                for payload in frames:
                    with self.lock:
                        self.serial += 1
                        self.rows.append(pb.ActivityBatch(id=self.serial, role=path.name.split('-')[0], payload=payload))
                        self.history_bytes += len(payload)
                        while self.history_bytes > 4 * 1024 * 1024 or len(self.rows) > 20000:
                            self.history_bytes -= len(self.rows.popleft().payload)
        try:
            command = [sys.executable, str(ROOT / 'run.py'), '--dora', self.dora]
            for key in ('frames', 'slots', 'consumers', 'bytes'):
                command.extend(['--' + key, str(self.config[key])])
            if self.config['failure']:
                command.extend(['--failure', self.config['failure']])
            env = dict(os.environ, CUDA_SHM_TRACE_DIR=str(self.directory))
            env.pop('CUDA_SHM_CHECKSUM', None)
            if self.config['checksum']:
                env['CUDA_SHM_CHECKSUM'] = '1'
            with (self.directory / 'run.log').open('w') as log:
                self.proc = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT, env=env)
                while self.proc.poll() is None:
                    ingest()
                    time.sleep(.05)
                ingest()
                if any(partial.values()):
                    raise ValueError('truncated final activity batch')
            with self.lock:
                self.log = (self.directory / 'run.log').read_text()[-100000:]
                self.state = 'passed' if self.proc.returncode == 0 else 'failed'
        except Exception as error:
            with self.lock:
                self.log = str(error)
                self.state = 'failed'

    def snapshot(self, since, run_id, known_state=''):
        with self.lock:
            reset = run_id != self.run_id
            if not reset and since == self.serial and known_state == self.state:
                return None
            if reset:
                since = 0
            result = pb.Snapshot(version=1, run=self.run_id, state=self.state,
                                 config=pb.RunConfig(**self.config), cursor=self.serial,
                                 oldest=self.rows[0].id if self.rows else 0,
                                 batches=[r for r in self.rows if r.id > since],
                                 artifacts=sorted(p.name for p in self.directory.iterdir()) if self.directory else [])
            if reset or known_state != self.state:
                result.log = self.log
            return result


def handler(experiment):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def respond(self, code, data, content='application/x-protobuf'):
            self.send_response(code)
            self.send_header('Content-Type', content)
            self.send_header('Content-Length', str(len(data)))
            self.send_header('Cache-Control', 'no-store')
            self.send_header('X-Content-Type-Options', 'nosniff')
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            url = urlparse(self.path)
            if url.path == '/api/state':
                query = parse_qs(url.query)
                try:
                    state = experiment.snapshot(int(query.get('since', ['0'])[0]), query.get('run', [''])[0], query.get('state', [''])[0])
                    self.respond(204 if state is None else 200, state.SerializeToString() if state is not None else b'')
                except ValueError:
                    self.respond(400, pb.Reply().SerializeToString())
            elif url.path.startswith('/artifact/'):
                name = url.path.removeprefix('/artifact/')
                with experiment.lock:
                    directory = experiment.directory
                if directory and name == Path(name).name and name.endswith(('.mcap', '.pb', '.log')) and (directory / name).is_file():
                    self.respond(200, (directory / name).read_bytes(), 'application/octet-stream')
                else:
                    self.respond(404, pb.Reply().SerializeToString())
            else:
                name = 'index.html' if url.path == '/' else url.path.lstrip('/')
                base = (ROOT / 'frontend/dist').resolve()
                path = (base / name).resolve()
                if path.is_relative_to(base) and path.is_file():
                    self.respond(200, path.read_bytes(), 'text/html' if path.suffix == '.html' else 'text/javascript' if path.suffix == '.js' else 'text/css')
                else:
                    self.respond(404, pb.Reply(error='not found').SerializeToString())

        def do_POST(self):
            # Browser-triggered GPU work is restricted to this local origin.
            if self.headers.get('Origin') != f'http://{self.headers.get("Host")}' or self.headers.get('Content-Type') != 'application/x-protobuf':
                self.respond(403, pb.Reply(error='same-origin Protobuf required').SerializeToString())
                return
            if self.path != '/api/run':
                self.respond(404, pb.Reply().SerializeToString())
                return
            try:
                length = int(self.headers.get('Content-Length', '0'))
                if not 0 < length <= 4096:
                    raise ValueError('invalid request length')
                config = pb.RunConfig.FromString(self.rfile.read(length))
                experiment.start({f.name: getattr(config, f.name) for f in config.DESCRIPTOR.fields})
                self.respond(202, pb.Reply().SerializeToString())
            except (ValueError, TypeError, DecodeError) as error:
                self.respond(400, pb.Reply(error=str(error)).SerializeToString())
    return Handler


if __name__ == '__main__':
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--port', type=int, default=8768)
    parser.add_argument('--dora', default='dora')
    parser.add_argument('--output', type=Path, default=Path('/tmp/cuda-shm-observations'))
    args = parser.parse_args()
    experiment = Experiment(args.dora, args.output.resolve())
    server = ThreadingHTTPServer(('127.0.0.1', args.port), handler(experiment))
    print(f'CUDA SHM viewer: http://127.0.0.1:{args.port}', flush=True)
    server.serve_forever()
