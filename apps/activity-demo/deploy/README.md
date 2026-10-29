# Activity Lab image

Build the binary and frontend for the target architecture and Ubuntu 24.04:

```sh
cargo build --release --locked -p activity-demo
npm ci --prefix apps/activity-demo/frontend
npm run build --prefix apps/activity-demo/frontend
stage=$(mktemp -d)
cp target/release/activity-demo "$stage/activity-demo"
cp -r apps/activity-demo/frontend/dist "$stage/dist"
docker build -t doramcap/activity-lab:local -f apps/activity-demo/deploy/Dockerfile "$stage"
rm -r "$stage"
```

The gallery supplies a fresh results directory and UID/GID. `activity.mcap` is
retained after shutdown. See [hosting](../../project-explorer/server/docs/hosting.md).
