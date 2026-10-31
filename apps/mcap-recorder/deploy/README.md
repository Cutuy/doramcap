# MCAP Recorder deployment

The headless recorder captures System Monitor telemetry to MCAP.
The Dockerfile takes `mcap-recorder` and `system-monitor`, built with monitor
features `bpf,dora`. The gallery uses `registration.pbtxt` and starts shared Dora first.

[Shared lifecycle and test results](../../../libs/workload-docker/docs/102926-shared-dora-experiment.md).
