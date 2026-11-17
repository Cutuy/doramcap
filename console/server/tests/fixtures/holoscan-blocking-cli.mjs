#!/usr/bin/env node
import { createConnection } from 'node:net';
import { join } from 'node:path';

// Keep the direct CLI process blocked on this open connection after announcing its PID.
const socket = createConnection(join(process.cwd(), 'catalog-close.sock'));
socket.on('connect', () => socket.write(`${process.pid}\n`));
socket.setEncoding('utf8');
let request = '';
socket.on('data', chunk => {
  request += chunk;
  if (request.includes('exit\n')) process.exit(0);
});
