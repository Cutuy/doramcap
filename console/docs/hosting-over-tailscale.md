# Console over Tailscale

- Before hosting from source, run [`stageSystemAppInputs`](../scripts/system-app-inputs.mts), use the staged app bundle, and verify all app manifests, profiles, graphs and shared files exist, including optional apps.
- Relay Console's loopback listener through the existing Tailscale address with auth, cookies, streams and origin validation/translation preserved; share the complete `http://TAILSCALE_IP:PORT/#access=TOKEN` immediately, then verify it in fresh headless Chromium while preserving active runs, Tailscale settings and the running Console.
