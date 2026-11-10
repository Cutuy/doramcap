import { spawn } from 'node:child_process';
import { constants as osConstants } from 'node:os';

export const SIGNAL_STATUS = { SIGHUP: 129, SIGINT: 130, SIGTERM: 143 } as const;
const SHUTDOWN_GRACE_MS = 5_000;
export class WorkerInterrupted extends Error {
  readonly signal: keyof typeof SIGNAL_STATUS;
  readonly status: number;
  constructor(signal: keyof typeof SIGNAL_STATUS) {
    super(`Worker interrupted by ${signal}`); this.signal = signal; this.status = SIGNAL_STATUS[signal];
  }
}
export type CommandResult = { code: number; stdout: string };
export type CommandOptions = { cwd?: string; capture?: boolean };
export type WorkerCommand = (program: string, args: readonly string[], options?: CommandOptions) => Promise<CommandResult>;
export type ProcessContext = {
  signal: AbortSignal; cwd: string; env: NodeJS.ProcessEnv; output: number;
  lockFd?: number; capture?: boolean; shutdownGraceMs?: number;
};
function signalCode(signal: NodeJS.Signals | null): number {
  return signal ? 128 + osConstants.signals[signal] : 1;
}
function groupAlive(pid: number): boolean {
  try { process.kill(-pid, 0); return true; }
  catch (error) { if ((error as NodeJS.ErrnoException).code === 'ESRCH') return false; throw error; }
}
/** Run an isolated process group; cancellation terminates descendants and waits for root/stdio cleanup. */
export async function runCommand(program: string, args: readonly string[], context: ProcessContext): Promise<CommandResult> {
  context.signal.throwIfAborted();
  return await new Promise<CommandResult>((resolve, reject) => {
    const child = spawn(program, [...args], { cwd: context.cwd, env: context.env, detached: true,
      stdio: ['ignore', context.capture ? 'pipe' : context.output, context.output, ...(context.lockFd === undefined ? [] : [context.lockFd])] });
    let stdout = '', closed = false, code = 1, forceExpired = false, settled = false;
    let forceTimer: ReturnType<typeof setTimeout> | undefined;
    const finish = (error?: unknown) => {
      if (settled) return;
      settled = true;
      if (forceTimer) clearTimeout(forceTimer);
      context.signal.removeEventListener('abort', abort);
      if (error) reject(error); else resolve({ code, stdout });
    };
    const killGroup = (signal: NodeJS.Signals) => {
      if (!child.pid) return;
      try { process.kill(-child.pid, signal); }
      catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ESRCH') throw error; }
    };
    const abort = () => {
      const reason = context.signal.reason;
      try { killGroup(reason instanceof WorkerInterrupted ? reason.signal : 'SIGTERM'); }
      catch (error) { finish(error); return; }
      forceTimer = setTimeout(() => {
        forceExpired = true;
        try { killGroup('SIGKILL'); }
        catch (error) { finish(error); return; }
        if (closed) finish(context.signal.reason);
      }, context.shutdownGraceMs ?? SHUTDOWN_GRACE_MS);
      try { if (closed && child.pid && !groupAlive(child.pid)) finish(reason); }
      catch (error) { finish(error); }
    };
    child.stdout?.setEncoding('utf8');
    child.stdout?.on('data', chunk => { stdout += chunk; });
    child.once('error', error => finish(context.signal.aborted ? context.signal.reason : error));
    child.once('close', (exitCode, signal) => {
      closed = true; code = exitCode ?? signalCode(signal);
      try {
        if (!context.signal.aborted) finish();
        else if (forceExpired || !child.pid || !groupAlive(child.pid)) finish(context.signal.reason);
      } catch (error) { finish(error); }
    });
    context.signal.addEventListener('abort', abort, { once: true });
    if (context.signal.aborted) abort();
  });
}
