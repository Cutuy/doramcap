# Console over Tailscale

For remote playground sessions, provide a direct Tailscale URL with embedded access:
`http://TAILSCALE_IP:43187/#access=TOKEN`. The browser signs in automatically.
Both computers need access to the existing tailnet. Keep Tailscale settings unchanged.
Build/setup: [Console README](../README.md).

## Prepare the host

- Use the requested checkout and revision. Build unprivileged with the Node version in
  [`.node-version`](../../.node-version), locked npm dependencies and the native addon.
- Configure absolute project, data and app-manifest paths using
  [Console configuration](configuration.md). Let the privileged Console process create
  its mode-0700 data directory, or provide one owned by that process's user.
- For a writable playground bind, match the container's user to the source folder's
  owner. The CPU-burn sample drops all capabilities; root in that container cannot
  bypass the host folder's permissions.
- Keep the launcher and relay running independently of the hosting command's terminal
  or chat. Use detached processes or an existing supervisor. Keep access logs private.

## Start Console on loopback

Use the normal launcher on the workload host. Follow the repository's
[privileged-command procedure](../../AGENTS.md#capability-required-tests) for noninteractive sudo.
Create a private launch directory first; `HOST_STATE` below names that directory.

```sh
umask 077
HOST_STATE=$(mktemp -d /tmp/console-host.XXXXXX)
sudo sh -c 'exec </dev/null; exec setsid --fork "$@"' console-host \
  "$(command -v node)" "$PWD/console/server/dist/main.js" \
  /absolute/console.pbtxt "$PWD/console/frontend/dist" --port 43187 \
  > "$HOST_STATE/server.log" 2>&1
```

Wait for the `Open:` line in `server.log`. Preserve its complete link privately.
The launcher binds `127.0.0.1`; its printed link is local to the workload host.

## Relay the existing Tailscale address

Use a small Node relay on the host's existing Tailscale IPv4 address. Console retains
its loopback listener, token exchange and HTTP-only session. The relay accepts the
browser's exact origin, translates it to Console's loopback origin and streams
responses, including `/api/live` and downloads. Simply changing the link's hostname
without this origin translation causes authentication to fail.

Save this as `$HOST_STATE/tailscale-proxy.mjs`:

```js
import { createServer, request } from 'node:http';
import { readFile } from 'node:fs/promises';
import { isIP } from 'node:net';

const [host, logPath] = process.argv.slice(2);
if (isIP(host) !== 4 || host === '0.0.0.0') throw new Error('Use the host Tailscale IPv4 address');
const log = await readFile(logPath, 'utf8');
const link = log.match(/^Open: (\S+)$/m)?.[1];
if (!link) throw new Error('Wait for the Console Open: line in the launch log');
const upstream = new URL(link);
if (upstream.hostname !== '127.0.0.1' || !upstream.hash.startsWith('#access='))
  throw new Error('Expected the running Console loopback access link');
const external = new URL(link);
external.hostname = host;

const relay = createServer((incoming, outgoing) => {
  if (incoming.headers.origin && incoming.headers.origin !== external.origin) {
    outgoing.writeHead(403); outgoing.end(); return;
  }
  const headers = { ...incoming.headers, host: upstream.host };
  if (headers.origin) headers.origin = upstream.origin;
  const forwarded = request({ hostname: upstream.hostname, port: upstream.port,
    path: incoming.url, method: incoming.method, headers }, response => {
    outgoing.writeHead(response.statusCode, response.headers);
    response.pipe(outgoing);
  });
  forwarded.on('error', () => {
    if (!outgoing.headersSent) outgoing.writeHead(502);
    outgoing.end();
  });
  incoming.on('aborted', () => forwarded.destroy());
  outgoing.on('close', () => forwarded.destroy());
  incoming.pipe(forwarded);
});
relay.listen(Number(upstream.port), host, () => console.log(`Open: ${external.href}`));
```

Start the relay as the ordinary host user. It binds only the selected Tailscale
address, using the same port as Console. Preserve its PID for cleanup.

```sh
setsid --fork node "$HOST_STATE/tailscale-proxy.mjs" "$(tailscale ip -4)" \
  "$HOST_STATE/server.log" < /dev/null > "$HOST_STATE/access.log" 2>&1
```

## Hand over the URL first

As soon as the relay listens and serves the page, send the **complete direct URL
from `access.log`**, including `#access=...`. Read it from the running launcher/relay
output so the token is copied exactly. The URL must name the host's Tailscale address.
The browser exchanges the token for a session and removes the fragment from its history.

Keep the link private. Continue validation after handing it over, report results
briefly, then stop validation and leave Console running. Coordinate any further
Start/Stop checks with the user's activity; preserve their active run.

## Verify the browser-facing endpoint

Use the exact direct URL in a fresh browser context. Headless Chromium is suitable;
check installed Playwright packages and `~/.cache/ms-playwright/` even when a browser
control tool reports no available browser.

Confirm the access exchange returns `204`, authenticated `/api/current` returns
`200`, and `/api/live` streams updates. Then verify Review → Start → Stop → View result,
a recording download and any requested file-change report. A successful HTML request
or a browser check against loopback alone does not establish direct Tailscale access.

## If access fails

| Symptom | Action |
| --- | --- |
| Page cannot be reached | Check both processes, their bind addresses and the relay log. Console needs loopback; the relay needs the host's existing Tailscale IP. |
| “Access link expired” | Inspect the actual `/api/session` response. `403` can mean a wrong token or origin. Compare the shared URL with the current relay output and reproduce it in a fresh browser before requesting more user diagnostics. |
| Page loads but access fails over Tailscale | Check that the relay validates the external origin and forwards the loopback origin. Preserve `Authorization`, `Cookie` and `Set-Cookie`. |
| “Connection lost; run may still be active.” | Check `/api/live`. `401` means the session is missing/expired; reopen the current full direct link. Stream/decoding failures also produce this message. |
| Port already occupied | Identify the existing process and preserve other sessions. Choose a free Console port and use that same port on the relay. |

Stop active runs before restarting Console. A restart invalidates browser sessions;
regenerate the relay's direct link from the new launch log and share it again.
Stop only this session's relay and launcher during cleanup; retain recordings.

## Optional SSH forwarding

When SSH forwarding is requested, append `--ssh user@TAILSCALE_IP` to the normal
launcher. Run its printed forwarding command on the browser computer:

```sh
ssh -N -o ExitOnForwardFailure=yes \
  -L 127.0.0.1:43187:127.0.0.1:43187 user@TAILSCALE_IP
```

Keep that terminal open and use the complete printed `127.0.0.1` access link.
`--browser-port` selects another local browser port. This access mode uses the
launcher's loopback URL and does not need the Tailscale relay.
