# Fault Demo wrapper

Runs the reporters, watchdog and dashboard in an isolated Dora flow. Each
generation retains `state.json`; container diagnostics remain private.

```sh
cargo build --release --locked -p fault-demo -p fault-watchdog
dora --version  # must report dora-cli 1.1.0
stage=$(mktemp -d)
cp "$(command -v dora)" "$stage/dora"
cp target/release/fault-demo-source target/release/fault-demo-dashboard target/release/fault-watchdog "$stage/"
cp apps/fault-demo/deploy/flow.yml "$stage/"
cp apps/fault-watchdog/config/default.pbtxt "$stage/watchdog.pbtxt"
docker build -t doramcap/fault-demo:local -f apps/fault-demo/deploy/Dockerfile "$stage"
rm -r "$stage"
```

Enable with `--enable-app fault-demo`. See [remaining work](../../project-explorer/server/docs/102727-remaining-apps.md#m4--fault-demo-planned).

Compose runs foreground Dora coordinator/daemon services. The registration declares
node groups; the shared Rust workflow owns launch, readiness, ordered stop and drain.
