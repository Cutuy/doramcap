# Fault Watchdog deployment

Fault Watchdog runs reporters and a dashboard. MCAP and OTLP variants provide
additional outputs through their matching Compose and registration files.
Build the same four native binaries as Fault Demo. The gallery starts shared Dora
before these app services; the MCAP writer keeps `seccomp=unconfined`.

[Shared lifecycle and test results](../../../libs/workload-docker/docs/102926-shared-dora-experiment.md).
