# Console

Run one local Compose project with CPU recording and inspect its current result.
The server requires a local Docker socket, prebuilt workload and CPU-monitor
images, and the capabilities required by the existing recorded Compose workflow.
The configured Docker socket must be directly accessible to the server user.
The monitor image must run the system-monitor binary as its entrypoint.

Copy `console.pbtxt.example`, set the absolute paths, public browser Origin and
CPU-monitor image, and create the secret file with mode `0600`. Start the package
with Node 22.12 or later:

```sh
node server/dist/main.js /path/to/console.pbtxt ./frontend
```

Sign in as `console` with the secret file's contents. For remote access, use an SSH tunnel or TLS-terminating proxy; Basic authentication alone does not encrypt credentials. The selected project must
be beneath the configured project root. Source files are trusted host inputs.
If cleanup is unverified, follow the [Docker experiment isolation recovery procedure](https://github.com/Cutuy/doramcap/blob/main/libs/workload-docker/docs/102626-docker-experiment-isolation.md) before starting another run.
