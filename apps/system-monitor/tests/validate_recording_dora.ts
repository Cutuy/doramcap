#!/usr/bin/env node
// Compare embedded snapshots with the same messages recorded through a real Dora flow.
// Requires prebuilt system-monitor (bpf, dora), mcap-recorder and compare_recordings; run elevated.
import { spawn, spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: { dora: { type: 'string' }, 'bin-dir': { type: 'string' } } });
if (!values.dora || !values['bin-dir']) throw new Error('usage: --dora DORA --bin-dir TARGET_PROFILE_DIR');
const bin = resolve(values['bin-dir']);
const root = mkdtempSync(join(tmpdir(), 'doramcap-recording-e2e-'));
const embedded = join(root, 'embedded.mcap'), received = join(root, 'received.mcap');
const topic = 'doramcap/cpuacct', samples = 24;

/** Run Dora in its own process group so a timeout also stops its nodes. */
function runDora(flow: string, timeoutMs: number): Promise<void> {
  const dora = spawn(values.dora!, ['run', flow], { cwd: root, stdio: 'inherit', detached: true });
  const signal = (name: NodeJS.Signals) => { try { process.kill(-dora.pid!, name); } catch { /* Already exited. */ } };
  const term = setTimeout(() => signal('SIGTERM'), timeoutMs), kill = setTimeout(() => signal('SIGKILL'), timeoutMs + 10_000);
  return new Promise((done, fail) => {
    dora.once('error', fail);
    dora.once('exit', (code, name) => {
      clearTimeout(term); clearTimeout(kill);
      if (code === 0) done(); else fail(new Error(`dora run ended with ${name ?? code}`));
    });
  });
}

try {
  writeFileSync(join(root, 'monitor.pbtxt'), `records: 4096 queue_slots: 8 samples: ${samples} dora {}
recording { path: "${embedded}" samples_topic: "${topic}" dictionary_topic: "${topic}/dictionary" }`);
  writeFileSync(join(root, 'recorder.pbtxt'), `output_path: "${received}"
report_path: "${join(root, 'received.report.pbtxt')}"
input { input: "snapshots" topic: "${topic}" message_encoding: "protobuf" schema_name: "doramcap.telemetry.v1.Packet" schema_encoding: "protobuf" schema_path: "${resolve(import.meta.dirname, '../../../libs/telemetry/protocol/proto/telemetry_descriptor.bin')}" timestamp_key: "capture_unix_ns" sequence_key: "sequence" }`);
  // JSON is a YAML subset and safely quotes filesystem paths.
  const flow = join(root, 'flow.yml');
  writeFileSync(flow, JSON.stringify({ nodes: [
    { id: 'monitor', path: join(bin, 'system-monitor'), args: `--config ${join(root, 'monitor.pbtxt')}`, outputs: ['snapshots', 'dictionary', 'health'] },
    { id: 'recorder', outputs: ['health'], path: join(bin, 'mcap-recorder'), args: `--config ${join(root, 'recorder.pbtxt')}`,
      inputs: { snapshots: { source: 'monitor/snapshots', queue_size: 64 } } },
  ] }));
  await runDora(flow, 45_000);
  const compared = spawnSync(join(bin, 'examples/compare_recordings'), [embedded, received, topic, String(samples)], { stdio: 'inherit' });
  if (compared.status !== 0) throw new Error('recordings differ');
} finally {
  rmSync(root, { recursive: true, force: true });
}
