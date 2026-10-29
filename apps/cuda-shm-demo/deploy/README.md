# CUDA shared-memory wrapper

Runs the CUDA demo and viewer with private IPC and logs. A compatible NVIDIA
Docker runtime is required; the wrapper checks GPU/IPC readiness.

```sh
cargo build --release --locked -p cuda-shm-demo
npm ci --prefix apps/cuda-shm-demo/frontend
npm run build --prefix apps/cuda-shm-demo/frontend
npm run build:server --prefix apps/cuda-shm-demo/frontend
dora --version  # dora-cli 1.1.0
stage=$(mktemp -d)
cp "$(command -v dora)" "$stage/dora"
cp target/release/cuda-shm-demo "$stage/"
cp apps/cuda-shm-demo/build/{run.mjs,viewer-cli.mjs} "$stage/"
cp -r apps/cuda-shm-demo/frontend/dist "$stage/dist"
docker build -t doramcap/cuda-shm-demo:local -f apps/cuda-shm-demo/deploy/Dockerfile "$stage"
rm -r "$stage"
```

Enable with `--enable-app cuda-shm-demo`. See [remaining GPU qualification](../../project-explorer/server/docs/102727-remaining-apps.md#m9--cuda-shared-memory-demo-planned).
