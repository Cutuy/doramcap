import assert from 'node:assert/strict';
import { execaSync } from 'execa';

const CONNECT_SECONDS = 10, KEEPALIVE_SECONDS = 10, KEEPALIVE_FAILURES = 3;
export const sshOptions = ['-o', 'BatchMode=yes', '-o', `ConnectTimeout=${CONNECT_SECONDS}`, '-o', `ServerAliveInterval=${KEEPALIVE_SECONDS}`, '-o', `ServerAliveCountMax=${KEEPALIVE_FAILURES}`];
/** Accept an SSH account/host name, rejecting options and remote-shell syntax. */
export function workerHost(value: string): string {
  assert(/^(?:[a-zA-Z0-9_][a-zA-Z0-9_.-]*@)?[a-zA-Z0-9][a-zA-Z0-9.-]*$/.test(value), `Invalid SSH worker: ${value}`);
  return value;
}
/** Execute a controlled remote command with key-only authentication. */
export function workerSsh(host: string, command: string, inherit = false): string {
  return execaSync('ssh', [...sshOptions, workerHost(host), command], inherit ? { stdio: 'inherit' } : { maxBuffer: Infinity }).stdout ?? '';
}
