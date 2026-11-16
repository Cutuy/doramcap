import { Worker } from 'node:worker_threads';
import { wire } from '../../../native.js';
import type { AnalysisRun } from '../../module.js';
import type { CpuAnalysis } from './model.js';
import { decodeAnalysis } from './protocol.js';

/** One worker per active analyzer isolates CRC, packet decoding and reduction from HTTP requests.
 * Cancellation is cooperative; this promise settles after worker exit and all descriptor cleanup.
 */
export function analyzeCpuInWorker(run: Readonly<AnalysisRun>, signal: AbortSignal): Promise<CpuAnalysis> {
  signal.throwIfAborted();
  const worker = new Worker(new URL(import.meta.url.endsWith('.ts') ? './worker.ts' : './worker.js', import.meta.url), {
    // Native safe-open descriptors have explicit cleanup outside Node's fs.open tracking.
    trackUnmanagedFds: false,
    workerData: wire.AnalysisRun.encode(run).finish(),
  });
  return new Promise((resolve, reject) => {
    let result: CpuAnalysis | undefined, failure: Error | undefined;
    const abort = () => worker.postMessage('abort');
    signal.addEventListener('abort', abort, { once: true });
    if (signal.aborted) abort();
    worker.on('message', (message: { result?: Uint8Array; error?: string }) => {
      if (message.result) { try { result = decodeAnalysis(message.result); } catch (error) { failure = error instanceof Error ? error : new Error(String(error)); } }
      if (message.error) failure = new Error(message.error);
    });
    worker.once('error', error => { failure = error instanceof Error ? error : new Error(String(error)); });
    worker.once('exit', code => {
      signal.removeEventListener('abort', abort);
      if (signal.aborted) reject(signal.reason);
      else if (failure) reject(failure);
      else if (code !== 0 || !result) reject(new Error('CPU analysis worker exited without a completed result'));
      else resolve(result);
    });
  });
}
