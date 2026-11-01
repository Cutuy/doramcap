import { createRequire } from 'node:module';
import { doramcap } from './generated/console.js';
const require = createRequire(import.meta.url);

import type { NativeCapabilities, NativeExecution } from '../native/index.js';
export type { NativeExecution, NativeCapabilities } from '../native/index.js';

export const wire = doramcap.console.v1;
export const execution = doramcap.console.execution.v1;
export const compose = doramcap.compose.v1;
export const workflow = doramcap.workflow.v1;
export const viewer = doramcap.viewer.v1;
// The addon ABI stays private; application code uses the generated message types.
type RawExecution = Omit<NativeExecution, 'profiles' | 'inspect' | 'start' | 'poll'> & {
  profiles(input: Buffer): Promise<Buffer>;
  inspect(input: Buffer): Promise<Buffer>;
  start(input: Buffer): Promise<Buffer>;
  poll(): Buffer;
};
type RawCapabilities = Omit<NativeCapabilities, 'parseConfig' | 'formatConfig' | 'open'> & {
  parseConfig(text: string): Buffer;
  formatConfig(bytes: Buffer): string;
  open(socket: string, version: number): Promise<RawExecution>;
};
const addon = require('../build/console.node') as RawCapabilities;

export const capabilities: NativeCapabilities = {
  formatConfig: (config) => addon.formatConfig(Buffer.from(wire.ConsoleConfig.encode(config).finish())),
  parseConfig: (text) => wire.ConsoleConfig.decode(addon.parseConfig(text)),
  expandGraph: (path) => addon.expandGraph(path),
  lockFile: (path) => addon.lockFile(path),
  openFile: (root, relative) => addon.openFile(root, relative),
  async open(socket, version) {
    const owner = await addon.open(socket, version);
    return {
      profiles: async (request) =>
        execution.Profiles.decode(
          await owner.profiles(Buffer.from(execution.ProfilesRequest.encode(request).finish())),
        ),
      inspect: async (request) =>
        execution.Inspection.decode(
          await owner.inspect(Buffer.from(execution.RunRequest.encode(request).finish())),
        ),
      start: (run, expected) =>
        owner
          .start(Buffer.from(execution.StartRequest.encode({ run, expected }).finish()))
          .then(() => undefined),
      stop: (runId) => owner.stop(runId),
      poll: () => execution.ExecutionUpdate.decode(owner.poll()),
      close: () => owner.close(),
    };
  },
};

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
  stop(input: Buffer): Promise<Buffer>;
  openArtifact(runId: string, kind: string): Promise<number>;
  close(): Promise<void>;
}
