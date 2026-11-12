# Console over Tailscale

- Keep Console on loopback and relay the host's existing Tailscale address, preserving auth, cookies and streams while translating the validated browser origin to Console's loopback origin; keep Tailscale settings unchanged.
- Immediately share `http://TAILSCALE_IP:PORT/#access=TOKEN` using the running launcher's token, then verify that exact URL in fresh headless Chromium; preserve the user's active run and leave Console running.
