# Storage Observatory wrapper

Runs a synthetic CPU monitor and buffered storage; retains activity and capture
MCAP files. Enable with `--enable-app storage-demo`.

The [Moby-derived seccomp profile](https://github.com/moby/profiles/blob/4b8214fc4971e06172938a1f1554ccff6647ee80/seccomp/default.json)
permits three io_uring syscalls. Review it for the host's Docker version and use
it only on a trusted host. The upstream [license](../../../libs/workload-docker/deploy/MOBY-LICENSE) applies.

```sh
cargo build --release --locked -p storage-demo --bin storage-demo
cargo build --release --locked -p system-monitor --bin system-monitor
npm ci --prefix apps/storage-demo/frontend
npm run build --prefix apps/storage-demo/frontend
stage=$(mktemp -d)
cp target/release/storage-demo target/release/system-monitor "$stage/"
cp -r apps/storage-demo/frontend/dist "$stage/dist"
docker build -t doramcap/storage-observatory:local -f apps/storage-demo/deploy/Dockerfile "$stage"
rm -r "$stage"
```
