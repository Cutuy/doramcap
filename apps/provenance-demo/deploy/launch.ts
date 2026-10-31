/** Own the Dora flow, drain the workload, and retain its final summary. */
import { spawn, spawnSync } from 'node:child_process';
import { openSync, closeSync } from 'node:fs';
import { copyFile, writeFile } from 'node:fs/promises';

const root = '/opt/provenance', results = '/results', dora = '/usr/local/bin/dora', binary = '/usr/local/bin/provenance-demo', prefix = 'provenance-hosted';
/** Environment for children owned by the embedded Dora flow. */
function embeddedFlowEnvironment(): Record<string, string | undefined> {
  const environment: Record<string, string | undefined> = { ...process.env, PROVENANCE_PREFIX: prefix };
  for (const key of ['DORA_NODE_ID', 'DORA_DAEMON_LOCAL_LISTEN_PORT', 'DORA_ZENOH_CONFIG_OVERLAY']) delete environment[key];
  return environment;
}
const env = embeddedFlowEnvironment();
let stopping = false; process.on('SIGINT', () => { stopping = true; }); process.on('SIGTERM', () => { stopping = true; });
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
let launcherFd: number;
const command = (args: string[], timeout = 30_000) => { const result = spawnSync(dora, args, { env, timeout, stdio: ['ignore', launcherFd, launcherFd] }); if (result.error || result.status !== 0) throw result.error ?? new Error(`dora ${args[0]} failed: ${result.status}`); };
type Process = ReturnType<typeof spawn>;
const waitForExit = (child: Process, timeout: number) => new Promise<boolean>(resolve => {
  if (child.exitCode !== null || child.signalCode !== null) return resolve(true);
  const onClose = () => { clearTimeout(timer); resolve(true); };
  const timer = setTimeout(() => { child.off('close', onClose); resolve(false); }, timeout);
  child.once('close', onClose);
});
const stop = async (child: Process, timeout: number) => {
  if (child.exitCode === null && child.signalCode === null) {
    child.kill('SIGINT');
    if (!await waitForExit(child, timeout)) { child.kill('SIGKILL'); await waitForExit(child, 5000); throw new Error(`${child.spawnfile} did not stop in time`); }
  }
  if (child.exitCode !== 0) throw new Error(`${child.spawnfile} exited with status ${child.exitCode}`);
};
const summary = async () => { const response = await fetch('http://127.0.0.1:8090/api/summary', { signal: AbortSignal.timeout(3000) }); if (!response.ok) throw new Error(`summary: ${response.status}`); await writeFile(`${results}/summary.json`, JSON.stringify(await response.json(), null, 2) + '\n'); };
async function main() {
  const launcher = openSync(`${results}/launcher.log`, 'w', 0o600), monitorLog = openSync(`${results}/monitor.log`, 'w', 0o600), pipelineLog = openSync(`${results}/pipeline.log`, 'w', 0o600);
  launcherFd = launcher;
  const processes: Process[] = []; let coordinator = false, flow = false; let monitor: Process | undefined, pipeline: Process | undefined; const errors: string[] = [];
  const start = (file: string, args: string[], fd: number) => { const child = spawn(file, args, { env, stdio: ['ignore', fd, fd] }); child.on('error', error => { errors.push(error.message); }); processes.push(child); return child; };
  try {
    await copyFile(`${root}/flow.yml`, '/tmp/flow.yml');
    command(['up']); coordinator = true;
    command(['start', '/tmp/flow.yml', '--name', prefix, '--detach']); flow = true;
    monitor = start(binary, ['monitor', '--port', '8090', '--assets', `${root}/dist`, '--measurements', `${root}/measurements.pbtxt`], monitorLog);
    pipeline = start(binary, ['pipeline', '--duration', 'until-stopped'], pipelineLog);
    start('socat', ['TCP-LISTEN:8088,bind=0.0.0.0,reuseaddr,fork', 'TCP:127.0.0.1:8090'], launcher);
    while (!stopping) { if (errors.length || processes.some(p => p.exitCode !== null || p.signalCode !== null)) throw new Error('hosted process exited early'); await delay(200); }
    await stop(pipeline, 12_000); await delay(1000); await summary(); await stop(monitor, 12_000);
  } catch (error) {
    errors.push(String(error));
    if (pipeline && pipeline.exitCode === null && pipeline.signalCode === null) try { await stop(pipeline, 12_000); } catch (e) { errors.push(String(e)); }
    if (monitor && monitor.exitCode === null && monitor.signalCode === null) try { await summary(); } catch (e) { errors.push(String(e)); }
  } finally {
    for (const child of processes.reverse()) if (child.exitCode === null && child.signalCode === null) {
      child.kill('SIGTERM'); if (!await waitForExit(child, 5000)) { child.kill('SIGKILL'); await waitForExit(child, 5000); }
    }
    if (flow) try { command(['stop', '--name', prefix], 20_000); } catch (e) { errors.push(String(e)); }
    if (coordinator) try { command(['down'], 20_000); } catch (e) { errors.push(String(e)); }
    closeSync(launcher); closeSync(monitorLog); closeSync(pipelineLog);
  }
  if (stopping && !errors.length) await writeFile(`${results}/shutdown.complete`, 'complete\n');
  else throw new Error(errors.join('; ') || 'flow stopped unexpectedly');
}
main().catch(error => { console.error(error.message); process.exitCode = 1; });
