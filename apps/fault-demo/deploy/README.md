# Fault Demo deployment

Fault Demo runs reporters, a watchdog and a dashboard, and retains `state.json`.
The image contains `fault-demo-source`, `fault-demo-controls`, `fault-demo-dashboard`
and `fault-watchdog`. Controls and dashboard are separate Dora participants.
The gallery uses `registration.pbtxt` and starts shared Dora before these services.

[Shared lifecycle and test results](../../../libs/workload-docker/docs/102926-shared-dora-experiment.md).
