import { closeSync, createReadStream, fstatSync, readFileSync } from 'node:fs';
import { posix } from 'node:path';
import { finished } from 'node:stream/promises';
import { McapStreamReader } from '@mcap/core';
import { capabilities, launch, telemetry } from '../native.js';
import type { doramcap } from '../generated/console.js';
import type { Facts, Side } from './compare.js';

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
const unavailable = (value: string) => ({ value, unavailable: true });
const key = (service: string, field: string) => 'service.' + service + '.' + field;
/** Preserve integer nanoseconds while expressing seconds in comparison values. */
const seconds = (ns: bigint) => String(ns / NS_PER_SECOND) + '.' + String(ns % NS_PER_SECOND).padStart(9, '0');

/** Read original evidence beneath the run; never use run-ID-normalized comparison input. */
function readEvidence(root: string, relative: string) {
  const fd = capabilities.openFile(root, relative);
  try {
    if (fstatSync(fd).size > MAX_MANIFEST_BYTES) throw new Error('Saved service placement exceeds the manifest limit');
    return readFileSync(fd);
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
  try { manifest = JSON.parse(readEvidence(root, side.id + '/experiment.input.json').toString('utf8')); } catch { /* Native runs have no Compose manifest. */ }
  if (manifest?.services && typeof manifest.services === 'object') {
    for (const service of Object.keys(manifest.services)) names.add(service);
    return [...names].map(service => {
      const parent = manifest!.services[service]?.cgroup_parent;
      if (typeof parent !== 'string') return { service };
      const slice = GENERATED_PARENT.exec(parent), directory = GENERATED_DIRECTORY.exec(parent);
      // A custom/shared parent does not establish exclusive attribution to this service.
      const matches = slice ? slice[2].replaceAll('\\x2d', '-') === service : directory?.[2] === service;
      return { service, ...(matches ? { path: slice ? slicePath(parent) : parent } : {}) };
    });
  }
  // Native unit names are repository-owned even when a user's Slice= puts them elsewhere.
  try {
    const identity = launch.SystemAppCgroup.decode(readEvidence(root, side.id + '/system-app-cgroup.pb'));
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
  facts(): Facts {
    const facts: Facts = new Map();
    for (const target of this.targets) {
      const value = this.measurements.get(target.service)!;
      if (!value.elapsed) {
        facts.set(key(target.service, 'cpu_coverage'), unavailable(!target.path && !target.unit ? 'Unavailable: service cgroup was not recorded'
          : 'Unavailable: no valid measured CPU interval'));
        continue;
      }
      facts.set(key(target.service, 'cpu_seconds'), { value: seconds(value.runtime) });
      facts.set(key(target.service, 'cpu_mean_percent'), { value: String(Number(value.runtime) / Number(value.elapsed) * 100) });
      facts.set(key(target.service, 'cpu_observed_seconds'), { value: seconds(value.elapsed) });
      const issues = [...value.issues].sort();
      facts.set(key(target.service, 'cpu_coverage'), { value: (issues.length || value.elapsed < this.span ? 'Partial' : 'Measured') +
        ': ' + seconds(value.elapsed) + ' of ' + seconds(this.span) + ' recorded s' + (issues.length ? '; ' + issues.join(', ') : '') });
    }
    if (!this.snapshotCount) facts.set('cpu_capture', unavailable('Unavailable: recording contains no CPU snapshots'));
    return facts;
  }
}

/** Read one completed CPU recording incrementally; the caller's signal cancels and closes its descriptor. */
async function scan(root: string, relative: string, targets: Target[], signal?: AbortSignal): Promise<Facts> {
  signal?.throwIfAborted();
  const fd = capabilities.openFile(root, relative);
  let before;
  try { before = fstatSync(fd); } catch (error) { closeSync(fd); throw error; }
  const stream = createReadStream('', { fd, autoClose: true, signal });
  const reader = new McapStreamReader({ validateCrcs: true });
  const reducer = new CpuComparison(targets);
  const schemas = new Map<number, string>(), channels = new Map<number, { schemaId: number; messageEncoding: string }>();
  try {
    for await (const chunk of stream) {
      signal?.throwIfAborted();
      reader.append(chunk);
      for (let record; (record = reader.nextRecord());) {
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
      const after = fstatSync(current);
      if (before.dev !== after.dev || before.ino !== after.ino || before.size !== after.size || before.mtimeMs !== after.mtimeMs)
        throw new Error('CPU recording changed during comparison');
    } finally { closeSync(current); }
    return reducer.facts();
  } finally { stream.destroy(); await finished(stream).catch(() => {}); }
}

/** CPU measurements are derived per request; missing/failed evidence cannot replace saved execution outcomes. */
export async function cpuComparison(root: string, side: Side, signal?: AbortSignal): Promise<Facts> {
  signal?.throwIfAborted();
  const services = targets(root, side), recordings = side.detail.status?.recordings?.filter(output => output.cpu) ?? [];
  const missing = (reason: string): Facts => new Map([
    ['cpu_capture', unavailable('Unavailable: ' + reason)],
    ...services.map(target => [key(target.service, 'cpu_coverage'), unavailable('Unavailable: ' + reason)] as const),
  ]);
  if (!recordings.length) return missing('CPU was not recorded');
  if (recordings.length !== 1) return missing('multiple CPU recordings have ambiguous coverage');
  if (side.detail.status?.outcome?.recordingFinalized !== true) return missing('CPU recording was not finalized');
  try { return await scan(root, side.id + '/' + recordings[0].path, services, signal); }
  catch (error) {
    signal?.throwIfAborted();
    return missing(error instanceof Error && /ENOENT|Missing/.test(error.message) ? 'CPU recording is missing'
      : 'CPU recording could not be read: ' + (error instanceof Error ? error.message : 'unknown error'));
  }
}
