# Console over Tailscale SSH

Use the normal Console launcher and an SSH local forward. Both computers need
Tailscale access and working SSH authentication. Build/setup: [Console README](../README.md).

## 1. Start on the workload host

```sh
sudo node console/server/dist/main.js /absolute/console.pbtxt /absolute/frontend/dist \
  --port 43187 --ssh user@TAILSCALE_IP
```

Use the host's SSH username and Tailscale IP. Console listens on `127.0.0.1`.
Keep this process running; preserve its printed forwarding command and private access link.

## 2. Connect from the browser computer

Run the printed command and keep that terminal open:

```sh
ssh -N -o ExitOnForwardFailure=yes \
  -L 127.0.0.1:43187:127.0.0.1:43187 user@TAILSCALE_IP
```

Open the **complete printed link**, including `#access=...`. Use `127.0.0.1`
exactly. The browser exchanges the token for a session and removes the fragment.

## 3. Verify before handing off

On the browser computer:

```sh
curl -fsS -o /dev/null -w '%{http_code}\n' http://127.0.0.1:43187/
```

Expect `200`, then use a real browser with the access link. Verify Review → Start →
Stop → View result and a recording download; HTML/API-only checks are insufficient.

## If access fails

| Symptom | Action |
| --- | --- |
| SSH exits or local port is occupied | Read the SSH error. For another browser port, launch Console with `--browser-port 43188` and use its new command/link. |
| “Connection lost; run may still be active.” | Check `/api/live` in browser Network. `401` means the session is missing/expired: reopen the current full access link. It is also shown for stream/decoding failures. |
| `403` | Use the printed hostname and browser port exactly; `localhost` and `127.0.0.1` are distinct origins. |

Stop active runs before restarting Console. A restart invalidates browser sessions;
reopen its current access link afterward. Keep the launcher, bind address and authentication unchanged.
