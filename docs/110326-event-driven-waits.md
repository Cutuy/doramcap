# Event-driven waits

![Event-driven workers](110326-event-driven-waits.svg)

Workers wait for work, shutdown or a relevant state change. Notifications wake them when progress is possible, reducing idle CPU use and improving responsiveness.

Apply this across activity tracing, system monitoring, demo apps and workload orchestration:

- Wake promptly when input arrives or capacity becomes available.
- Respond to shutdown, completion and failure while waiting.
- Preserve safe resource reuse and drain accepted work on shutdown.
- Keep timers for sampling, pacing, simulated work and real deadlines.
