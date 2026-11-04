#!/usr/bin/env node
// Run prebuilt policy probes with timeouts; mutate only a unique test cgroup subtree.
import { spawn, spawnSync } from 'node:child_process';
import { strict as assert } from 'node:assert';
import { mkdirSync, readFileSync, readdirSync, rmdirSync, watch, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: { binary: { type: 'string' }, 'enable-root-cpuset': { type: 'boolean' } } });
if (!values.binary) throw new Error('usage: --binary PATH [--enable-root-cpuset]');
const binary = resolve(values.binary);
assert.equal(process.geteuid!(), 0, 'native tests require root');
const probe = (args: (string | number)[], timeout: number) => {
  const result = spawnSync(binary, args.map(String), { stdio: 'inherit', timeout });
  assert.equal(result.status, 0, `probe ${args.join(' ')}`);
};
const allowed = /Cpus_allowed_list:\s*(\S+)/.exec(readFileSync('/proc/self/status', 'utf8'))![1];
const cpus = allowed.split(',').flatMap((range) => {
  const [first, last = first] = range.split('-').map(Number);
  return Array.from({ length: last - first + 1 }, (_, index) => first + index);
});
assert.ok(cpus.length >= 2, 'two CPUs required for watchdog separation');
const pin = (list: string) => assert.equal(spawnSync('taskset', ['-a', '-cp', list, String(process.pid)], { stdio: 'ignore' }).status, 0);
const rt = cpus.at(-1)!;
// Keep watchdog execution on a housekeeping CPU, separate from RT workloads.
pin(String(cpus[0]));
const root = `/sys/fs/cgroup/resource-policy-validation-${process.pid}`, control = '/sys/fs/cgroup/cgroup.subtree_control';
const DRAIN_TIMEOUT_MS = 10_000;
// Resolve once the cgroup subtree has no live processes; reject at the deadline.
const drained = (cgroup: string) => new Promise<void>((done, fail) => {
  const events = join(cgroup, 'cgroup.events');
  const finish = (settle: () => void) => { clearTimeout(deadline); watcher.close(); settle(); };
  const check = () => { if (/^populated 0$/m.test(readFileSync(events, 'utf8'))) finish(done); };
  const watcher = watch(events, check);
  const deadline = setTimeout(() => finish(() => fail(new Error(`${cgroup} still populated after kill`))), DRAIN_TIMEOUT_MS);
  check();
});
let enabledRoot = false, createdRoot = false;
try {
  if (!readFileSync(control, 'utf8').split(/\s+/).includes('cpuset')) {
    if (!values['enable-root-cpuset']) throw new Error('cpuset delegation unavailable; host setup requires separate approval');
    writeFileSync(control, '+cpuset');
    enabledRoot = true;
  }
  mkdirSync(root);
  createdRoot = true;
  // Controller delegation below this point affects only our test subtree.
  writeFileSync(join(root, 'cgroup.subtree_control'), '+cpuset');
  for (const priority of [0, 1, 30]) probe([root, `app-${priority}`, rt, priority], 15_000);
  writeFileSync(join(root, 'cpuset.cpus'), String(rt));
  probe([root, 'rejected', cpus[0], 'reject'], 10_000);
  // The dedicated probe drops identities after exec, inside private CI directories.
  probe(['--denied', '-', rt], 10_000);
  const codes = await Promise.all([0, 1, 30].map((priority) => new Promise<number | null>((done) => {
    const child = spawn(binary, ['--contend', String(priority), String(rt)], { stdio: 'inherit', timeout: 10_000 });
    child.once('exit', done);
  })));
  assert.deepEqual(codes, [0, 0, 0]);
  console.log(`PASS native resource-policy on ${process.arch}`);
} finally {
  if (createdRoot) {
    // Timeouts signal only the direct probe; kill surviving descendants, then remove empty owned leaves.
    writeFileSync(join(root, 'cgroup.kill'), '1');
    await drained(root);
    for (const entry of readdirSync(root, { withFileTypes: true })) if (entry.isDirectory()) rmdirSync(join(root, entry.name));
    rmdirSync(root);
  }
  if (enabledRoot) writeFileSync(control, '-cpuset');
  pin(allowed);
}
