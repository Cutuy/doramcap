#!/usr/bin/env python3
"""Launch two app processes against five dynamic Dora nodes; stop only this flow."""
import argparse, json, os, shutil, signal, subprocess, tempfile, time, uuid
from pathlib import Path
from urllib.request import urlopen

APP = Path(__file__).resolve().parent
ROOT = APP.parents[1]

def descriptor(prefix):
    lines = ['nodes:']
    for name in ('camera', 'lidar', 'detect', 'control', 'monitor'):
        lines += [f'  - id: {prefix}-{name}', '    path: dynamic']
        inputs = {}
        if name == 'detect': inputs = {'camera': f'{prefix}-camera/data', 'lidar': f'{prefix}-lidar/data'}
        if name == 'control': inputs = {'objects': f'{prefix}-detect/data'}
        if name == 'monitor':
            inputs = {n: f'{prefix}-{n}/telemetry' for n in ('camera','lidar','detect','control')}
            inputs['keepalive'] = 'dora/timer/millis/1000'
        if inputs:
            lines += ['    inputs:']
            for key, source in inputs.items():
                lines += [f'      {key}:', f'        source: {source}', '        queue_size: 512']
        if name != 'monitor': lines += ['    outputs: [data, telemetry]']
    return '\n'.join(lines)+'\n'

def snapshot(port):
    with urlopen(f'http://127.0.0.1:{port}/api/summary', timeout=3) as response: return json.load(response)

def validate(s, dropped=False):
    assert set(s['nodes']) == {'camera','lidar','detect','control'}, s['nodes']
    pids = {n['pid'] for n in s['nodes'].values()}
    assert len(pids) == 1 and s['monitor_pid'] not in pids
    assert s['complete'] > 20 and s['activity_batches'] > 0 and s['closed_spans'] > 30, {k:v for k,v in s.items() if k not in ('results','patches')}
    assert s['capture_losses'] == 0 and s['rejected'] == 0, s
    assert all(n['activity_queue_drops'] == 0 for n in s['nodes'].values())
    assert s['records'] <= 4*64 and s['replay_bytes'] <= 8*1024*1024 and s['replay_frames'] <= 4096
    complete = [r for r in s['results'] if r['status'] == 1]
    assert {r['measurement'] for r in complete} == {'camera_to_detection','camera_to_control','lidar_age_at_control','detection_duration'}
    for r in complete:
        assert r['start']['clock'] == r['end']['clock']
        assert int(r['duration_ns']) == int(r['end']['timestamp_ns'])-int(r['start']['timestamp_ns']) >= 0
    lidar = [r['start']['generation']['generation'] for r in complete if r['measurement']=='lidar_age_at_control']
    assert len(set(lidar)) < len(lidar), 'expected reused lidar frames'
    if dropped: assert s['incomplete'] > 0 and any(n['simulated_report_drops'] > 0 for n in s['nodes'].values())
    else: assert s['incomplete'] == 0, s['incomplete']
    print('PASS: four nodes / one pipeline PID; separate monitor; exact latency pairs; reused lidar; activity; bounds', flush=True)

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--dora', default=shutil.which('dora'))
    parser.add_argument('--binary', type=Path, default=ROOT/'target/debug/dataflow-tracing')
    parser.add_argument('--assets', type=Path, default=APP/'frontend/dist')
    parser.add_argument('--port', type=int, default=8090)
    parser.add_argument('--duration', type=int, default=3600)
    parser.add_argument('--drop-every', type=int, default=0)
    parser.add_argument('--report-delay-ms', type=int, default=0)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--output', type=Path)
    args=parser.parse_args()
    if not args.dora: parser.error('pass --dora /path/to/dora (version 1.1.0)')
    prefix='prov-'+uuid.uuid4().hex[:10]
    env={**os.environ, 'PROVENANCE_PREFIX':prefix}
    subprocess.run([args.dora,'up'],check=True,timeout=30)
    processes=[]
    with tempfile.TemporaryDirectory(prefix='dataflow-tracing-') as tmp:
        flow=Path(tmp)/'flow.yml';flow.write_text(descriptor(prefix))
        logs=[]
        started=False
        try:
            subprocess.run([args.dora,'start',str(flow),'--name',prefix,'--detach'],check=True,timeout=30)
            started=True
            for role, options in [('monitor',['--port',str(args.port),'--assets',str(args.assets.resolve()),'--measurements',str(APP/'configs/measurements.pbtxt')]),
                                  ('pipeline',['--duration',str(args.duration),'--drop-every',str(args.drop_every),'--report-delay-ms',str(args.report_delay_ms)])]:
                log=open(Path(tmp)/(role+'.log'),'w+');logs.append(log)
                processes.append(subprocess.Popen([str(args.binary.resolve()),role,*options],env=env,stdout=log,stderr=subprocess.STDOUT))
            print(f'Dataflow Tracing: http://127.0.0.1:{args.port} · logs {tmp}',flush=True)
            deadline=time.monotonic()+args.duration+30
            while processes[1].poll() is None:
                if processes[0].poll() is not None: raise RuntimeError('monitor exited early')
                if time.monotonic()>deadline: raise TimeoutError('pipeline did not stop')
                time.sleep(.2)
            if processes[1].returncode: raise RuntimeError('pipeline failed')
            time.sleep(1)
            s=snapshot(args.port)
            if args.output: args.output.write_text(json.dumps(s,indent=2))
            if args.check: validate(s,args.drop_every>0)
        except KeyboardInterrupt:
            pass
        except Exception:
            for log in logs:
                log.flush();log.seek(0);print(log.read()[-16000:])
            raise
        finally:
            if started: subprocess.run([args.dora,'stop','--name',prefix],timeout=20,check=False,stdout=subprocess.DEVNULL)
            for process in processes:
                if process.poll() is None:
                    process.send_signal(signal.SIGINT)
                    try: process.wait(timeout=8)
                    except subprocess.TimeoutExpired: process.kill();process.wait()
            for log in logs: log.close()
if __name__=='__main__': main()
