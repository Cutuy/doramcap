# Console backend

TypeScript owns configuration, app selection, reviews, request coordination,
progress views and persistence. A native bridge runs Compose and systemd workflows
on its owner thread and exposes bounded events and safe OS handles.

```sh
npm --prefix console/server ci
npm --prefix console/server run build:native
npm --prefix console/server run build
sudo node console/server/dist/main.js /absolute/console.pbtxt /absolute/frontend/dist
```

Build unprivileged; run with root privileges on the workload host. Compose uses
mount-namespace inspection and OverlayFS isolation (`CAP_SYS_ADMIN`); systemd uses
the system service manager. Docker connects on the first Compose operation; systemd
startup and inspection work without a Docker daemon. See [Console setup](../README.md).

The [execution contract](../proto/execution.proto) carries a shared run plan, one
backend plan, configuration snapshots and execution facts. The workflow owns repository admission,
reset, preflight and cleanup. Console builds the browser-facing status in TypeScript.

[HTTP contract](../contract/transport.ts) · [Configuration](../docs/configuration.md) · [Recovery](../docs/recovery.md) ·
[Execution boundaries](../docs/103026-console-compose-steps.svg) ·
[Command flow](../docs/102926-console-interop.svg)

Run `npm --prefix console/server test` for server, native transport and OS-helper
checks. The opt-in `console/server/tests/real-compose.ts` exercises two real
start/stop cycles; build unprivileged and use the repository capability-test procedure.
`console/server/tests/real-systemd.ts` runs CPU monitoring, Fault Watchdog and a user unit
under systemd; `DORAMCAP_TEST_BINARIES` names a directory of release-feature host binaries.

The [native client](src/native.ts) accepts and returns generated protobuf messages.
It keeps N-API byte serialization private; application code uses the
[typed interface](native/index.d.ts).

Check an existing playground through HTTP with the saved launch instructions or
startup log containing its private `Open:` link:

```sh
npm --prefix console/server run check:playground -- --launch-file /path/to/console-launch.txt --output /path/to/check.json
```

The check selects CPU monitoring with recording and the live bridge, starts a run,
receives live frames, stops it, checks cleanup and downloads a sealed CPU MCAP.
Use repeated `--expect-step ID` options to require specific successful workflow
steps, such as `systemd.runtime.prepare` and `systemd.runtime.remove` with PR #247.
It uses generated protobuf HTTP bindings and needs no native addon or root access.

For a remote browser over Tailscale, keep Console on loopback and run the printed
SSH forwarding command on the browser computer. Open the printed `127.0.0.1` link
through that tunnel; its host and port preserve Console's Origin checks.
