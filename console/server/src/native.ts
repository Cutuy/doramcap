import { createRequire } from 'node:module';
import { doramcap } from './generated/console.js';
const require = createRequire(import.meta.url);

import type { NativeCapabilities, NativeExecution } from '../native/index.js';
export type { NativeExecution, NativeCapabilities } from '../native/index.js';

export const wire = doramcap.console.v1;
export const execution = doramcap.console.execution.v1;
export const compose = doramcap.compose.v1;
export const composePlan = doramcap.compose_plan.v1;
export const launch = doramcap.launch.v1;
export const systemd = doramcap.systemd_plan.v1;
export const workflow = doramcap.workflow.v1;
export const live = doramcap.console.live.v1;
export const systemConfig = doramcap.system_config.v1;
// The addon ABI stays private; application code uses the generated message types.
type RawExecution = Omit<NativeExecution, 'composeProfiles' | 'inspect' | 'start' | 'poll'> & {
  composeProfiles(input: Buffer): Promise<Buffer>;
  inspect(input: Buffer): Promise<Buffer>;
  start(input: Buffer): Promise<Buffer>;
  poll(): Buffer;
};
type RawCapabilities = Omit<NativeCapabilities, 'parseConfig' | 'parseManifest' | 'formatConfig' | 'open'> & {
  parseConfig(text: string): Buffer;
  parseManifest(text: string): Buffer;
  formatConfig(bytes: Buffer): string;
  open(socket: string, version: number): Promise<RawExecution>;
};
const addon = require('../build/console.node') as RawCapabilities;

/** Decode designed Review rejections as errors with structured cause facts. */
function reviewResult<T extends { rejection?: doramcap.console.execution.v1.IComposeRejection | null }>(result: T): T {
  if (result.rejection) {
    const { reason, subject, detail } = result.rejection;
    throw Object.assign(new Error(detail ?? ''), { reason: reason ?? '', subject: subject ?? '' });
  }
  return result;
}

export const capabilities: NativeCapabilities = {
  formatConfig: (config) => addon.formatConfig(Buffer.from(wire.ConsoleConfig.encode(config).finish())),
  parseConfig: (text) => wire.ConsoleConfig.decode(addon.parseConfig(text)),
  parseManifest: (text) => launch.AppManifest.decode(addon.parseManifest(text)),
  expandGraph: (path) => addon.expandGraph(path),
  lockFile: (path) => addon.lockFile(path),
  openFile: (root, relative) => addon.openFile(root, relative),
  async open(socket, version) {
    const owner = await addon.open(socket, version);
    return {
      composeProfiles: async (request) =>
        reviewResult(execution.ComposeProfiles.decode(
          await owner.composeProfiles(Buffer.from(execution.ComposeProfilesRequest.encode(request).finish())),
        )),
      inspect: async (request) =>
        reviewResult(execution.Inspection.decode(
          await owner.inspect(Buffer.from(execution.RunRequest.encode(request).finish())),
        )),
      start: (request) =>
        owner
          .start(Buffer.from(execution.StartRequest.encode(request).finish()))
          .then(() => undefined),
      stop: (runId) => owner.stop(runId),
      poll: () => execution.ExecutionUpdate.decode(owner.poll()),
      close: () => owner.close(),
    };
  },
};

/** The end of an error, where its cause is, for a browser message. */
export const browserError = (error: string) => error.slice(-execution.Limit.MAX_ERROR_CHARS);

export const objectOptions = { longs: String, enums: String, defaults: true } as const;
export type Config = doramcap.console.v1.ConsoleConfig;
export type App = doramcap.console.v1.ISystemAppDefinition;
export type Project = doramcap.compose.v1.IComposeProject;
export type Output = doramcap.console.v1.IRecordingOutput;
export type Run = doramcap.console.execution.v1.RunRequest;
export type Snapshot = doramcap.console.v1.Status;

/** Browser-facing operations implemented by the TypeScript application service. */
export interface ConsoleBackend {
  prepare(input: Buffer): Promise<Buffer>;
  start(input: Buffer): Promise<Buffer>;
  status(): Promise<Buffer>;
  update(cursor: Buffer): Promise<{ data: Buffer; cursor: Buffer }>;
  /** Replay latest bridge frames, then forward live ones; returns the unsubscribe function. */
  frames(consume: (frame: Buffer) => void): () => void;
  stop(input: Buffer): Promise<Buffer>;
  openArtifact(runId: string, kind: string): Promise<number>;
  /** Authorized, backpressured JSON Lines; abort closes in-flight manifest descriptors. */
  fileChanges(runId: string, signal: AbortSignal): AsyncIterable<Buffer>;
  /** Encoded HistoryQuery to encoded HistoryPage; a failed index yields a degraded page. */
  history(input: Buffer): Promise<Buffer>;
  /** Encoded HistoryDetail; rejects with `Missing` for an unknown id. */
  historyRun(runId: string): Promise<Buffer>;
  /** Encoded HistoryComparisonRequest to encoded HistoryComparison; rejects with `Missing` for an unknown id. */
  historyCompare(input: Buffer): Promise<Buffer>;
  close(): Promise<void>;
}
