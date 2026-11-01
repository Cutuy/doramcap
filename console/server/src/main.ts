import { createConsoleServer } from './http.js';
import { ConsoleService } from './service.js';
const [configPath, assets] = process.argv.slice(2);
if (!configPath || !assets) throw new Error('usage: console-server CONFIG.pbtxt FRONTEND_DIST');
const { backend, config, secret } = await ConsoleService.open(configPath);
try {
  const server = createConsoleServer({ backend, secret, origin: config.publicOrigin, assets });
  const address = new URL(`http://${config.listenAddress}`);
  await new Promise<void>((ok, fail) => {
    server.once('error', fail);
    server.listen(Number(address.port), address.hostname.replace(/^\[|\]$/g, ''), ok);
  });
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
