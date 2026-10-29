# Orchestrator Lab wrapper

This pending profile packages the web server and Docker worker separately. Only
the worker receives the Docker socket and private recovery directory.

Stage the independent `orchestrator-demo` binary and `dist/` for
`Dockerfile.server`; stage `orchestrator-worker` and `worker-host.ts` for
`Dockerfile.worker`. Build the monitor image as
`doramcap/system-monitor:local`.

The gallery does not load the pending registration. See [remaining work](../../project-explorer/server/docs/102727-remaining-apps.md#m7--orchestrator-lab-planned).
