import { closeSync, fstatSync, readSync } from 'node:fs';
import { capabilities, browserError } from './native.js';
import { doramcap } from './generated/console.js';
import { LIMITS } from './generated/transport.js';

const { LogIndex } = doramcap.journal.v1;
const { Status, LiveMessage } = doramcap.console.v1;
export const LOGS_BUDGET_ERROR = 'Logs unavailable: log metadata exceeds the Console message budget';
type LogFields = { logs: doramcap.journal.v1.ILogIndex | null; logsError: string };
const fitsStatus = (fields: LogFields) =>
  LiveMessage.encode({ status: Status.encode(fields).finish() }).finish().length <= LIMITS.frameBytes;

/** Read the authoritative index; the caller checks its complete encoded response budget.
 * Missing indexes remain older runs without saved identities; other failures are explicit. */
export function readLogs(runRoot: string, fits: (fields: LogFields) => boolean = fitsStatus): LogFields {
  try {
    const fd = capabilities.openFile(runRoot, 'logs/index.pb');
    try {
      const size = fstatSync(fd).size;
      // Bound input allocation by the existing transport limit, including a growth check.
      if (size > LIMITS.frameBytes) throw new Error('Log index exceeds the Console message budget');
      const bytes = Buffer.alloc(size + 1);
      let length = 0;
      while (length < bytes.length) {
        const read = readSync(fd, bytes, length, bytes.length - length, null);
        if (!read) break;
        length += read;
      }
      if (length > size) throw new Error('Log index changed while reading');
      const logs = LogIndex.decode(bytes.subarray(0, length));
      for (const program of logs.programs) program.reason = browserError(program.reason ?? '');
      const fields = { logs, logsError: '' };
      return fits(fields) ? fields : { logs: null, logsError: LOGS_BUDGET_ERROR };
    } finally { closeSync(fd); }
  } catch (error) {
    // The native opener returns a textual OS error rather than a Node errno.
    if (/No such file or directory/.test(String(error))) return { logs: null, logsError: '' };
    return { logs: null, logsError: browserError(`Logs unavailable: ${String(error)}`) };
  }
}
