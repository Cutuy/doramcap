/** Protobuf execution transport implemented by the Rust addon. */
export interface NativeExecution {
  /** Encode ProfilesRequest; decode Profiles. */
  profiles(input: Buffer): Promise<Buffer>;
  /** Encode RunRequest; decode Inspection containing the plan and configuration snapshot. */
  inspect(input: Buffer): Promise<Buffer>;
  /** Encode StartRequest; resolve after preflight accepts, before workflow completion. */
  start(input: Buffer): Promise<Buffer>;
  /** Signal the active run's observation loop; completion is reported by poll(). */
  stop(runId: string): Promise<void>;
  /** Decode ExecutionUpdate; drains bounded events and retains the terminal result. */
  poll(): Buffer;
  /** Stop the active workflow, await cleanup and join the owner thread. */
  close(): Promise<void>;
}

/** Native capabilities used by the TypeScript Console application. */
export interface NativeCapabilities {
  /** Decode ConsoleConfig pbtxt into protobuf bytes; validation belongs to the caller. */
  parseConfig(text: string): Buffer;
  /** Expand a caller-approved Dora graph into native graph JSON bytes. */
  expandGraph(path: string): Promise<Buffer>;
  /** Hold an exclusive process lock until close or handle release. */
  lockFile(path: string): { close(): void };
  /** Open a regular file beneath root; the caller authorizes access and closes the fd. */
  openFile(root: string, relative: string): number;
  /** Open the execution owner with an explicit Docker socket and contract version 1. */
  open(socket: string, version: number): Promise<NativeExecution>;
}
