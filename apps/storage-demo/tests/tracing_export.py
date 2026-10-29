"""Run with a storage-demo binary built using tracing-export."""
import os, subprocess, tempfile, json, urllib.request, time, signal, fcntl
from pathlib import Path
import argparse
args=argparse.ArgumentParser()
args.add_argument("--bin-dir",type=Path,required=True)
bins=args.parse_args().bin_dir.resolve()
with tempfile.TemporaryDirectory() as d:
 p=subprocess.Popen([str(bins/'storage-demo'),'--port','0','--dir',d+'/session',],env={**os.environ,'STORAGE_TRACE_STDERR':'1'},stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 try:
  fcntl.fcntl(p.stderr,fcntl.F_SETPIPE_SZ,4096)
  base='http://'+p.stdout.readline().split('http://')[1].split()[0]
  def state():
   with urllib.request.urlopen(base+'/api/state',timeout=3) as r:return json.load(r)
  def control(action,**kw):
   with urllib.request.urlopen(urllib.request.Request(base+'/api/control',json.dumps(dict(action=action,**kw)).encode(),{'Content-Type':'application/json'}),timeout=3) as r:assert r.status==202
  deadline=time.monotonic()+10
  while state()['status']!='live' and time.monotonic()<deadline:time.sleep(.05)
  control('probe_burst',count=256,intervalMs=0)
  while time.monotonic()<deadline:
   s=state()
   if s.get('probes',0)>=256:break
   time.sleep(.05)
  assert s['probes']==256,s
  assert s['ring']['completions']>0
  # stderr stays unread and fills; restart and recording must still work.
  old=s['session'];control('restart')
  while time.monotonic()<deadline:
   s=state()
   if s.get('session')!=old and s.get('heartbeats',0)>0:break
   time.sleep(.05)
  assert s.get('session')!=old and s.get('heartbeats',0)>0,s
  control('stop')
  while time.monotonic()<deadline:
   s=state()
   if s['status']=='sealed':break
   time.sleep(.05)
  assert s['status']=='sealed',s
  print('Blocked tracing stderr: 256 probes, restart, seal and live HTTP passed')
 finally:
  p.send_signal(signal.SIGINT)
  _,err=p.communicate(timeout=10)
  assert p.returncode==0,err
  assert 'storage_observation' in err
