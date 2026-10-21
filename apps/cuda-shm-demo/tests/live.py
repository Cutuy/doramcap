"""Exercise a running viewer against real CUDA/Dora; no browser dependency."""
import json
import time
from urllib.request import Request, urlopen

import os
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parents[1]))
from generated import viewer_pb2 as pb, activity_pb2 as activity
BASE = os.environ.get('CUDA_VIEWER_URL', 'http://127.0.0.1:8768')

def get():
    with urlopen(BASE + '/api/state') as response:
        assert response.headers['Content-Type'] == 'application/x-protobuf'
        snapshot = pb.Snapshot.FromString(response.read())
        rows = []
        for envelope in snapshot.batches:
            batch = activity.Batch.FromString(envelope.payload)
            for record in batch.records:
                fields = {}
                for f in record.fields:
                    value = batch.strings[f.offset:f.offset+f.length].decode() if f.kind == 4 else bool(f.bits) if f.kind == 3 else f.bits
                    fields[batch.dictionary[f.name]] = value
                rows.append(dict(pid=batch.pid, batch=batch.sequence, time_ms=(batch.unix_origin_ns+record.monotonic_ns)/1e6,
                                 fields=fields, losses=dict(records=batch.dropped_records, fields=batch.dropped_fields,
                                                         spans=batch.rejected_spans, unregistered=batch.unregistered_records)))
        return dict(run=snapshot.run, state=snapshot.state, events=rows, log=snapshot.log, artifacts=list(snapshot.artifacts))

for mode, checksum in [('', False), ('', True), ('stall', False), ('duplicate', False)]:
    # Arrange
    config = dict(frames=24, slots=2, consumers=3, bytes=4097, failure=mode, checksum=checksum)
    request = Request(BASE + '/api/run', data=pb.RunConfig(**config).SerializeToString(),
                      headers={'Content-Type': 'application/x-protobuf', 'Origin': BASE})
    # Act
    with urlopen(request) as response:
        assert response.status == 202
    deadline = time.monotonic() + 75
    live = False
    while time.monotonic() < deadline:
        state = get()
        live |= state['state'] == 'running' and bool(state['events'])
        if state['state'] != 'running':
            break
        time.sleep(.05)
    # Assert
    assert state['state'] == 'passed', state['log']
    assert live, 'no live activity before run finished'
    events = state['events']
    assert len({e['pid'] for e in events}) == 4
    assert not any(any(e['losses'].values()) for e in events)
    assert len([a for a in state['artifacts'] if a.endswith('.mcap')]) == 4
    for event in events:
        assert 'time_ms' in event and 'batch' in event
    if not mode:
        stats = [e['fields'] for e in events if e['fields'].get('event') == 'stats'][-1]
        assert stats['published'] == stats['reclaimed'] == 24
        assert stats['pending'] > 0
        reclaimed = [e for e in events if e['fields'].get('event') == 'reclaimed']
        assert len(reclaimed) == 24
        for end in reclaimed:
            f = end['fields']
            related = [e for e in events if (e['fields'].get('slot'), e['fields'].get('generation')) == (f['slot'], f['generation'])]
            returns = [e for e in related if e['fields'].get('event') == 'returned_received']
            done = [e for e in related if e['fields'].get('event') == 'done_record_queued']
            assert len(returns) == len(done) == 3
            assert all(r['time_ms'] <= end['time_ms'] for r in returns)
        samples = [e['fields'] for e in events if e['fields'].get('event') == 'cpu_checksum']
        assert len(samples) == (72 if checksum else 0)
        assert all(s['checksum'] == s['expected'] and s['matches'] for s in samples)
    elif mode == 'stall':
        assert not any(e['fields'].get('event') == 'reclaimed' for e in events)
    else:
        assert 'duplicate return' in state['log']
    print(f"PASS mode={mode or 'normal'} checksum={checksum} records={len(events)} run={state['run']}", flush=True)
