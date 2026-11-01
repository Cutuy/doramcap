import { writeFile } from 'node:fs/promises';
import { parseArgs } from 'node:util';
import { launchInstructions, listenConsole, portNumber } from './launch.js';
import { createConsoleServer } from './http.js';
import { ConsoleService } from './service.js';
const { values, positionals: [configPath, assets] } = parseArgs({
  allowPositionals: true,
  options: { port: { type: 'string' }, 'browser-port': { type: 'string' }, ssh: { type: 'string' }, 'launch-file': { type: 'string' } },
});
const port = values.port ? portNumber(values.port) : undefined;
const browserPort = values['browser-port'] ? portNumber(values['browser-port']) : undefined;
// Validate destination before opening privileged resources.
if (values.ssh) launchInstructions('http://127.0.0.1:43187', 43187, '', values.ssh);
if (!configPath || !assets) throw new Error('usage: console-server CONFIG.pbtxt FRONTEND_DIST');
const { backend, config, secret } = await ConsoleService.open(configPath);
try {
  if (new URL(`http://${config.listenAddress}`).hostname !== '127.0.0.1')
    throw new Error('Console launch requires listen_address on 127.0.0.1');
  const preferred = port ?? portNumber(new URL(`http://${config.listenAddress}`).port);
  const launched = await listenConsole(origin => createConsoleServer({ backend, secret, origin, assets }),
    preferred, browserPort, port === undefined);
  const { server } = launched;
  const instructions = launchInstructions(launched.origin, launched.port, secret, values.ssh) + `\n\nProjects: ${config.projectRoot}`;
  if (values['launch-file']) {
    await writeFile(values['launch-file'], instructions, { mode: 0o600, flag: 'wx' });
    console.log('Console ready; use doramcap-console status to retrieve the access link.');
  } else console.log(instructions);
  let stopping = false;
  const shutdown = async () => {
    if (stopping) return;
    stopping = true;
    const deadline = setTimeout(() => process.exit(1), 120_000);
    server.close();
    server.closeIdleConnections();
    try {
      await backend.close();
      clearTimeout(deadline);
      process.exitCode = 0;
    } catch {
      clearTimeout(deadline);
      process.exitCode = 1;
    }
  };
  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);
} catch (error) {
  await backend.close();
  throw error;
}
