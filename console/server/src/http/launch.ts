import { once } from 'node:events';
import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';

export const DEFAULT_PORT = 43187;

export function portNumber(value: string): number {
  if (!/^\d+$/.test(value) || Number(value) < 1024 || Number(value) > 65535)
    throw new Error('Port must be an integer in 1024..65535');
  return Number(value);
}

/** Reserve and release one free loopback port for a child that binds it soon after. */
export async function availablePort(): Promise<number> {
  const server = createServer();
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const { port } = server.address() as AddressInfo;
  server.close();
  await once(server, 'close');
  return port;
}

/** Bind loopback directly; automatic selection retries occupied ports without a probe/bind race. */
export async function listenConsole(create: (origin: string) => Server,
  preferred = DEFAULT_PORT, browserPort?: number, fallback = true) {
  for (let port = preferred; port < Math.min(preferred + (fallback ? 100 : 1), 65536); port++) {
    const origin = `http://127.0.0.1:${browserPort ?? port}`;
    const server = create(origin);
    try {
      server.listen(port, '127.0.0.1');
      await once(server, 'listening');
      return { server, port, origin };
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'EADDRINUSE' || !fallback) throw error;
    }
  }
  throw new Error('No available Console port; choose another with --port');
}

/** Render commands for the browser computer; destinations are quoted as shell arguments. */
export function launchInstructions(origin: string, port: number, secret: string, ssh?: string): string {
  const browser = new URL(origin);
  const lines = ['Console is ready on this Linux host.'];
  if (ssh) {
    if (!/^[a-zA-Z0-9_][a-zA-Z0-9_.@:-]*$/.test(ssh))
      throw new Error('--ssh must be an SSH destination such as user@workstation');
    lines.push('', 'On the computer where your browser runs, open another terminal:', '',
      `  ssh -N -o ExitOnForwardFailure=yes -L 127.0.0.1:${browser.port}:127.0.0.1:${port} '${ssh}'`,
      '', 'Keep that terminal open, then open the link below.');
  }
  lines.push('', `Open: ${origin}/#access=${encodeURIComponent(secret)}`);
  return lines.join('\n');
}
