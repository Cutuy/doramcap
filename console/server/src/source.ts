import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const run = promisify(execFile);
/** Bounds each `git` command, so a slow repository cannot hold up a run. */
const GIT_DEADLINE_MS = 5000;
const git = async (directory: string, ...args: string[]) =>
  (await run('git', ['-c', `safe.directory=${directory}`, '-C', directory, ...args], { timeout: GIT_DEADLINE_MS })).stdout.trim();

/** The project's commit and whether it has local changes. Each stays unset when its own command fails. */
export async function sourceState(directory: string): Promise<{ sourceCommit?: string; sourceDirty?: boolean }> {
  const [commit, changes] = await Promise.allSettled([git(directory, 'rev-parse', 'HEAD'), git(directory, 'status', '--porcelain')]);
  return {
    ...(commit.status === 'fulfilled' ? { sourceCommit: commit.value } : {}),
    ...(changes.status === 'fulfilled' ? { sourceDirty: changes.value !== '' } : {}),
  };
}
