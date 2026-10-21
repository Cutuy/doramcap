import importlib.util
from pathlib import Path
import tempfile
import sys
import struct
sys.path.insert(0, str(Path(__file__).parents[1]))
import unittest
spec = importlib.util.spec_from_file_location('viewer', Path(__file__).parents[1] / 'viewer.py')
viewer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(viewer)

class ViewerTests(unittest.TestCase):
    def test_reject_invalid_controls(self):
        # Arrange
        invalid = [{'frames': 0}, {'consumers': 9}, {'bytes': -1}, {'slots': True}, {'checksum': 'yes'}, {'failure': 'shell'}, {'frames': 4, 'slots': 4}]
        # Act / Assert
        for config in invalid:
            with self.subTest(config=config), self.assertRaises(ValueError):
                viewer.validate(config)

    def test_snapshot_reconnect_and_incremental_cursor(self):
        # Arrange
        experiment = viewer.Experiment('dora', Path(tempfile.gettempdir()))
        experiment.run_id = 'new'
        experiment.rows.extend([viewer.pb.ActivityBatch(id=1), viewer.pb.ActivityBatch(id=2)])
        experiment.serial = 2
        # Act
        reconnect = experiment.snapshot(100, 'old')
        incremental = experiment.snapshot(1, 'new')
        # Assert
        self.assertEqual(len(reconnect.batches), 2)
        self.assertEqual([r.id for r in incremental.batches], [2])
        self.assertEqual(incremental.cursor, 2)

    def test_partial_batches_and_invalid_lengths(self):
        # Arrange
        framed = struct.pack('<I', 3) + b'abc' + struct.pack('<I', 2) + b'xy'
        # Act / Assert
        for cut in range(len(framed) + 1):
            first, tail = viewer.split_frames(framed[:cut])
            second, tail = viewer.split_frames(tail + framed[cut:])
            self.assertEqual(first + second, [b'abc', b'xy'])
            self.assertEqual(tail, b'')
        with self.assertRaises(ValueError):
            viewer.split_frames(struct.pack('<I', 5 * 1024 * 1024))

    def test_idle_poll_and_log_once(self):
        # Arrange
        experiment = viewer.Experiment('dora', Path(tempfile.gettempdir()))
        experiment.run_id = 'a'
        experiment.state = 'passed'
        experiment.log = 'finished'
        # Act
        changed = experiment.snapshot(0, 'a', 'running')
        idle = experiment.snapshot(0, 'a', 'passed')
        # Assert
        self.assertEqual(changed.log, 'finished')
        self.assertIsNone(idle)

    def test_http_binary_contract_and_unchanged_response(self):
        from threading import Thread
        from urllib.request import Request, urlopen
        from urllib.error import HTTPError
        # Arrange
        experiment = viewer.Experiment('dora', Path(tempfile.gettempdir()))
        experiment.run_id = 'fixture'
        experiment.serial = 1
        experiment.rows.append(viewer.pb.ActivityBatch(id=1, role='producer', payload=b'opaque'))
        received = []
        experiment.start = received.append
        server = viewer.ThreadingHTTPServer(('127.0.0.1', 0), viewer.handler(experiment))
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        base = f'http://127.0.0.1:{server.server_port}'
        try:
            # Act
            with urlopen(base + '/api/state') as response:
                decoded = viewer.pb.Snapshot.FromString(response.read())
                content_type = response.headers['Content-Type']
            # Assert
            self.assertEqual(content_type, 'application/x-protobuf')
            self.assertEqual(decoded.batches[0].payload, b'opaque')
            # Act / Assert
            with urlopen(base + '/api/state?since=1&run=fixture&state=idle') as response:
                self.assertEqual(response.status, 204)
                self.assertEqual(response.read(), b'')
            config = viewer.pb.RunConfig(frames=24, slots=2, consumers=3, bytes=4097, checksum=True)
            request = Request(base + '/api/run', data=config.SerializeToString(),
                              headers={'Content-Type':'application/x-protobuf', 'Origin':base})
            with urlopen(request) as response:
                self.assertEqual(response.status, 202)
            self.assertEqual(received[0]['frames'], 24)
            self.assertTrue(received[0]['checksum'])
            request = Request(base + '/api/run', data=b'{}', headers={'Content-Type':'application/json', 'Origin':base})
            with self.assertRaises(HTTPError) as rejected:
                urlopen(request)
            self.assertEqual(rejected.exception.code, 403)
            self.assertIn('Protobuf', viewer.pb.Reply.FromString(rejected.exception.read()).error)
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

if __name__ == '__main__':
    unittest.main()
