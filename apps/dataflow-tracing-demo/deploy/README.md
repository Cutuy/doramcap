# Dataflow Tracing Demo wrapper

Runs the dynamic pipeline and monitor in a generation-owned Dora coordinator.
The wrapper drains the pipeline and retains its summary; logs remain private.

```sh
cargo build --release --locked -p dataflow-tracing-demo
npm --prefix apps/dataflow-tracing-demo/frontend ci
npm --prefix apps/dataflow-tracing-demo/frontend run build
dora --version  # must report dora-cli 1.1.0
stage=$(mktemp -d)
cp "$(command -v dora)" "$stage/dora"
cp target/release/dataflow-tracing-demo "$stage/"
cp -r apps/dataflow-tracing-demo/frontend/dist "$stage/dist"
cp apps/dataflow-tracing-demo/configs/measurements.pbtxt "$stage/measurements.pbtxt"
cp apps/dataflow-tracing-demo/deploy/{launch.ts,flow.yml} "$stage/"
docker build -t doramcap/dataflow-tracing-demo:local -f apps/dataflow-tracing-demo/deploy/Dockerfile "$stage"
rm -r "$stage"
```

Enable with `--enable-app dataflow-tracing-demo`. The hosted pipeline runs until session
expiry; standalone finite-duration operation remains available.
