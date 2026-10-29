# Fault Watchdog wrapper

Runs an independent watchdog, synthetic reporters and the Fault Demo dashboard.
Each generation retains `state.json`; container diagnostics remain private.

Stage the binaries and files as in the [Fault Demo wrapper](../../fault-demo/deploy/README.md),
using this directory's `flow.yml` and `Dockerfile`. Tag the image
`doramcap/fault-watchdog:local` and enable `--enable-app fault-watchdog`.

For MCAP, build with `--features mcap`, stage `watchdog-mcap.pbtxt` as
`watchdog.pbtxt`, and use `compose.mcap.yaml` and `registration.mcap.pbtxt`.
The MCAP profile uses `seccomp=unconfined` for io_uring and rotates at 16 MiB.

For OTLP, build with `--features otel`, configure a container-reachable collector
URL in a private copy of `watchdog-otel.pbtxt`, and use `compose.otel.yaml` and
`registration.otel.pbtxt`. Tag the images `:mcap` or `:otel`, respectively. Install
one trusted registration as `registration.pbtxt` before starting the manager.

Compose runs foreground Dora coordinator/daemon services. The registration declares
node groups; the shared Rust workflow owns launch, readiness, ordered stop and drain.
