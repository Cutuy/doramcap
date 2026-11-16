import { parentPort, workerData } from 'node:worker_threads';
import { execution, wire } from '../native.js';
import { analyzeCpu } from './cpu-comparison.js';

const port = parentPort!;
const controller = new AbortController();
port.on('message', message => { if (message === 'abort') controller.abort(); });
try {
  const result = await analyzeCpu(workerData.root, { id: workerData.id, manifest: null,
    detail: wire.HistoryDetail.decode(workerData.detail), capture: execution.RunCapture.decode(workerData.capture) }, controller.signal);
  // Deliver queued cancellation before publishing a completed result.
  await new Promise<void>(resolve => setImmediate(resolve));
  controller.signal.throwIfAborted();
  port.postMessage({ result });
} catch (error) {
  port.postMessage({ error: error instanceof Error ? error.message : String(error) });
} finally { port.close(); }
