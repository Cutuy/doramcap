#!/usr/bin/env node
// Check cgroup CPU accounting against the kernel's cpu.stat; creates and removes only its own cgroups.
// Build system-monitor with --features bpf unprivileged, then run elevated with --monitor ABSOLUTE_PATH.
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { strict as assert } from 'node:assert';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, rmdirSync, statSync, writeFileSync } from 'node:fs';
import { setTimeout as sleep } from 'node:timers/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const FLAG_ROOT = 16n, FLAG_READ_ERROR = 64n;
type Row = { cgroup: bigint; parent: bigint; cpu: bigint; flags: bigint; runtime: bigint };
type Sample = { dropped: bigint; overflow: bigint; rows: Row[] };

/** Yield protobuf fields; this test reads only varint and length-delimited values. */
function* fields(data: Uint8Array): Generator<[number, bigint | Uint8Array]> {
  let at = 0;
  const varint = () => {
    for (let value = 0n, shift = 0n; ; shift += 7n) {
      const byte = data[at++];
      assert.notEqual(byte, undefined, 'truncated protobuf');
      value |= BigInt(byte & 127) << shift;
      if (byte < 128) return value;
    }
  };
  while (at < data.length) {
    const tag = Number(varint());
    if ((tag & 7) === 0) yield [tag >>> 3, varint()];
    else {
      assert.equal(tag & 7, 2, 'unexpected protobuf wire type');
      const size = Number(varint());
      yield [tag >>> 3, data.subarray(at, (at += size))];
    }
  }
}
const message = (data: Uint8Array, field: number) => {
  for (const [number, value] of fields(data)) if (number === field) return value as Uint8Array;
  throw new Error(`missing protobuf field ${field}`);
};

/** Read a length-prefixed Packet stream: snapshot = 1, then SnapshotFrame.snapshot = 2. */
function samples(path: string): Sample[] {
  const data = readFileSync(path), result: Sample[] = [];
  for (let at = 0; at < data.length;) {
    const size = data.readUInt32LE(at);
    const snapshot = message(message(data.subarray(at + 4, at + 4 + size), 1), 2);
    const sample: Sample = { dropped: 0n, overflow: 0n, rows: [] };
    for (const [number, value] of fields(snapshot)) {
      if (number === 6) sample.dropped = value as bigint;
      if (number === 7) sample.overflow = value as bigint;
      if (number !== 8) continue;
      const row: Row = { cgroup: 0n, parent: 0n, cpu: 0n, flags: 0n, runtime: 0n };
      for (const [id, item] of fields(value as Uint8Array)) {
        const name = (['', 'cgroup', 'parent', 'cpu', 'flags', 'runtime'] as const)[id];
        if (name) row[name] = item as bigint;
      }
      sample.rows.push(row);
    }
    result.push(sample);
    at += 4 + size;
  }
  return result;
}
const usage = (group: string) => BigInt(/usage_usec (\d+)/.exec(readFileSync(join(group, 'cpu.stat'), 'utf8'))![1]) * 1000n;
const exited = (child: ChildProcess) => new Promise<number | null>((done) => {
  if (child.exitCode !== null || child.signalCode !== null) done(child.exitCode);
  else child.once('exit', done);
});
/** Runtime a group accumulated between the first and last samples. */
function delta(all: Sample[], select: (row: Row) => boolean) {
  const rows = (sample: Sample) => new Map(sample.rows.filter(select).map((row) => [row.cpu, row.runtime]));
  const first = rows(all[0]), last = rows(all.at(-1)!);
  assert.ok(first.size > 0, 'missing rows');
  assert.deepEqual([...first.keys()], [...last.keys()]);
  return [...last].reduce((sum, [cpu, runtime]) => sum + runtime - first.get(cpu)!, 0n);
}

const { values } = parseArgs({ options: { monitor: { type: 'string' } } });
if (!values.monitor) throw new Error('usage: --monitor ABSOLUTE_PATH');
const monitor = resolve(values.monitor);
assert.equal(process.geteuid!(), 0, 'native validation requires privilege');
const work = mkdtempSync(join(tmpdir(), 'doramcap-native-'));
const config = (name: string, text: string) => {
  writeFileSync(join(work, name), `app { id: "monitor" config { [type.googleapis.com/doramcap.monitor.v1.MonitorConfig] { records: 16384 queue_slots: 8 ${text} } } }`);
  return ['--config', join(work, name)];
};

// Failed output initialization must not publish readiness or touch an existing recording.
const ready = join(work, 'ready'), existing = join(work, 'existing.mcap');
writeFileSync(existing, 'existing recording');
const topics = 'samples_topic: "doramcap/cpuacct" dictionary_topic: "doramcap/cpuacct/dictionary"';
for (const section of [`output_path: "${work}"`, `recording { path: "${existing}" ${topics} }`]) {
  const failed = spawnSync(monitor, config('failing.pbtxt', `samples: 1 ${section}`), { env: { ...process.env, DORAMCAP_READY_FILE: ready }, timeout: 15_000 });
  assert.notEqual(failed.status, 0);
  assert.ok(!existsSync(ready), 'failed initialization published readiness');
  assert.equal(readFileSync(existing, 'utf8'), 'existing recording');
}
console.log('PASS failed initialization publishes no readiness');

// Two monitors run together; each must report what the kernel charged two busy cgroups.
const root = `/sys/fs/cgroup/doramcap-validation-${process.pid}`, groups = [root, join(root, 'a'), join(root, 'b')];
const children: ChildProcess[] = [];
mkdirSync(root); // Fail if it exists; never adopt another workload's cgroup.
try {
  for (const group of groups.slice(1)) mkdirSync(group);
  const ids = groups.map((group) => statSync(group, { bigint: true }).ino), baseline = groups.map(usage);
  const outputs = ['first.pb', 'second.pb'].map((name) => join(work, name));
  const monitors = outputs.map((output, index) => spawn(monitor, config(`${index}.pbtxt`, `samples: 0 output_path: "${output}"`),
    { env: { ...process.env, DORAMCAP_READY_FILE: `${output}.ready` }, stdio: ['ignore', 'ignore', 'inherit'] }));
  children.push(...monitors);
  for (let i = 0; i < 150 && !outputs.every((output) => existsSync(`${output}.ready`)); i++) await sleep(100);
  assert.ok(outputs.every((output) => existsSync(`${output}.ready`)), 'monitors did not become ready');
  await sleep(500); // Hold a first sample from before the workload starts.
  const workers = groups.slice(1).map((group) => {
    const worker = spawn('/bin/sh', ['-c', 'while :; do :; done'], { stdio: 'ignore' });
    children.push(worker);
    writeFileSync(join(group, 'cgroup.procs'), String(worker.pid));
    return worker;
  });
  await sleep(3000);
  for (const worker of workers) worker.kill('SIGKILL');
  await Promise.all(workers.map(exited));
  await sleep(500); // Hold a last sample from after the workload stops.
  const expected = groups.map((group, index) => usage(group) - baseline[index]);
  for (const child of monitors) child.kill('SIGINT');
  for (const code of await Promise.all(monitors.map(exited))) assert.equal(code, 0);

  for (const output of outputs) {
    const all = samples(output);
    assert.ok(all.length >= 2, 'too few samples');
    for (const sample of all) {
      assert.equal(sample.dropped, 0n);
      assert.equal(sample.overflow, 0n);
      assert.ok(sample.rows.every((row) => !(row.flags & FLAG_READ_ERROR)), 'native read error');
    }
    ids.forEach((id, index) => {
      const total = delta(all, (row) => row.cgroup === id), reference = expected[index];
      const difference = total > reference ? total - reference : reference - total;
      assert.ok(reference > 100_000_000n, 'workload did not run');
      assert.ok(difference <= 2_000_000n || difference * 100n <= reference, `cgroup ${id}: monitor ${total} ns, cpu.stat ${reference} ns`);
    });
    const leaves = all.at(-1)!.rows.filter((row) => row.cgroup === ids[1] || row.cgroup === ids[2]);
    assert.ok(leaves.every((row) => row.parent === ids[0]), 'wrong parent cgroup');
    // Global CPU records include the controlled work, even with a namespaced cgroup mount.
    assert.ok(delta(all, (row) => (row.flags & FLAG_ROOT) !== 0n) * 100n >= expected[0] * 99n, 'root CPU rows missed controlled work');
  }
  console.log(`PASS two monitors match cpu.stat within 1% on ${process.arch}`);
} finally {
  for (const child of children) child.kill('SIGKILL');
  await Promise.all(children.map(exited));
  for (const group of groups.toReversed()) {
    for (let i = 0; existsSync(group); i++) {
      try { rmdirSync(group); } catch (error) { if (i >= 50) throw error; await sleep(100); }
    }
  }
  rmSync(work, { recursive: true, force: true });
}

// The worker launcher runs these suites through this one entry point.
const repository = join(dirname(fileURLToPath(import.meta.url)), '../../..');
for (const [suite, ...args] of [
  ['libs/resources/resource-policy/tests/validate_native.ts', '--binary', join(dirname(monitor), 'examples/native_policy'), '--enable-root-cpuset'],
  ['libs/workload-supervisor/tests/validate_native.ts', '--monitor', monitor],
]) {
  const result = spawnSync(process.execPath, [join(repository, suite), ...args], { stdio: 'inherit', timeout: 300_000 });
  assert.equal(result.status, 0, suite);
}
