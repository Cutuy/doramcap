# Provenance Lab wrapper

Runs the dynamic pipeline and monitor in a generation-owned Dora coordinator.
The wrapper drains the pipeline and retains its summary; logs remain private.

```sh
cargo build --release --locked -p provenance-demo
npm --prefix apps/provenance-demo/frontend ci
npm --prefix apps/provenance-demo/frontend run build
dora --version  # must report dora-cli 1.1.0
stage=$(mktemp -d)
cp "$(command -v dora)" "$stage/dora"
cp target/release/provenance-demo "$stage/"
cp -r apps/provenance-demo/frontend/dist "$stage/dist"
cp apps/provenance-demo/configs/measurements.pbtxt "$stage/measurements.pbtxt"
cp apps/provenance-demo/deploy/{launch.ts,flow.yml} "$stage/"
docker build -t doramcap/provenance-demo:local -f apps/provenance-demo/deploy/Dockerfile "$stage"
rm -r "$stage"
```

Enable with `--enable-app provenance-demo`. The hosted pipeline runs until session
expiry; standalone finite-duration operation remains available.
