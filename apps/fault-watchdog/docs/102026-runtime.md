![Architecture](production-watchdog.svg)

# Runtime contract

- Each configured Dora input owns one producer incarnation. Start with a reporter snapshot; send ready batches immediately and repeat snapshots periodically.
- A new incarnation retires the old source. Closed/timed-out sources remain visible and stale. Fresh evidence restores freshness; silence cannot recover health.
- The host drains each event, advances deadlines, and publishes ready updates without a fill timer. Maintenance defaults to 10 ms; condition deadlines can wake it earlier.
- Each sink owns a bounded queue and blocking executor. Producer reporting has no worker thread. Output overflow increments diagnostics and gives repair snapshots admission priority.
- Epoch/revision identify events across retries. Delivery can duplicate or lose history. Periodic snapshots restore current state. Filtered self-diagnostics also create revision gaps.
- Local ingress/sink conditions enter aggregation. Each sink excludes its own condition; other outputs retain it. Error detail includes cumulative sink errors/drops. Stderr is the final fallback.
- MCAP uses the shared writer, a create-new path, numbered rotation files and explicit idle flush. An I/O failure disables recording until restart; partial writes are unsafe to retry.
- OTel exports domain updates and state snapshots as structured logs. The full endpoint includes `/v1/logs`. Identity/counter attributes use strings to retain integer precision.
- Shutdown closes all sink queues before waiting to one configured deadline. Timeout reports unfinished output; an in-flight OS call can outlive that wait. No durable-delivery guarantee is implied.

## Configuration

`WatchdogConfig` lives in this app; shared domain messages remain in `fault-schema`.
Scalar presence supports overrides, including zero retries. Nested sink messages
replace the entire inherited sink configuration. Defaults enable live output only.

```text
mcap { enabled: true path: "/tmp/faults.mcap" flush_ms: 100 rotation_bytes: 67108864 }
otel { enabled: true endpoint: "http://127.0.0.1:4318/v1/logs" timeout_ms: 1000 service_name: "watchdog" }
```

Add these fields inside the typed payload in `config/default.pbtxt`; compile the
corresponding features. Startup rejects unavailable features and frame/queue budgets
that cannot contain the configured maximum state. Source timeout and producer
condition deadline are independent.

## Validation

See [implementation evidence](102026-production-watchdog.md). Demo timing includes
HTTP polling and Dora delivery; it is a smoke measurement, not a real-time guarantee.
