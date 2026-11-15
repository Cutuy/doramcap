![Service invocation comparison](111526-invocation-comparison.svg)

# Service invocation comparison

Created 11/15/26. [Implementation](111526-invocation-comparison-impl.md).

History and Review compare each service's command, entrypoint and stop signal.
Native preparation normalizes generated Dora ports and paths; configured arguments stay exact.

One `experiment.setup.pb` holds each run’s typed setup and captured/normalized invocations: mode 0600, atomic
publication, review/run directory retention and exclusion from file changes. SQLite owns history, status and machine
captures; native preparation also runs during Review, before a run row exists.

Missing or invalid evidence keeps captured text with an **Unavailable** verdict.
Saving comparison evidence never blocks execution. Sensitive names and credential URLs
mask whole invocations.
Values compare before the 512-character display bound; safe reads allow 4 MiB.
