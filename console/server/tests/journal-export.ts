// Build export_journal unprivileged, then run this script. Its fixtures need no journal privileges.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, readFile, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import protobuf from 'protobufjs';

const repository = resolve('../..');
const binary = join(repository, 'target/release/examples/export_journal');
const schema = await protobuf.load(join(repository, 'libs/workload-lifecycle/proto/journal.proto'));
const Index = schema.lookupType('doramcap.journal.v1.LogIndex');
const root = await mkdtemp(join(tmpdir(), 'journal-export-'));
const id = 'a'.repeat(32), other = 'b'.repeat(32);
const pending = (programId: string, identity = id, matchKind = 1) => ({ programId, identity, matchKind, state: 1 });
try {
  const bin = join(root, 'bin');
  await mkdir(bin);
  // A command fixture enforces exact identity matches and the sync-before-read order.
  await writeFile(join(bin, 'journalctl'), `#!${process.execPath}
const fs = require('node:fs');
const path = require('node:path');
const root = path.dirname(__dirname);
const args = process.argv.slice(2);
const mode = fs.readFileSync(path.join(root, 'mode'), 'utf8');
if (args.includes('--sync')) {
  if (mode === 'sync-failed') { process.stderr.write('journal unavailable'); process.exit(1); }
  fs.writeFileSync(path.join(root, 'synced'), 'yes'); process.exit(0);
}
if (!fs.existsSync(path.join(root, 'synced'))) process.exit(9);
const matched = args.find(value => value.startsWith('_SYSTEMD_INVOCATION_ID='));
if (matched && !args.includes('INVOCATION_ID=' + matched.split('=')[1])) process.exit(8);
if (mode === 'empty') process.exit(0);
const identity = matched?.split('=')[1] ?? args.find(value => value.startsWith('CONTAINER_ID_FULL='))?.split('=')[1];
process.stdout.write(JSON.stringify({__REALTIME_TIMESTAMP:'1700000000123456', MESSAGE: mode === 'large' ? 'x'.repeat(2048) : identity + '\\nsecond line'}) + '\\n');
if (mode === 'read-failed') { process.stderr.write('read failed'); process.exit(1); }
`, { mode: 0o700 });
  async function run(name: string, programs: object[], mode = 'normal', limited = false) {
    const artifacts = join(root, name);
    await mkdir(join(artifacts, 'logs'), { recursive: true });
    await writeFile(join(artifacts, 'logs/index.pb'), Index.encode(Index.create({ programs })).finish());
    await writeFile(join(root, 'mode'), mode);
    const env = { ...process.env, PATH: `${bin}:${process.env.PATH}` };
    if (limited) execFileSync('/bin/bash', ['-c', 'trap "" XFSZ; exec "$@"', 'journal-test', 'prlimit', '--fsize=1024', binary, artifacts], { env });
    else execFileSync(binary, [artifacts], { env });
    const bytes = await readFile(join(artifacts, 'logs/index.pb'));
    return { artifacts, programs: (Index.toObject(Index.decode(bytes), { longs: Number, defaults: true }).programs as { state: number; entryCount: number; sizeBytes: number; reason: string }[]) };
  }
  // Arrange / Act: the same program ID in adjacent runs has different invocation IDs.
  const first = await run('first', [pending('same-unit')]);
  const second = await run('second', [pending('same-unit', other)]);
  // Assert: each independent file contains its own identity and exactly one escaped entry.
  assert.equal(first.programs[0].state, 2);
  assert.equal(first.programs[0].entryCount, 1);
  assert.equal(await readFile(join(first.artifacts, 'logs/0.log'), 'utf8'), `1700000000.123456 ${id}\\nsecond line\n`);
  assert.doesNotMatch(await readFile(join(second.artifacts, 'logs/0.log'), 'utf8'), new RegExp(id));
  assert.equal(first.programs[0].sizeBytes, (await stat(join(first.artifacts, 'logs/0.log'))).size);
  assert.equal((await stat(join(first.artifacts, 'logs/index.pb'))).mode & 0o777, 0o600);
  const empty = await run('retained-away', [pending('unit')], 'empty');
  assert.equal(empty.programs[0].state, 2); assert.equal(empty.programs[0].entryCount, 0);
  const failed = await run('unavailable', [pending('unit'), { ...pending('project'), state: 3, reason: 'Project logging' }], 'sync-failed');
  assert.equal(failed.programs[0].state, 4); assert.match(failed.programs[0].reason, /unavailable/);
  assert.equal(failed.programs[1].state, 3);
  const partial = await run('partial', [pending('unit')], 'read-failed');
  assert.equal(partial.programs[0].state, 4); assert.equal(partial.programs[0].entryCount, 1);
  assert.ok(partial.programs[0].sizeBytes > 0);
  const exhausted = await run('storage-limit', [pending('unit')], 'large', true);
  assert.equal(exhausted.programs[0].state, 4); assert.match(exhausted.programs[0].reason, /File too large/);
  const interruptedPath = join(root, 'interrupted/logs');
  await mkdir(interruptedPath, { recursive: true });
  await writeFile(join(interruptedPath, '0.log'), 'already saved\n');
  const interrupted = await run('interrupted', [pending('unit')]);
  assert.equal(interrupted.programs[0].state, 4); assert.match(interrupted.programs[0].reason, /preserved/);
  assert.equal(await readFile(join(interruptedPath, '0.log'), 'utf8'), 'already saved\n');
  const terminal = await run('terminal', [{ ...pending('unit'), state: 2, entryCount: 7, sizeBytes: 99 }], 'sync-failed');
  assert.equal(terminal.programs[0].state, 2); assert.equal(terminal.programs[0].entryCount, 7);
  const container = await run('container', [pending('app/replica', 'c'.repeat(64), 2)]);
  assert.equal(container.programs[0].state, 2);
  console.log('Journal export: isolation, retention, failures, storage exhaustion, recovery and container match passed.');
} finally { await rm(root, { recursive: true, force: true }); }
