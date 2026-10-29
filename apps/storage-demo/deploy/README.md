# Storage Observatory wrapper

Runs a native BPF CPU monitor and buffered storage; retains activity and capture
MCAP files. Enable with `--enable-app storage-demo`.

The container uses `seccomp=unconfined` for io_uring.

```sh
cargo build --release --locked -p storage-demo --bin storage-demo
cargo build --release --locked -p system-monitor --bin system-monitor --features bpf
npm ci --prefix apps/storage-demo/frontend
npm run build --prefix apps/storage-demo/frontend
stage=$(mktemp -d)
cp target/release/storage-demo target/release/system-monitor "$stage/"
cp -r apps/storage-demo/frontend/dist "$stage/dist"
docker build -t doramcap/storage-observatory:local -f apps/storage-demo/deploy/Dockerfile "$stage"
rm -r "$stage"
```
