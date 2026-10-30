#!/usr/bin/env python3
"""Privileged integration test; creates/removes only its own cgroup subtree.

Build system-monitor with --features bpf. Run this script via sudo, passing
--monitor /absolute/path/to/system-monitor. No persistent configuration changes.
"""
import argparse
import json

import os
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'libs/telemetry/protocol/python'))
from wire import framed


def usage(path):
    return int(dict(line.split() for line in (path / 'cpu.stat').read_text().splitlines())['usage_usec']) * 1000


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--monitor', required=True, type=Path)
    args = parser.parse_args()
    assert os.geteuid() == 0, 'native validation requires privilege'
    # The approved worker launcher runs this aggregate native suite. Build remains unprivileged.
    policy_suite = Path(__file__).resolve().parents[3] / 'libs/resources/resource-policy/tests/validate_native.py'
    if policy_suite.exists():
        policy_binary = args.monitor.resolve().parent / 'examples/native_policy'
        subprocess.run([sys.executable, str(policy_suite), '--binary', str(policy_binary), '--enable-root-cpuset'], check=True, timeout=90)
    supervisor_suite = Path(__file__).resolve().parents[3] / 'libs/workload-supervisor/tests/validate_native.py'
    if supervisor_suite.exists():
        import runpy
        runpy.run_path(str(supervisor_suite))['validate'](args.monitor)
    cpus = sorted(os.sched_getaffinity(0))
    assert len(cpus) >= 2, 'two available CPUs required'
    root = Path('/sys/fs/cgroup') / f'doramcap-validation-{os.getpid()}'
    workers = []
    monitor = None
    concurrent_monitor = None
    groups = [root, root / 'a', root / 'b']
    root.mkdir()  # fail if it exists; never adopt another workload's cgroup
    try:
        for group in groups[1:]:
            group.mkdir()
        identities = [group.stat().st_ino for group in groups]
        baseline = [usage(group) for group in groups]
        with tempfile.TemporaryDirectory(prefix='doramcap-native-') as directory:
            output = Path(directory) / 'snapshots.pb'
            monitor = subprocess.Popen([str(args.monitor), '--backend', 'bpf', '--samples', '65',
                                        '--records', '16384', '--output', str(output)],
                                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
            concurrent_output = Path(directory) / 'concurrent.pb'
            concurrent_monitor = subprocess.Popen(
                [str(args.monitor), '--backend', 'bpf', '--samples', '80',
                 '--records', '16384', '--output', str(concurrent_output)],
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
            time.sleep(0.8)
            for process in (monitor, concurrent_monitor):
                if process.poll() is not None:
                    raise AssertionError('monitor failed before workload start: ' + process.communicate()[1])
            for group, cpu in zip(groups[1:], cpus[:2]):
                worker = subprocess.Popen([sys.executable, '-c',
                    'import os,sys,time; from pathlib import Path; '
                    'os.sched_setaffinity(0,{int(sys.argv[2])}); '
                    'Path(sys.argv[1],"cgroup.procs").write_text(str(os.getpid())); '
                    'end=time.monotonic()+3; '\
                    '\nwhile time.monotonic()<end: pass', str(group), str(cpu)])
                workers.append(worker)
            for worker in workers:
                assert worker.wait(timeout=10) == 0
            expected = [usage(group) - before for group, before in zip(groups, baseline)]
            _, stderr = monitor.communicate(timeout=15)
            assert monitor.returncode == 0, stderr
            assert concurrent_monitor.poll() is None, 'concurrent monitor stopped when the first monitor exited'
            _, concurrent_stderr = concurrent_monitor.communicate(timeout=15)
            assert concurrent_monitor.returncode == 0, concurrent_stderr
            concurrent_batches = framed(concurrent_output.read_bytes())
            assert len(concurrent_batches) == 80
            assert [b['sequence'] for b in concurrent_batches] == list(range(80))
            assert all(b['source'] == 'bpf' and b['capacity_overflow'] == 0 and b['dropped_batches'] == 0 for b in concurrent_batches)
            assert all(not (r['flags'] & 64) for b in concurrent_batches for r in b['records']), 'concurrent native read error'
            for group_id in identities:
                first = {r['cpu']: r['runtime_ns'] for r in concurrent_batches[0]['records'] if r['cgroup_id'] == group_id}
                last = {r['cpu']: r['runtime_ns'] for r in concurrent_batches[-1]['records'] if r['cgroup_id'] == group_id}
                assert first and first.keys() == last.keys(), 'concurrent monitor missing cgroup/core rows'
                assert sum(last[cpu] - value for cpu, value in first.items()) > 100_000_000
            batches = framed(output.read_bytes())
            assert len(batches) == 65 and [b['sequence'] for b in batches] == list(range(65))
            assert all(b['source'] == 'bpf' and b['capacity_overflow'] == 0 and b['dropped_batches'] == 0 for b in batches)
            assert all(not (r['flags'] & 64) for b in batches for r in b['records']), 'native read error'
            observed = []
            for group_id, reference in zip(identities, expected):
                first = {r['cpu']: r for r in batches[0]['records'] if r['cgroup_id'] == group_id}
                last = {r['cpu']: r for r in batches[-1]['records'] if r['cgroup_id'] == group_id}
                assert first and first.keys() == last.keys(), 'missing cgroup/core rows'
                assert all(r['flags'] & 1 for r in last.values()), 'unavailable runtime'
                delta = {cpu: last[cpu]['runtime_ns'] - row['runtime_ns'] for cpu, row in first.items()}
                total = sum(delta.values())  # validation of one group's per-core data, never child rollup
                assert reference > 100_000_000
                assert abs(total-reference) <= max(2_000_000, reference * .01), (group_id,total,reference)
                observed.append({'id':group_id,'runtime_ns':total,'cpu_stat_ns':reference,'per_core_ns':delta})
                previous = None
                for batch in batches:
                    current = {r['cpu']:r['runtime_ns'] for r in batch['records'] if r['cgroup_id']==group_id}
                    if previous is not None:
                        assert all(current[c]>=n for c,n in previous.items()), 'counter regression'
                    previous = current
            for index, cpu in enumerate(cpus[:2], 1):
                assert observed[index]['per_core_ns'][cpu] >= observed[index]['runtime_ns'] * .99
                rows = [r for r in batches[-1]['records'] if r['cgroup_id'] == identities[index]]
                assert all(r['parent_id'] == identities[0] for r in rows)
            durations = [(b['monotonic_end_ns']-b['monotonic_start_ns'])/1e6 for b in batches]
            intervals = [(b['monotonic_start_ns']-a['monotonic_start_ns'])/1e6 for a,b in zip(batches,batches[1:])]
            assert 90 <= statistics.median(intervals) <= 110
            assert all(b['monotonic_end_ns']>=b['monotonic_start_ns'] for b in batches)
            # Independently exercise bounded native result capacity.
            tiny = subprocess.run([str(args.monitor),'--backend','bpf','--samples','2','--records','1'],
                                  capture_output=True,timeout=10)
            assert tiny.returncode == 0, tiny.stderr
            small = framed(tiny.stdout)
            assert len(small)==2 and all(len(b['records'])==1 and b['capacity_overflow']>0 for b in small)
            print(json.dumps({'result':'PASS','kernel':os.uname().release,'architecture':os.uname().machine,
                'samples':len(batches),'rows_per_snapshot':len(batches[-1]['records']),
                'collection_ms_median':statistics.median(durations),'collection_ms_max':max(durations),
                'interval_ms_median':statistics.median(intervals),'interval_ms_max':max(intervals),
                'native_capacity_overflow_checked':True,'concurrent_monitor_samples':len(concurrent_batches),'groups':observed},indent=2))
    finally:
        for worker in workers:
            if worker.poll() is None:
                worker.kill()
            worker.wait()
        if monitor is not None and monitor.poll() is None:
            monitor.terminate()
            monitor.communicate(timeout=5)
        if concurrent_monitor is not None and concurrent_monitor.poll() is None:
            concurrent_monitor.terminate()
            concurrent_monitor.communicate(timeout=5)
        for group in reversed(groups):
            if group.exists():
                group.rmdir()


if __name__ == '__main__':
    main()
