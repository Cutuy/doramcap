# Gallery wrapper

Runs all three modes (32 × 1-MiB messages each), displays the diagram and measurements,
and retains verified MCAP files. Requires an NVIDIA Docker runtime.

```sh
cargo build --release --locked -p gpudirect-demo
stage=$(mktemp -d)
cp target/release/gpudirect-demo apps/gpudirect-demo/deploy/serve.mts apps/gpudirect-demo/architecture.svg "$stage/"
docker build -t doramcap/gpudirect-demo:local -f apps/gpudirect-demo/deploy/Dockerfile "$stage"
rm -r "$stage"
```

Enable the gallery server with `--enable-app gpudirect-demo`; it uses the
gallery’s standard shared runtime. The existing gallery
supplies `GPUDIRECT_RESULTS_DIR`, `GPUDIRECT_UID` and `GPUDIRECT_GID` for each run.
The container requests one GPU, a 256-MiB memlock budget and io_uring access.
Readiness follows full readback; shutdown waits for active CUDA work and marks
completed outputs for the gallery's result publisher.

Native wrapper checks: `node apps/gpudirect-demo/tests/hosting.mts /absolute/path/gpudirect-demo`.
