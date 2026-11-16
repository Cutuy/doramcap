import { parentPort, workerData } from 'node:worker_threads';
import { wire } from '../../../native.js';
import { encodeAnalysis } from './protocol.js';
import { analyzeCpu } from './analyzer.js';

const port = parentPort!;
const controller = new AbortController();
port.on('message', message => { if (message === 'abort') controller.abort(); });
try {
  const result = await analyzeCpu(wire.AnalysisRun.decode(workerData), controller.signal);
  // Deliver queued cancellation before publishing a completed result.
  await new Promise<void>(resolve => setImmediate(resolve));
  controller.signal.throwIfAborted();
  port.postMessage({ result: encodeAnalysis(result) });
} catch (error) {
  port.postMessage({ error: error instanceof Error ? error.message : String(error) });
} finally { port.close(); }
