# Fault Watchdog deployment

Fault Watchdog runs reporters and a dashboard. `watchdog.pbtxt` selects its outputs;
copy [`watchdog-mcap.pbtxt`](watchdog-mcap.pbtxt) or [`watchdog-otel.pbtxt`](watchdog-otel.pbtxt)
over it to add MCAP or OTLP. Build the same four native binaries as Fault Demo.
The gallery starts shared Dora before these app services. The watchdog keeps
`seccomp=unconfined` for the MCAP writer's io_uring calls.

[Shared lifecycle and test results](../../../libs/workload-docker/docs/102926-shared-dora-experiment.md).

The dashboard binds host loopback port 8089 in every configuration; Fault Demo uses 8088.
