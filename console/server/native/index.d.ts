import type { doramcap } from '../src/generated/console.js';

/** Typed execution client; the TypeScript adapter owns native serialization. */
export interface NativeExecution {
  profiles(
    request: doramcap.console.execution.v1.ProfilesRequest,
  ): Promise<doramcap.console.execution.v1.Profiles>;
  /** Resolve the plan and configuration snapshot without starting execution. */
  inspect(
    request: doramcap.console.execution.v1.RunRequest,
  ): Promise<doramcap.console.execution.v1.Inspection>;
  /** Resolve after preflight accepts, before workflow completion. */
  start(
    run: doramcap.console.execution.v1.RunRequest,
    expected: doramcap.compose.v1.ConfigurationSnapshot,
  ): Promise<void>;
  /** Signal the active run's observation loop; completion is reported by poll(). */
  stop(runId: string): Promise<void>;
  /** Drain bounded events and retain the terminal result across polls. */
  poll(): doramcap.console.execution.v1.ExecutionUpdate;
  /** Stop the active workflow, await cleanup and join the owner thread. */
  close(): Promise<void>;
}

/** Native capabilities used by the TypeScript Console application. */
export interface NativeCapabilities {
  /** Parse ConsoleConfig pbtxt; validation belongs to the caller. */
  formatConfig(config: doramcap.console.v1.ConsoleConfig): string;
  parseConfig(text: string): doramcap.console.v1.ConsoleConfig;
  /** Expand a caller-approved Dora graph into native graph JSON bytes. */
  expandGraph(path: string): Promise<Buffer>;
  /** Hold an exclusive process lock until close or handle release. */
  lockFile(path: string): { close(): void };
  /** Open a regular file beneath root; the caller authorizes access and closes the fd. */
  openFile(root: string, relative: string): number;
  /** Open the execution owner with an explicit Docker socket and contract version 1. */
  open(socket: string, version: number): Promise<NativeExecution>;
}
