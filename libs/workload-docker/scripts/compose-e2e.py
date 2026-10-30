#!/usr/bin/env python3
"""Opt-in native Docker tests. Build unprivileged; elevate only Docker and test binary."""
import argparse, json, os, re, shutil, subprocess, tempfile, time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--profile',default='ci');args=p.parse_args()
    output=args.output.absolute();output.mkdir(parents=True,exist_ok=False)
    main_checkout=Path(subprocess.check_output(['git','worktree','list','--porcelain'],cwd=ROOT,text=True).splitlines()[0].removeprefix('worktree '))
    credential=main_checkout/'.sudo-password'
    if not credential.is_file() or credential.stat().st_mode&0o777 != 0o600:raise SystemExit('Provision main .sudo-password locally, mode 0600')
    subprocess.run(['git','check-ignore','-q','.sudo-password'],cwd=main_checkout,check=True)
    def elevated(command,**kw):
        with credential.open('rb') as secret:return subprocess.run(['sudo','-S','-p','','--',*command],stdin=secret,check=True,**kw)
    subprocess.run(['cargo','build','--profile',args.profile,'--locked','-p','system-monitor','--features','bpf'],cwd=ROOT,check=True)
    built=subprocess.run(['cargo','test','--profile',args.profile,'--locked','-p','workload-docker','--test','docker','--no-run','--message-format=json'],cwd=ROOT,check=True,capture_output=True,text=True)
    executable=next(json.loads(line)['executable'] for line in built.stdout.splitlines() if json.loads(line).get('executable'))
    target=Path(json.loads(subprocess.check_output(['cargo','metadata','--no-deps','--format-version','1'],cwd=ROOT))['target_directory'])/('debug' if args.profile=='dev' else args.profile)
    prefix=f'cmp-{os.getpid()}-{int(time.time())}'
    image=f'doramcap-compose-test:{prefix}'
    with tempfile.TemporaryDirectory(prefix='compose-image-') as tmp:
        stage=Path(tmp);fs=stage/'rootfs';fs.mkdir()
        # Include a tiny local shell toolset for foreign test services; no registry pull.
        for source,dest in [(target/'system-monitor','system-monitor'),(Path('/bin/sh'),'bin/sh'),(Path('/bin/sleep'),'bin/sleep'),(Path('/bin/true'),'bin/true'),(Path('/bin/false'),'bin/false'),(Path('/bin/rm'),'bin/rm'),(Path('/bin/touch'),'bin/touch')]:
            destination=fs/dest;destination.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,destination)
            for line in subprocess.check_output(['ldd',str(source)],text=True).splitlines():
                match=re.search(r'(?:=>\s+)?(/\S+)',line)
                if match:
                    lib=Path(match.group(1));out=fs/lib.relative_to('/') if 'ld-linux' in lib.name else fs/'lib'/lib.name
                    out.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(lib,out)
        (fs/'seed-mod').write_text('original');(fs/'seed-delete').write_text('original')
        (fs/'recordings').mkdir();(stage/'Dockerfile').write_text('FROM scratch\nCOPY rootfs /\nENTRYPOINT ["/system-monitor"]\n')
        elevated(['docker','build','--network=none','-t',image,str(stage)])
    try:
        elevated(['env',f'DORAMCAP_TEST_IMAGE={image}',f'DORAMCAP_TEST_PREFIX={prefix}',f'DORAMCAP_TEST_OUTPUT={output}','DORAMCAP_TEST_SOCKET=/var/run/docker.sock',executable,'--ignored','--nocapture','--test-threads=1'])
    finally:
        # Test panic cleanup is scoped to this harness's exact generated run IDs.
        for suffix in ['normal0','normal1','partial','unhealthy','timeout','outsider','diff-failure','busy-overlay']:
            run=f'{prefix}-{suffix}'
            ids=elevated(['docker','ps','-aq','--filter',f'label=io.doramcap.run={run}'],capture_output=True,text=True).stdout.split()
            if ids:elevated(['docker','rm','-fv',*ids])
            for kind in ['network','volume']:
                ids=elevated(['docker',kind,'ls','-q','--filter',f'label=io.doramcap.run={run}'],capture_output=True,text=True).stdout.split()
                if ids:elevated(['docker',kind,'rm',*ids])
        outsider=elevated(['docker','ps','-aq','--filter',f'name=^/{prefix}-outsider-container$'],capture_output=True,text=True).stdout.split()
        if outsider:elevated(['docker','rm','-f',*outsider])
        # Failed evidence tests intentionally retain mounts; release only this
        # harness's overlay mountpoints after its containers have been removed.
        mountpoints = [line.split()[4] for line in Path('/proc/self/mountinfo').read_text().splitlines()]
        for mountpoint in reversed(mountpoints):
            path = Path(mountpoint)
            if path.is_relative_to(output) and path.name == 'merged' and 'overlays' in path.parts:
                elevated(['umount', str(path)])
        elevated(['docker','image','rm',image])
    print(f'PASS: multi-project native Compose workflow; evidence {output}')
if __name__=='__main__':main()
