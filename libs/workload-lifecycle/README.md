# Workload lifecycle

Shared recording checks and cooperative recorded-run sequencing for Compose and Supervisor.

![Lifecycle](docs/102926-supervisor-monitor-mvp.svg)

See [recording steps](src/recording.rs), the [shared run sequence](src/run.rs) used by Compose,
[phase hooks](src/recorded_run.rs) used by Supervisor,
and the [System Monitor deployment](../../apps/system-monitor/deploy/README.md).
Plan: [launch forms for Compose, systemd and Supervisor](docs/110726-launch-forms.md).
[Launch contracts](proto/launch.proto) separate app configuration from the shared runtime
filled by preparation; [shared run fields](docs/110726-shared-run-fields.md) records adoption.
