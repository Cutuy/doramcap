# Dataflow Tracing wrapper

Runs the dynamic pipeline and monitor in a generation-owned Dora coordinator.
The wrapper drains the pipeline and retains its summary; logs remain private.

```sh
cargo build --release --locked -p dataflow-tracing
npm --prefix apps/dataflow-tracing/frontend ci
npm --prefix apps/dataflow-tracing/frontend run build
dora --version  # must report dora-cli 1.1.0
stage=$(mktemp -d)
cp "$(command -v dora)" "$stage/dora"
cp target/release/dataflow-tracing "$stage/"
cp -r apps/dataflow-tracing/frontend/dist "$stage/dist"
cp apps/dataflow-tracing/configs/measurements.pbtxt "$stage/measurements.pbtxt"
cp apps/dataflow-tracing/deploy/{launch.ts,flow.yml} "$stage/"
docker build -t doramcap/dataflow-tracing:local -f apps/dataflow-tracing/deploy/Dockerfile "$stage"
rm -r "$stage"
```

Enable with `--enable-app dataflow-tracing`. The hosted pipeline runs until session
expiry; standalone finite-duration operation remains available.
