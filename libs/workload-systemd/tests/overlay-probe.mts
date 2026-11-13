// Build-free host qualification. Elevate only this test executable; all units are runtime-only.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, readlinkSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

function command(program: string, args: string[]): string {
  const result = spawnSync(program, args, { encoding: 'utf8' });
  assert.equal(result.status, 0, `${program} ${args.join(' ')}\n${result.stdout}${result.stderr}`);
  return result.stdout.trim();
}

assert.equal(process.getuid?.(), 0, 'run this test executable through sudo');
const root = mkdtempSync(join(tmpdir(), 'doramcap-systemd-overlay-probe-'));
const name = `doramcap-overlay-probe-${process.pid}`;
const transient = `${name}-transient.service`;
const adopted = `${name}-adopted.service`;
const runtimeLink = `/run/systemd/system/${adopted}`;
const runtimeDropIns = `${runtimeLink}.d`;
const directory = (name: string) => {
  const path = join(root, name);
  mkdirSync(path, { recursive: true });
  return path;
};
const lower = directory('lower');
const upper = directory('upper');
const work = directory('work');
const merged = directory('merged');
const other = directory('other');
const readonly = directory('readonly');
const target = directory('target');
const otherTarget = directory('other-target');
const readonlyTarget = directory('readonly-target');
const missing = join(root, 'missing');
let mounted = false;
let linked = false;

function binds(property: string): unknown[] {
  const path = command('systemctl', ['show', '--property=Id', '--value', adopted]);
  assert.equal(path, adopted);
  const unitPath = command('busctl', [
    'call', 'org.freedesktop.systemd1', '/org/freedesktop/systemd1',
    'org.freedesktop.systemd1.Manager', 'GetUnit', 's', adopted,
  ]).match(/^o "([^"]+)"$/)?.[1];
  assert.ok(unitPath);
  const result = JSON.parse(command('busctl', [
    '--json=short', 'get-property', 'org.freedesktop.systemd1', unitPath,
    'org.freedesktop.systemd1.Service', property,
  ]));
  assert.equal(result.type, 'a(ssbt)');
  return result.data;
}

try {
  console.log(`mount namespaces: test=${readlinkSync('/proc/self/ns/mnt')}, systemd=${readlinkSync('/proc/1/ns/mnt')}`);
  writeFileSync(join(lower, 'before'), 'lower\n');
  writeFileSync(join(other, 'keep'), 'other\n');
  writeFileSync(join(readonly, 'keep'), 'readonly\n');
  command('mount', ['-t', 'overlay', 'overlay', '-o',
    `lowerdir=${lower},upperdir=${upper},workdir=${work},index=on,metacopy=off,redirect_dir=off,xino=on`, merged]);
  mounted = true;
  writeFileSync(join(merged, 'from-console'), 'console\n');
  command('systemd-run', [
    '--quiet', '--wait', '--pipe', '--collect', `--unit=${transient}`,
    '--property=Type=oneshot', '--property=TimeoutStartSec=15s',
    `--property=BindPaths=${merged}:${target}`,
    '/bin/sh', '-ec', `test "$(cat ${target}/from-console)" = console; printf transient > ${target}/from-transient`,
  ]);
  assert.equal(readFileSync(join(upper, 'from-transient'), 'utf8'), 'transient');
  assert.equal(command('test', ['!', '-e', join(lower, 'from-transient')]), '');
  console.log('PASS: transient service sees Console overlay; writes stay in its upper');

  const file = join(root, adopted);
  writeFileSync(file, `[Unit]\nDescription=Console overlay qualification\n[Service]\nType=oneshot\nRemainAfterExit=yes\nTimeoutStartSec=15s\nBindPaths=${lower}:${target}\nBindPaths=-${missing}:${missing}:norbind\nBindReadOnlyPaths=${readonly}:${readonlyTarget}\nExecStart=/bin/sh -ec 'test "$(cat ${target}/before)" = lower; test "$(cat ${otherTarget}/keep)" = other; if test -e ${target}/from-console; then test "$(cat ${target}/from-console)" = console; printf adopted > ${target}/from-adopted; fi'\n`);
  mkdirSync(runtimeDropIns);
  writeFileSync(join(runtimeDropIns, '20-user.conf'), `[Service]\nBindPaths=${other}:${otherTarget}\n`);
  command('systemctl', ['link', '--runtime', file]);
  linked = true;
  command('systemctl', ['daemon-reload']);
  command('systemctl', ['start', adopted]);
  const before = binds('BindPaths');
  const readOnlyBefore = binds('BindReadOnlyPaths');
  assert.deepEqual(before, [[lower, target, false, 16384], [missing, missing, true, 0], [other, otherTarget, false, 16384]]);
  assert.deepEqual(readOnlyBefore, [[readonly, readonlyTarget, false, 16384]]);
  writeFileSync(join(runtimeDropIns, '99-doramcap-overlays.conf'), `[Service]\nBindPaths=\nBindPaths=${merged}:${target}\nBindPaths=-${missing}:${missing}:norbind\nBindPaths=${other}:${otherTarget}\n`);
  command('systemctl', ['daemon-reload']);
  assert.deepEqual(binds('BindPaths'), [[merged, target, false, 16384], ...before.slice(1)]);
  command('systemctl', ['restart', adopted]);
  assert.equal(readFileSync(join(upper, 'from-adopted'), 'utf8'), 'adopted');
  assert.equal(command('test', ['!', '-e', join(lower, 'from-adopted')]), '');
  console.log('PASS: adopted unit sees Console overlay; reset preserves writable entry order, optional flag and recursion');
  const readOnlyAfter = binds('BindReadOnlyPaths');
  console.log(`read-only binds before reset: ${JSON.stringify(readOnlyBefore)}`);
  console.log(`read-only binds after reset: ${JSON.stringify(readOnlyAfter)}`);
  assert.deepEqual(readOnlyAfter, readOnlyBefore, 'BindPaths= reset must preserve existing read-only binds');
  console.log('PASS: reset preserves read-only binds');
} finally {
  // Every start has a bounded oneshot operation; cleanup only touches this test's units and mount.
  if (linked) command('systemctl', ['stop', adopted]);
  if (linked) rmSync(runtimeLink);
  rmSync(runtimeDropIns, { recursive: true, force: true });
  command('systemctl', ['daemon-reload']);
  if (mounted) command('umount', [merged]);
  assert.ok(!readFileSync('/proc/self/mountinfo', 'utf8').split('\n').some(line => line.split(' ')[4] === merged));
  rmSync(root, { recursive: true });
  assert.equal(command('systemctl', ['list-units', '--all', '--no-legend', '--plain', transient, adopted]), '');
  console.log('CLEAN: test overlay and runtime units removed');
}
