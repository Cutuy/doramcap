/** Launch and verify the independent CUDA Dora nodes. */
import { spawn } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: {
  dora: { type: 'string', default: 'dora' }, consumers: { type: 'string', default: '3' },
  frames: { type: 'string', default: '12' }, slots: { type: 'string', default: '2' },
  bytes: { type: 'string', default: '4096' }, 'slot-alignment': { type: 'string', default: '4096' },
  failure: { type: 'string' }, sanitizer: { type: 'string' }, seed: { type: 'string', default: '0x12345678' },
  checksum: { type: 'boolean', default: false },
} });
const integer = (value: string, name: string) => {
  const n = Number(value);
  if (!Number.isSafeInteger(n) || n < 0) throw new Error(`invalid ${name}`);
  return n;
};
const consumers = integer(values.consumers, 'consumers');
const frames = integer(values.frames, 'frames');
const slots = integer(values.slots, 'slots');
const bytes = integer(values.bytes, 'bytes');
const alignment = integer(values['slot-alignment'], 'slot alignment');
const seed = integer(values.seed, 'seed');
if (seed > 0xffffffff) throw new Error('seed must fit u32');
if (values.failure && !['stall', 'duplicate', 'corrupt-gpu', 'corrupt-cpu'].includes(values.failure)) throw new Error('invalid failure mode');
const binary = process.env.CUDA_SHM_BINARY ?? resolve(dirname(fileURLToPath(import.meta.url)), '../../../target/release/cuda-shm-demo');
const quote = (s: string) => `'${s.replaceAll("'", "'\\''")}'`;
const path = values.sanitizer ?? binary;
const prefix = values.sanitizer ? `--tool memcheck --error-exitcode 99 ${quote(binary)} ` : '';
const corrupt = values.failure?.startsWith('corrupt') ? ' --corrupt' : '';
const nodes = [{ id: 'producer', path, args: `${prefix}producer --consumers ${consumers} --frames ${frames} --slots ${slots} --bytes ${bytes} --slot-alignment ${alignment} --timeout ${values.failure ? 5 : 40} --seed ${seed}${corrupt}`,
  inputs: Object.fromEntries(Array.from({ length: consumers }, (_, i) => [`consumer${i}`, { source: `consumer${i}/returns`, queue_size: 128 }])), outputs: ['frames'] }];
for (let i = 0; i < consumers; i++) nodes.push({ id: `consumer${i}`, path,
  args: `${prefix}consumer --id ${i} --seed ${seed}${i === 0 && ['stall', 'duplicate'].includes(values.failure ?? '') ? ` --${values.failure}` : ''}`,
  inputs: { frames: { source: 'producer/frames', queue_size: 128 } }, outputs: ['returns'] });
const tmp = await mkdtemp(resolve(tmpdir(), 'cuda-dora-flow-'));
try {
  const flow = resolve(tmp, 'flow.yml');
  await writeFile(flow, JSON.stringify({ nodes }));
  const env = { ...process.env };
  if (values.checksum || values.failure === 'corrupt-cpu') env.CUDA_SHM_CHECKSUM = '1';
  if (values.failure === 'corrupt-gpu') delete env.CUDA_SHM_CHECKSUM;
  const child = spawn(values.dora, ['run', flow], { env, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  child.stdout.on('data', chunk => { output += chunk; });
  child.stderr.on('data', chunk => { output += chunk; });
  let timedOut = false;
  const kill = () => { if (child.pid) { try { process.kill(-child.pid, 'SIGKILL'); } catch {} } };
  const timeout = setTimeout(() => { timedOut = true; kill(); }, 65_000);
  const stop = () => kill();
  process.once('SIGINT', stop); process.once('SIGTERM', stop);
  const status = await new Promise<number | null>((res, rej) => { child.once('error', rej); child.once('close', res); });
  clearTimeout(timeout); process.removeListener('SIGINT', stop); process.removeListener('SIGTERM', stop);
  output = output.replace(/\x1b\[[0-9;]*m/g, '');
  console.log(output);
  const require = (condition: unknown, message: string) => { if (!condition) throw new Error(message); };
  require(!timedOut, 'Dora did not terminate');
  if (values.failure) {
    const expected: Record<string, string> = { stall: 'TIMEOUT: no unsafe reuse', duplicate: 'duplicate return', 'corrupt-cpu': 'CPU_CONTENT_MISMATCH', 'corrupt-gpu': 'GPU_CONTENT_MISMATCH' };
    require(status !== 0 && output.includes(expected[values.failure]), 'expected deterministic rejection missing');
    if (values.failure.startsWith('corrupt')) require(!output.includes('CONSUMER_SUMMARY'), 'corrupt contents reported success');
    if (values.failure === 'stall') { const m = output.match(/TIMEOUT: no unsafe reuse; published=(\d+).*reclaimed: (\d+)/); require(m && Number(m[1]) === slots && Number(m[2]) === 0, 'unexpected stall accounting'); }
  } else {
    require(status === 0, `Dora exited ${status}`);
    const summaries = (name: string) => output.split('\n').filter(line => line.includes(`${name} `)).map(line => JSON.parse(line.split(`${name} `)[1]));
    const producers = summaries('PRODUCER_SUMMARY'), readers = summaries('CONSUMER_SUMMARY');
    require(producers.length === 1 && readers.length === consumers, 'missing process summaries');
    require(producers[0].stats.reclaimed === frames && producers[0].stats.published_pending > 0, 'producer did not publish and reclaim');
    require(readers.every(c => c.frames === frames && c.gpu_verified && c.max_generation > 1 && c.seed === seed && (!env.CUDA_SHM_CHECKSUM || c.cpu_verified)), 'consumer validation failed');
  }
  console.log('PASS:', values.failure || 'SPMC GPU payload + asynchronous publication + reuse');
} finally { await rm(tmp, { recursive: true, force: true }); }
