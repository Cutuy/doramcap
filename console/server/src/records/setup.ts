import type { doramcap } from '../generated/console.js';
import { closeSync, fstatSync, readSync } from 'node:fs';
import { capabilities, compose } from '../native.js';

export type SetupRecord = typeof compose.SetupRecord.prototype;
const SETUP_FILE = 'experiment.setup.pb', LEGACY_FILE = 'experiment.input.json';
const MAX_SETUP_BYTES = 4 * 1024 * 1024;
const FIRST_TYPED_INVOCATION_VERSION = 2;
export const STOP_GRACE_PERIOD_VERSION = 3;
const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);

/** Read a regular private file with bounded allocation, rejecting size changes and unsafe paths. */
function read(root: string, relative: string): Buffer | null {
  try {
    const fd = capabilities.openFile(root, relative);
    try {
      const stat = fstatSync(fd);
      if (!stat.isFile() || stat.size > MAX_SETUP_BYTES) return null;
      const bytes = Buffer.alloc(stat.size);
      let offset = 0;
      while (offset < bytes.length) {
        const count = readSync(fd, bytes, offset, bytes.length - offset, null);
        if (!count) return null;
        offset += count;
      }
      return fstatSync(fd).size === bytes.length ? bytes : null;
    } finally { closeSync(fd); }
  } catch { return null; }
}

/** Convert only released JSON input evidence; legacy invocations cannot establish a normalized verdict. */
function legacy(bytes: Buffer, id: string): SetupRecord | null {
  const model: unknown = JSON.parse(bytes.toString('utf8'));
  if (!object(model) || !object(model.services)) return null;
  const record: SetupRecord = compose.SetupRecord.create({ runId: id, complete: true });
  const field = (value: unknown): doramcap.compose.v1.IInvocationValue | null => {
    if (value == null) return null;
    if (typeof value === 'string') return value ? { text: value } : { arguments: { values: [] } };
    if (Array.isArray(value) && value.every(arg => typeof arg === 'string')) return { arguments: { values: value } };
    throw new Error('Invalid legacy invocation');
  };
  for (const [name, service] of Object.entries(model.services)) {
    if (!object(service)) return null;
    if (service.stop_signal != null && typeof service.stop_signal !== 'string') return null;
    const captured = { command: field(service.command), entrypoint: field(service.entrypoint), stopSignal: service.stop_signal ?? '' };
    const environment = Object.fromEntries(Object.entries(object(service.environment) ? service.environment : {})
      .map(([name, value]) => [name, String(value ?? '')]));
    const binds = (Array.isArray(service.volumes) ? service.volumes : []).filter(object)
      .filter(mount => mount.type === 'bind' && typeof mount.source === 'string' && typeof mount.target === 'string')
      .map(mount => ({ source: mount.source as string, target: mount.target as string }));
    record.services[name] = compose.SetupService.fromObject({ environment, binds, captured });
  }
  for (const kind of ['configs', 'secrets'] as const) {
    if (object(model[kind])) for (const [name, value] of Object.entries(model[kind]))
      if (object(value) && typeof value.file === 'string') record[kind][name] = value.file;
  }
  return record;
}

/** Read one complete typed snapshot; unreadable/newer evidence falls back to released input JSON. */
export function readSetupRecord(root: string, id: string, directory = id): SetupRecord | null {
  const prefix = directory ? directory + '/' : '';
  const bytes = read(root, prefix + SETUP_FILE);
  if (bytes) try {
    const record = compose.SetupRecord.decode(bytes);
    if (record.version >= FIRST_TYPED_INVOCATION_VERSION && record.version <= STOP_GRACE_PERIOD_VERSION && record.runId === id && record.complete && Object.keys(record.services).length
      && Object.entries(record.services).every(([name, service]) => name && service.captured != null && service.normalized != null
        && (service.binds ?? []).every(bind => bind.source && bind.target))) return record;
  } catch { /* Legacy input can still supply captured setup facts. */ }
  const previous = read(root, prefix + LEGACY_FILE);
  try { const record = previous ? legacy(previous, id) : null; return record; } catch { return null; }
}
