import { readFile } from 'node:fs/promises';
import { createConsoleServer } from './http.js';
import { openNative, wire } from './native.js';

const configPath = process.argv[2];
const assets = process.argv[3];
if (!configPath || !assets) throw new Error('usage: console-server CONFIG.pbtxt FRONTEND_DIST');
const native = await openNative(configPath);
try {
  const settings = wire.ServerSettings.decode(await native.settings());
  const secret = (await readFile(settings.accessSecretFile, 'utf8')).replace(/\r?\n$/, '');
  const server = createConsoleServer({ native, secret, origin: settings.publicOrigin, assets });
  const address = new URL(`http://${settings.listenAddress}`);
  await new Promise<void>((ok, fail) => { server.once('error', fail); server.listen(Number(address.port), address.hostname.replace(/^\[|\]$/g, ''), ok); });
  let stopping = false;
  const shutdown = async () => {
    if (stopping) return;
    stopping = true;
    const deadline = setTimeout(() => process.exit(1), 120_000);
    server.close();
    server.closeIdleConnections();
    try { await native.close(); clearTimeout(deadline); process.exitCode = 0; }
    catch { clearTimeout(deadline); process.exitCode = 1; }
  };
  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);
} catch (error) {
  await native.close();
  throw error;
}
