import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { doramcap } from './generated/console.js';

const { SourceState } = doramcap.console.execution.v1;

const run = promisify(execFile);
/** Bounds each `git` command, so a slow repository cannot hold up a run. */
const GIT_DEADLINE_MS = 5000;
const git = async (directory: string, ...args: string[]) =>
  (await run('git', ['-c', `safe.directory=${directory}`, '-C', directory, ...args], { timeout: GIT_DEADLINE_MS })).stdout.trim();

/** The project's commit and source state; failed commands leave the commit unset or the state unknown. */
export async function sourceState(directory: string): Promise<{ sourceCommit?: string; sourceState: doramcap.console.execution.v1.SourceState }> {
  const [commit, changes] = await Promise.allSettled([git(directory, 'rev-parse', 'HEAD'), git(directory, 'status', '--porcelain')]);
  return {
    ...(commit.status === 'fulfilled' ? { sourceCommit: commit.value } : {}),
    sourceState: changes.status === 'fulfilled'
      ? changes.value === '' ? SourceState.SOURCE_STATE_CLEAN : SourceState.SOURCE_STATE_DIRTY
      : SourceState.SOURCE_STATE_UNKNOWN,
  };
}
