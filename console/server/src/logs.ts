import { closeSync, fstatSync, readFileSync } from 'node:fs';
import { capabilities, browserError } from './native.js';
import { doramcap } from './generated/console.js';

const { LogIndex } = doramcap.journal.v1;
// The complete list must fit the saved History and browser frame budgets. Oversize is explicit.
const MAX_INDEX_BYTES = 1024 * 1024;

/** Read the authoritative list through the run-confined opener; absent indexes are older runs. */
export function readLogs(runRoot: string) {
  try {
    const fd = capabilities.openFile(runRoot, 'logs/index.pb');
    try {
      if (fstatSync(fd).size > MAX_INDEX_BYTES) throw new Error('Log index exceeds the Console display budget');
      const bytes = readFileSync(fd);
      if (bytes.length > MAX_INDEX_BYTES) throw new Error('Log index exceeds the Console display budget');
      const logs = LogIndex.decode(bytes);
      for (const program of logs.programs) program.reason = browserError(program.reason ?? '');
      return { logs, logsError: '' };
    } finally { closeSync(fd); }
  } catch (error) {
    // The native opener returns a textual OS error rather than a Node errno.
    if (/No such file or directory/.test(String(error))) return { logs: null, logsError: '' };
    return { logs: null, logsError: browserError(`Logs unavailable: ${String(error)}`) };
  }
}
