# Console

Select a Compose project and optional system apps, inspect the resolved services,
then run the shared experiment and download its recordings. Dora coordinator and
IPC daemon are included automatically.

![Console and workflow boundaries](docs/103026-console-compose-steps.svg)

```sh
npm --prefix console/server ci
npm --prefix console/server run build:native
npm --prefix console/server run build
npm --prefix console/frontend ci
npm --prefix console/frontend run build
sudo node console/server/dist/main.js /absolute/console.pbtxt /absolute/console/frontend/dist
```

Build unprivileged; run Console with sudo on the local Docker host. The workflow
checks Docker's mount namespace and uses `CAP_SYS_ADMIN` for OverlayFS isolation;
Docker socket access alone is insufficient.

Use Node.js 22 or newer. Build the native addon on the target platform. Configure
host paths and app inputs in [console.pbtxt.example](console.pbtxt.example) and
[configuration](docs/configuration.md). The browser uses HTTP Basic authentication
with username `console` and the configured secret file's contents.

[HTTP contract](contract/transport.ts) · [Backend](server/README.md) · [Recovery](docs/recovery.md) ·
[System app specification](docs/103026-system-apps-plan.md)
