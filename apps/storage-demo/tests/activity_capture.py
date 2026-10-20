"""Exercise source-time activity capture; build with the activity-tracing feature."""
import argparse, json, re, select, signal, subprocess, tempfile, time, urllib.request
from pathlib import Path
p = argparse.ArgumentParser()
p.add_argument('--bin-dir', type=Path, required=True)
bins = p.parse_args().bin_dir.resolve()
with tempfile.TemporaryDirectory(prefix='storage-activity-') as directory:
    capture = str(Path(directory) / 'activity.mcap')
    proc = subprocess.Popen([str(bins/'storage-demo'), '--dir', directory+'/session',
        '--port', '0', '--monitor', str(bins/'system-monitor'), '--activity', capture],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        assert select.select([proc.stdout], [], [], 15)[0], 'startup timeout'
        base = 'http://' + proc.stdout.readline().split('http://')[1].split()[0]
        def wait(predicate):
            deadline = time.monotonic()+15
            while time.monotonic()<deadline:
                with urllib.request.urlopen(base+'/api/state', timeout=3) as r: state=json.load(r)
                assert state['status']!='error', state
                if predicate(state): return state
                time.sleep(.05)
            raise AssertionError('state timeout')
        def control(action, **kwargs):
            request=urllib.request.Request(base+'/api/control', json.dumps(dict(action=action, **kwargs)).encode(), {'Content-Type':'application/json'})
            with urllib.request.urlopen(request, timeout=3) as r: assert r.status==202
        wait(lambda s:s['status']=='live' and s.get('cpu',0)>0)
        control('managed_large', sizeMiB=1)
        wait(lambda s:any(r['action']=='managed_large' and r['status']=='accepted' for r in s['exercises']['receipts']))
        control('sealed_slab')
        wait(lambda s:any(r['action']=='sealed_slab' and r['status']=='accepted' for r in s['exercises']['receipts']))
        control('stop')
        wait(lambda s:s['status']=='sealed')
    finally:
        proc.send_signal(signal.SIGINT)
        _, err=proc.communicate(timeout=15)
        assert proc.returncode==0, err
    assert 'storage activity capture:' in err, err
    assert not re.search(r'(?:records|fields|spans|unregistered|encoded_batches|dropped): [1-9]', err), err
    subprocess.run([str(bins/'examples'/'validate_activity'), capture], check=True)
