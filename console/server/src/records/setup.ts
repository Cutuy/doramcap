import type { doramcap } from '../generated/console.js';
import { closeSync, fstatSync, readSync } from 'node:fs';
import { capabilities, compose } from '../native.js';

export type SetupRecord = typeof compose.SetupRecord.prototype;
const SETUP_FILE = 'experiment.setup.pb', LEGACY_FILE = 'experiment.input.json';
const MAX_SETUP_BYTES = 4 * 1024 * 1024;
const SETUP_VERSION = 1;
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

const validValue = (value: doramcap.compose.v1.IInvocationValue | null | undefined, argumentsAllowed: boolean) => {
  if (!value) return false;
  const cases = ['unset', 'nullValue', 'text', 'arguments'].filter(key => Object.hasOwn(value, key));
  if (cases.length !== 1) return false;
  if (cases[0] === 'unset') return value.unset === true;
  if (cases[0] === 'nullValue') return value.nullValue === true;
  if (cases[0] === 'text') return typeof value.text === 'string';
  return argumentsAllowed && value.arguments != null && (value.arguments.values ?? []).every(arg => typeof arg === 'string');
};
const validInvocation = (value: doramcap.compose.v1.IInvocation | null | undefined) => value != null
  && validValue(value.command, true) && validValue(value.entrypoint, true) && validValue(value.stopSignal, false);

/** Convert only released JSON input evidence; legacy invocations cannot establish a normalized verdict. */
function legacy(bytes: Buffer, id: string): SetupRecord | null {
  const model: unknown = JSON.parse(bytes.toString('utf8'));
  if (!object(model) || !object(model.services)) return null;
  const record: SetupRecord = compose.SetupRecord.create({ runId: id, complete: true });
  const field = (value: unknown): doramcap.compose.v1.IInvocationValue => value === undefined ? { unset: true }
    : value === null ? { nullValue: true } : typeof value === 'string' ? { text: value }
      : Array.isArray(value) && value.every(arg => typeof arg === 'string') ? { arguments: { values: value } }
        : {};
  for (const [name, service] of Object.entries(model.services)) {
    if (!object(service)) return null;
    const captured = { command: field(service.command), entrypoint: field(service.entrypoint), stopSignal: field(service.stop_signal) };
    if (!validInvocation(captured)) return null;
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

/** Normalize run-local setup paths while keeping captured and canonical invocation values exact. */
function normalize(record: SetupRecord, id: string) {
  const path = (value: string) => value.replaceAll(id, '@run').replaceAll('reviews/@run/resolved', 'runs/@run');
  for (const service of Object.values(record.services)) {
    for (const [name, value] of Object.entries(service.environment ?? {})) service.environment![name] = path(value);
    for (const bind of service.binds ?? []) { bind.source = path(bind.source ?? ''); bind.target = path(bind.target ?? ''); }
  }
  for (const kind of ['configs', 'secrets'] as const)
    for (const [name, value] of Object.entries(record[kind])) record[kind][name] = path(value);
  return record;
}

/** Read one complete typed snapshot; unreadable/newer evidence falls back to released input JSON. */
export function readSetupRecord(root: string, id: string, directory = id): SetupRecord | null {
  const bytes = read(root, `${directory}/${SETUP_FILE}`);
  if (bytes) try {
    const record = compose.SetupRecord.decode(bytes);
    if (record.version === SETUP_VERSION && record.runId === id && record.complete && Object.keys(record.services).length
      && Object.entries(record.services).every(([name, service]) => name && validInvocation(service.captured) && validInvocation(service.normalized)
        && (service.binds ?? []).every(bind => bind.source && bind.target))) return normalize(record, id);
  } catch { /* Legacy input can still supply captured setup facts. */ }
  const previous = read(root, `${directory}/${LEGACY_FILE}`);
  try { const record = previous ? legacy(previous, id) : null; return record ? normalize(record, id) : null; } catch { return null; }
}
