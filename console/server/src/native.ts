import { createRequire } from 'node:module';
import type { doramcap } from './generated/console.cjs';
const require = createRequire(import.meta.url);
const codecs = require('./generated/console.cjs') as typeof import('./generated/console.cjs');

import type { NativeCapabilities } from '../native/index.js';
export type { NativeExecution, NativeCapabilities } from '../native/index.js';

export const capabilities = require('../build/console.node') as NativeCapabilities;
export const wire = codecs.doramcap.console.v1;
export const execution = codecs.doramcap.console.execution.v1;
export const compose = codecs.doramcap.compose.v1;
export const workflow = codecs.doramcap.workflow.v1;
export const viewer = codecs.doramcap.viewer.v1;
export const objectOptions = { longs: String, enums: String, defaults: true } as const;
export type Config = doramcap.console.v1.ConsoleConfig;
export type App = doramcap.console.v1.ISystemAppDefinition;
export type Project = doramcap.compose.v1.IComposeProject;
export type Output = doramcap.console.v1.IRecordingOutput;
export type Run = doramcap.console.execution.v1.IRunRequest;
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
