# Fault Watchdog deployment

Fault Watchdog runs reporters and a dashboard from [one prebuilt config](../config/default.pbtxt).
Build the same four native binaries as Fault Demo. The gallery starts shared Dora
before these app services. Console offers MCAP and OTLP through the
[standalone profiles](standalone/README.md).

[Shared lifecycle and test results](../../../libs/workload-docker/docs/102926-shared-dora-experiment.md).

The dashboard binds host loopback port 8089 in every launch; Fault Demo uses 8088.
