# Supervisor recording

One foreground Supervisor daemon owns one BPF monitor. The
[shared lifecycle](../../../../libs/workload-lifecycle/docs/102926-supervisor-monitor-mvp.md)
waits for `cpu.pb` advancement, stops writers, checks `cpu.mcap`, then verifies cleanup.

Prerequisites: Linux >=6.8, cgroup v2, readable kernel BTF, Supervisor 4.2.5,
effective BPF/PERFMON/SYSLOG capabilities (privileged host execution also works),
sufficient locked memory
(256 MiB recommended), and a writable delegated cgroup parent. Provision these
before running; the application performs no sudo or host setup.

Build unprivileged, edit absolute paths and fresh identities in
[run.pbtxt](run.pbtxt), then run with the required capabilities:

```sh
cargo build --profile ci --locked -p system-monitor --features bpf
cargo build --profile ci --locked -p workload-supervisor --example supervisor_monitor
target/ci/examples/supervisor_monitor apps/system-monitor/deploy/supervisor/run.pbtxt 5
```

The duration starts after recording readiness; Ctrl-C requests cooperative stop.
Paths may contain spaces. Percent signs, quotes, backslashes, INI comment characters,
control characters and Unix socket paths >=108 bytes are rejected.
The generated private daemon config includes [recording.conf](recording.conf).
The monitor uses BPF, 32,768 records and buffered embedded MCAP recording.

Outputs include stream/MCAP files, monitor/daemon logs, protobuf run settings,
`ownership.pb` (boot/cgroup/process identities and binary/config hashes), and
`report.json`. Execution, writer quiescence, recording validity and cleanup have
separate fields. A forced stop never certifies recording success.

For Compose parity with a locally built monitor image:

```sh
cargo build --profile ci --locked -p workload-docker --example compose_monitor
target/ci/examples/compose_monitor monitor-compose /tmp/monitor-compose IMAGE 5
```

Both examples emit shared recording step events. Compose also retains Docker events,
filesystem evidence and its existing Dora/OverlayFS workflow hooks.

An abandoned run blocks reuse. Preserve its directory and inspect `ownership.pb`
with `protoc --decode=doramcap.supervisor.v1.Ownership` and
`libs/workload-supervisor/proto/supervisor.proto`. Verify boot ID, cgroup device/inode,
daemon PID/start time and subtree membership before manual intervention. Use only
that verified subtree for termination, await emptiness and reap the owner before
removing its cgroup. PID existence alone grants no authority. Automatic recovery
and importing external Supervisor configurations are deferred.
