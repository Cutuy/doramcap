import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const codecs = require('./generated/console.cjs') as typeof import('./generated/console.cjs');
const addon = require('../build/console.node') as {
  open(path: string, version: number): Promise<NativeConsole>;
};

export interface NativeConsole {
  settings(): Promise<Buffer>;
  prepare(input: Buffer): Promise<Buffer>;
  start(input: Buffer): Promise<Buffer>;
  status(): Promise<Buffer>;
  update(cursor: Buffer): Promise<{data: Buffer; cursor: Buffer}>;
  stop(input: Buffer): Promise<Buffer>;
  openArtifact(runId: string, kind: string): Promise<number>;
  close(): Promise<void>;
}

export async function openNative(configPath: string): Promise<NativeConsole> {
  return addon.open(configPath, 1);
}

export const wire = codecs.doramcap.console.v1;
