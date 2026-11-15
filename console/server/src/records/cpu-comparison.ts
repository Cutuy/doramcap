import { closeSync, createReadStream, fstatSync, readSync } from 'node:fs';
import { opendir } from 'node:fs/promises';
import { join, posix } from 'node:path';
import { finished } from 'node:stream/promises';
import { Worker } from 'node:worker_threads';
import { setImmediate as nextTurn } from 'node:timers/promises';
import { McapStreamReader } from '@mcap/core';
import { capabilities, execution, launch, telemetry, wire } from '../native.js';
import type { doramcap } from '../generated/console.js';
import type { Side } from './compare.js';
import type { AnalysisModule } from './analysis.js';

type Snapshot = doramcap.telemetry.v1.ISnapshotFrame;
type Dictionary = doramcap.telemetry.v1.IDictionaryEntry;
type Target = { service: string; path?: string; unit?: string };
type Counters = { id: string; parent: string; path: string; values: Map<number, bigint> };
type Measurement = { runtime: bigint; elapsed: bigint; issues: Set<string> };
const integer = (value: unknown) => BigInt(String(value ?? 0));
const NS_PER_SECOND = 1_000_000_000n;
const RUNTIME = 1, INCLUSIVE = 4, SYNTHETIC = 8, ROOT = 16, READ_ERROR = 64, VIRTUAL = 128;
const ALL_CPUS = 0xffffffff, WIRE_VERSION = 2, SCHEMA_VERSION = 1;
/** Existing telemetry packet and saved Compose manifest limits. */
const MAX_PACKET_BYTES = 128 * 1024 * 1024, MAX_MANIFEST_BYTES = 4 * 1024 * 1024;
const PACKET_SCHEMA = 'doramcap.telemetry.v1.Packet';
const GENERATED_PARENT = /^doramcap-compose-[0-9a-f]{12}-[0-9a-f]{12}-(system|user)-(.+)\.slice$/;
const GENERATED_DIRECTORY = /^\/doramcap-compose-[0-9a-f]{12}\/[0-9a-f]{12}\/(system|user)\/([^/]+)$/;
const NATIVE_SYSTEM = /^doramcap-[0-9a-f]{12}-system\.slice$/;
const key = (service: string, field: string) => 'service.' + service + '.' + field;
const RECORDS_PER_TURN = 64;
const MAX_PRESENTATION_CHARS = 512;
type ServiceAnalysis = { runtime: bigint; elapsed: bigint; span: bigint; issues: string[]; unavailable?: string };
type CpuAnalysis = { services: Map<string, ServiceAnalysis>; unavailable?: string };
/** Preserve integer nanoseconds while expressing seconds in comparison values. */
const seconds = (ns: bigint) => String(ns / NS_PER_SECOND) + '.' + String(ns % NS_PER_SECOND).padStart(9, '0');

/** Read original evidence beneath the run; never use run-ID-normalized comparison input. */
function readEvidence(root: string, relative: string) {
  const fd = capabilities.openFile(root, relative);
  try {
    const before = fstatSync(fd, { bigint: true });
    if (before.size > BigInt(MAX_MANIFEST_BYTES)) throw new Error('Saved service placement exceeds the manifest limit');
    const bytes = Buffer.alloc(Number(before.size));
    let offset = 0;
    while (offset < bytes.length) {
      const read = readSync(fd, bytes, offset, bytes.length - offset, null);
      if (!read) throw new Error('Saved service placement is incomplete');
      offset += read;
    }
    const after = fstatSync(fd, { bigint: true });
    if (before.size !== after.size || before.mtimeNs !== after.mtimeNs) throw new Error('Saved service placement changed');
    return bytes;
  } finally { closeSync(fd); }
}
const serviceNames = (side: Side) => new Set([
  ...(side.detail.status?.outcome?.serviceExits ?? []).map(exit => exit.service!),
  ...(side.capture.setup?.images ?? []).map(image => image.service!),
]);
/** Expand a systemd slice's native hierarchy without decoding escapes inside its components. */
function slicePath(name: string) {
  const parts = name.slice(0, -'.slice'.length).split('-');
  return '/' + parts.map((_, index) => parts.slice(0, index + 1).join('-') + '.slice').join('/');
}
function targets(root: string, side: Side): Target[] {
  const names = serviceNames(side);
  let manifest: Record<string, any> | undefined;
  try { manifest = JSON.parse(readEvidence(root, 'experiment.input.json').toString('utf8')); } catch { /* Native runs have no Compose manifest. */ }
  if (manifest?.services && typeof manifest.services === 'object') {
    // Inclusive counters include services placed at the same parent or anywhere below it.
    const countParents = (systemd: boolean) => {
      const counts = new Map<string, number>();
      for (const config of Object.values(manifest!.services) as any[]) {
        const parent = config?.cgroup_parent;
        if (typeof parent !== 'string') return;
        let path = systemd ? parent.endsWith('.slice') && !parent.includes('/') ? slicePath(parent) : undefined
          : posix.isAbsolute(parent) ? posix.resolve(parent) : undefined;
        // Relative cgroupfs placement needs the unsaved daemon cgroup; exclusivity is unknown.
        if (!path) return;
        for (; path !== '/'; path = posix.dirname(path)) counts.set(path, (counts.get(path) ?? 0) + 1);
      }
      return counts;
    };
    const sliceParents = countParents(true), directoryParents = countParents(false);
    for (const service of Object.keys(manifest.services)) names.add(service);
    return [...names].map(service => {
      const parent = manifest!.services[service]?.cgroup_parent;
      if (typeof parent !== 'string') return { service };
      const slice = GENERATED_PARENT.exec(parent), directory = GENERATED_DIRECTORY.exec(parent);
      // A custom/shared parent does not establish exclusive attribution to this service.
      const matches = slice ? slice[2].replaceAll('\\x2d', '-') === service : directory?.[2] === service;
      const path = slice ? slicePath(parent) : parent, counts = slice ? sliceParents : directoryParents;
      return { service, ...(matches && counts?.get(path) === 1 ? { path } : {}) };
    });
  }
  // Native unit names are repository-owned even when a user's Slice= puts them elsewhere.
  try {
    const identity = launch.SystemAppCgroup.decode(readEvidence(root, 'system-app-cgroup.pb'));
    const leaf = posix.basename(identity.path);
    if (!NATIVE_SYSTEM.test(leaf)) return [...names].map(service => ({ service }));
    const prefix = leaf.slice(0, -'-system.slice'.length);
    for (const program of side.detail.status?.logs?.programs ?? []) if (!program.programId!.includes('/')) names.add(program.programId!);
    return [...names].map(service => ({ service, unit: prefix + '-' + service + '.service' }));
  } catch { return [...names].map(service => ({ service })); }
}

/** One sequential reduction; keeps one dictionary, two counter endpoints and one result per service. */
export class CpuComparison {
  private session = '';
  private revision = -1n;
  private dictionary = new Map<string, Dictionary>();
  private previous?: { session: string; sequence: bigint; end: bigint; groups: Map<string, Counters> };
  private span = 0n;
  private measurements = new Map<string, Measurement>();
  private paths = new Map<string, Target[]>();
  private units = new Map<string, Target[]>();
  private snapshotCount = 0;
  constructor(private targets: Target[]) {
    for (const target of targets) {
      this.measurements.set(target.service, { runtime: 0n, elapsed: 0n, issues: new Set() });
      const index = target.path ? this.paths : this.units, identity = target.path ?? target.unit;
      if (identity) index.set(identity, [...(index.get(identity) ?? []), target]);
    }
  }
  private issue(reason: string) { for (const value of this.measurements.values()) value.issues.add(reason); }
  private names(session: string, revision: bigint, entries: Dictionary[]) {
    if (session !== this.session) {
      this.session = session;
      this.revision = -1n;
      this.dictionary.clear();
    }
    if (revision < this.revision) return;
    this.revision = revision;
    this.dictionary = new Map(entries.map(entry => [String(entry.id), entry]));
  }
  /** Accept original CPU Packet bytes; unsupported versions fail the source explicitly. */
  receive(payload: Uint8Array) {
    if (payload.length > MAX_PACKET_BYTES) throw new Error('CPU packet exceeds the supported telemetry limit');
    const packet = telemetry.Packet.decode(payload);
    if (packet.dictionary) {
      const frame = packet.dictionary;
      if (frame.version !== WIRE_VERSION || !frame.sessionId) throw new Error('Unsupported CPU dictionary');
      this.names(frame.sessionId, integer(frame.revision), frame.entries ?? []);
      if (integer(frame.overflow)) this.issue('dictionary overflow');
      return;
    }
    const frame = packet.snapshot, snapshot = frame?.snapshot;
    if (!frame || !snapshot || frame.version !== WIRE_VERSION || snapshot.schemaVersion !== SCHEMA_VERSION || !frame.sessionId)
      throw new Error('Unsupported CPU snapshot');
    if (frame.dictionaryFull) this.names(frame.sessionId, integer(frame.dictionaryRevision), frame.dictionary ?? []);
    this.snapshotCount++;
    const session = frame.sessionId, sequence = integer(snapshot.sequence), end = integer(snapshot.monotonicEndNs);
    const previous = this.previous;
    if (previous && previous.session !== session) this.issue('monitor restarted');
    if (previous && previous.session === session && end <= previous.end) {
      this.issue('out-of-order snapshots');
      return;
    }
    const advancing = previous && previous.session === session && sequence > previous.sequence && end > previous.end;
    const elapsed = advancing ? end - previous.end : 0n;
    if (previous && !advancing && previous.session === session) this.issue('counter sequence reset');
    if (advancing) {
      this.span += elapsed;
      if (sequence !== previous.sequence + 1n) this.issue('snapshots skipped');
    }
    if (integer(snapshot.droppedBatches)) this.issue('snapshots dropped');
    if (integer(snapshot.capacityOverflow)) this.issue('counter capacity overflow');
    if (integer(frame.dictionaryOverflow)) this.issue('dictionary overflow');
    const groups = this.groups(frame);
    for (const target of this.targets) {
      const measurement = this.measurements.get(target.service)!;
      const next = groups.get(target.service), old = previous?.groups.get(target.service);
      if (!elapsed) continue;
      if (!next || !old) { measurement.issues.add('unmeasured intervals'); continue; }
      if (next.id !== old.id || next.path !== old.path || next.parent !== old.parent || next.values.size !== old.values.size) {
        measurement.issues.add('cgroup identity changed'); continue;
      }
      let runtime = 0n, valid = true;
      for (const [cpu, value] of next.values) {
        const before = old.values.get(cpu);
        if (before === undefined || value < before) { valid = false; break; }
        runtime += value - before;
      }
      if (!valid) { measurement.issues.add('counter reset'); continue; }
      measurement.runtime += runtime;
      measurement.elapsed += elapsed;
    }
    this.previous = { session, sequence, end, groups };
  }
  private groups(frame: Snapshot) {
    const groups = new Map<string, Counters>();
    if (frame.sessionId !== this.session || integer(frame.dictionaryRevision) !== this.revision || integer(frame.snapshot?.capacityOverflow)) return groups;
    const byId = new Map<string, Target>();
    const ambiguous = new Set<string>();
    for (const [id, entry] of this.dictionary) {
      const path = entry.path;
      if (path == null) continue;
      const matches = [...(this.paths.get(path) ?? []), ...(this.units.get(posix.basename(path)) ?? [])];
      if (matches.length > 1) ambiguous.add(id);
      if (matches.length) byId.set(id, matches[0]);
    }
    const invalid = new Set<string>();
    for (const row of frame.snapshot?.records ?? []) {
      const id = String(row.cgroupId), target = byId.get(id);
      if (!target || ambiguous.has(id)) continue;
      const parent = String(row.parentId), path = this.dictionary.get(id)!.path!;
      const flags = Number(row.flags), cpu = Number(row.cpu);
      const existing = groups.get(target.service);
      if ((flags & (RUNTIME | INCLUSIVE)) !== (RUNTIME | INCLUSIVE) || (flags & (ROOT | READ_ERROR | VIRTUAL | SYNTHETIC)) ||
          cpu === ALL_CPUS || parent !== String(this.dictionary.get(id)!.parentId) ||
          existing && (existing.id !== id || existing.values.has(cpu))) {
        invalid.add(target.service); continue;
      }
      const group = existing ?? { id, parent, path, values: new Map<number, bigint>() };
      group.values.set(cpu, integer(row.runtimeNs));
      groups.set(target.service, group);
    }
    for (const service of invalid) groups.delete(service);
    return groups;
  }
  result(): CpuAnalysis {
    const services = new Map<string, ServiceAnalysis>();
    for (const target of this.targets) {
      const value = this.measurements.get(target.service)!;
      services.set(target.service, { runtime: value.runtime, elapsed: value.elapsed, span: this.span,
        issues: [...value.issues].sort(), ...(!value.elapsed ? { unavailable: !target.path && !target.unit
          ? 'Service cgroup was not recorded' : 'No valid measured CPU interval' } : {}) });
    }
    return { services, ...(!this.snapshotCount ? { unavailable: 'Recording contains no CPU snapshots' } : {}) };
  }
}

/** Inspect numbered monitor rotations without retaining directory contents; gaps cannot imply complete coverage. */
async function rotations(root: string, relative: string, signal?: AbortSignal) {
  signal?.throwIfAborted();
  // Validate the original file through the evidence boundary before listing any sibling names.
  const fd = capabilities.openFile(root, relative);
  const name = posix.basename(relative), stem = posix.parse(name).name, prefix = stem + '.';
  let count = 0n, maximum = 0n;
  try {
    const directory = await opendir(join(root, posix.dirname(relative)));
    for await (const entry of directory) {
      signal?.throwIfAborted();
      if (entry.name === name || !entry.name.startsWith(prefix) || !entry.name.endsWith('.mcap')) continue;
      const digits = entry.name.slice(prefix.length, -'.mcap'.length);
      if (!/^\d{6,}$/.test(digits)) continue;
      const index = BigInt(digits);
      if (!index || String(index).padStart(6, '0') !== digits) throw new Error('CPU rotation names are ambiguous');
      count++;
      if (index > maximum) maximum = index;
    }
    signal?.throwIfAborted();
  } finally { closeSync(fd); }
  if (count !== maximum) throw new Error('CPU recording has missing rotation segments');
  return count;
}
/** Read one completed segment incrementally; the caller's signal cancels and closes its descriptor. */
async function scanSegment(root: string, relative: string, reducer: CpuComparison, signal?: AbortSignal) {
  signal?.throwIfAborted();
  const fd = capabilities.openFile(root, relative);
  let before;
  try { before = fstatSync(fd, { bigint: true }); } catch (error) { closeSync(fd); throw error; }
  const stream = createReadStream('', { fd, autoClose: true, signal });
  const reader = new McapStreamReader({ validateCrcs: true });
  const schemas = new Map<number, string>(), channels = new Map<number, { schemaId: number; messageEncoding: string }>();
  let records = 0;
  try {
    for await (const chunk of stream) {
      signal?.throwIfAborted();
      reader.append(chunk);
      for (let record; (record = reader.nextRecord());) {
        if (++records % RECORDS_PER_TURN === 0) await nextTurn(undefined, { signal });
        signal?.throwIfAborted();
        if (record.type === 'Schema') {
          if (record.encoding === 'protobuf') schemas.set(record.id, record.name);
        } else if (record.type === 'Channel') channels.set(record.id, record);
        else if (record.type === 'Message') {
          const channel = channels.get(record.channelId);
          if (channel?.messageEncoding === 'protobuf' && schemas.get(channel.schemaId) === PACKET_SCHEMA) reducer.receive(record.data);
        }
      }
    }
    if (!reader.done() || reader.bytesRemaining()) throw new Error('CPU recording is incomplete');
    // Keep the opened object authoritative and reject changes during traversal.
    const current = capabilities.openFile(root, relative);
    try {
      const after = fstatSync(current, { bigint: true });
      if (before.dev !== after.dev || before.ino !== after.ino || before.size !== after.size || before.mtimeNs !== after.mtimeNs)
        throw new Error('CPU recording changed during comparison');
    } finally { closeSync(current); }
  } finally { stream.destroy(); await finished(stream).catch(() => {}); }
}
/** Scan the base and every consecutive native monitor rotation; no file-length or message-count cap. */
async function scan(root: string, relative: string, targets: Target[], signal?: AbortSignal): Promise<CpuAnalysis> {
  const count = await rotations(root, relative, signal), reducer = new CpuComparison(targets);
  await scanSegment(root, relative, reducer, signal);
  const parsed = posix.parse(relative);
  for (let index = 1n; index <= count; index++) {
    signal?.throwIfAborted();
    await scanSegment(root, posix.join(parsed.dir, parsed.name + '.' + String(index).padStart(6, '0') + '.mcap'), reducer, signal);
  }
  if (await rotations(root, relative, signal) !== count) throw new Error('CPU recording rotations changed during comparison');
  return reducer.result();
}

/** Read one run's original evidence. Missing or malformed sources remain per-side evidence. */
export async function analyzeCpu(root: string, side: Side, signal?: AbortSignal): Promise<CpuAnalysis> {
  signal?.throwIfAborted();
  root = join(root, side.id);
  const services = targets(root, side), recordings = side.detail.status?.recordings?.filter(output => output.cpu) ?? [];
  const missing = (reason: string): CpuAnalysis => ({ unavailable: reason, services: new Map(services.map(target =>
    [target.service, { runtime: 0n, elapsed: 0n, span: 0n, issues: [], unavailable: reason }])) });
  if (!recordings.length) return missing('CPU was not recorded');
  if (recordings.length !== 1) return missing('Multiple CPU recordings have ambiguous coverage');
  if (side.detail.status?.outcome?.recordingFinalized !== true) return missing('CPU recording was not finalized');
  try { return await scan(root, recordings[0].path!, services, signal); }
  catch (error) {
    signal?.throwIfAborted();
    return missing(error instanceof Error && /ENOENT|Missing/.test(error.message) ? 'CPU recording is missing'
      : 'CPU recording could not be read: ' + (error instanceof Error ? error.message : 'Unknown error'));
  }
}
const bounded = (text: string) => text.length > MAX_PRESENTATION_CHARS ? text.slice(0, MAX_PRESENTATION_CHARS) + '…' : text;
const evidence = (value?: ServiceAnalysis) => !value || value.unavailable ? 'Unavailable: ' + (value?.unavailable ?? 'Service was not recorded')
  : (value.issues.length || value.elapsed < value.span ? 'Partial' : 'Measured') + ': ' + seconds(value.elapsed) + ' of ' + seconds(value.span) +
    ' recorded s' + (value.issues.length ? '; ' + value.issues.join(', ') : '');
/** Exact measurement comparison and source-specific labels, units and evidence stay in this module. */
export function compareCpu(left: CpuAnalysis, right: CpuAnalysis) {
  const rows: typeof wire.ComparisonRow.prototype[] = [];
  const add = (name: string, label: string, values: (string | undefined)[], display: string[], coverage: string[], alwaysVisible = false) => {
    const unavailable = values.some(value => value === undefined);
    rows.push(wire.ComparisonRow.create({ group: wire.ComparisonGroup.COMPARISON_OUTCOME, name, label: bounded(label), alwaysVisible,
      result: unavailable ? wire.ComparisonResult.COMPARISON_UNAVAILABLE : values[0] === values[1] ? wire.ComparisonResult.COMPARISON_SAME : wire.ComparisonResult.COMPARISON_CHANGED,
      cells: values.map((value, side) => ({ values: [bounded(value ?? 'Unavailable')], displayValues: [bounded(display[side] ?? '')], evidence: bounded(coverage[side] ?? '') })),
    }));
  };
  for (const service of [...new Set([...left.services.keys(), ...right.services.keys()])].sort()) {
    const sides = [left.services.get(service), right.services.get(service)], coverage = sides.map(evidence);
    const measured = (format: (value: ServiceAnalysis) => string) => sides.map(value => value && !value.unavailable ? format(value) : undefined);
    const runtime = measured(value => seconds(value.runtime));
    const utilization = measured(value => String(Number(value.runtime) / Number(value.elapsed) * 100));
    const observed = measured(value => seconds(value.elapsed));
    const withUnit = (values: (string | undefined)[], unit: string) => values.map(value => value === undefined ? 'Unavailable' : value + unit);
    add(key(service, 'cpu_seconds'), service + ' · Measured CPU time', runtime, withUnit(runtime, ' s'), coverage);
    add(key(service, 'cpu_mean_percent'), service + ' · Mean CPU · core %', utilization, withUnit(utilization, '%'), coverage);
    const [a, b] = sides;
    if (a && b && !a.unavailable && !b.unavailable) {
      const equal = a.runtime * b.elapsed === b.runtime * a.elapsed;
      const row = rows.at(-1)!;
      row.result = equal ? wire.ComparisonResult.COMPARISON_SAME : wire.ComparisonResult.COMPARISON_CHANGED;
      if (!equal && utilization[0] === utilization[1]) row.cells.forEach((cell, side) => {
        const measurement = sides[side]!;
        cell.displayValues = [String(measurement.runtime * 100n) + '/' + String(measurement.elapsed) + ' %'];
      });
    }
    add(key(service, 'cpu_observed_seconds'), service + ' · Measured duration', observed, withUnit(observed, ' s'), coverage, true);
    add(key(service, 'cpu_coverage'), service + ' · CPU coverage', sides.map(value => value && !value.unavailable ? evidence(value) : undefined), coverage, [], true);
  }
  if (left.unavailable || right.unavailable) {
    const values = [left.unavailable, right.unavailable];
    add('cpu_capture', 'CPU recording', [undefined, undefined], values.map(value => value ? 'Unavailable: ' + value : 'Recorded'), [], true);
  }
  return rows;
}
/** CPU's private per-run measurements are erased only at the generic registration boundary. */
export const cpuModule = (root: string): AnalysisModule<CpuAnalysis> => ({
  id: 'cpu', label: 'CPU', analyze: (run, signal) => analyzeCpuInWorker(root, run, signal), compare: compareCpu,
});

/** One worker per active analyzer isolates CRC, packet decoding and reduction from HTTP requests.
 * Cancellation is cooperative; this promise settles after worker exit and all descriptor cleanup.
 */
function analyzeCpuInWorker(root: string, run: Readonly<Side>, signal: AbortSignal): Promise<CpuAnalysis> {
  signal.throwIfAborted();
  const worker = new Worker(new URL(import.meta.url.endsWith('.ts') ? './cpu-worker.ts' : './cpu-worker.js', import.meta.url), {
    workerData: { root, id: run.id, detail: wire.HistoryDetail.encode(run.detail).finish(), capture: execution.RunCapture.encode(run.capture).finish() },
  });
  return new Promise((resolve, reject) => {
    let result: CpuAnalysis | undefined, failure: Error | undefined;
    const abort = () => worker.postMessage('abort');
    signal.addEventListener('abort', abort, { once: true });
    if (signal.aborted) abort();
    worker.on('message', (message: { result?: CpuAnalysis; error?: string }) => {
      if (message.result) result = message.result;
      if (message.error) failure = new Error(message.error);
    });
    worker.once('error', error => { failure = error instanceof Error ? error : new Error(String(error)); });
    worker.once('exit', code => {
      signal.removeEventListener('abort', abort);
      if (signal.aborted) reject(signal.reason);
      else if (failure) reject(failure);
      else if (code !== 0 || !result) reject(new Error('CPU analysis worker exited without a completed result'));
      else resolve(result);
    });
  });
}
