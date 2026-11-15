![Console learns starts and exits from Docker events](111526-exits-from-docker-events.svg)

# Compose app exits from Docker events

Created: 2026-11-15. [Implementation](111526-exits-from-docker-events-impl.md).

Our Compose apps used a Docker healthcheck, `test -s <app>.ready`, every 200 ms for the
whole run. Each probe is a `docker exec`, which starts processes through containerd and
runc inside the app's cgroup. On a 2-vCPU ARM64 host, four idle containers with that
probe used 74% of 200% CPU; Top CPU consumers showed the probes as the apps' own load.

Console needs to know when a Compose app crashes or exits, not when it is ready. Docker
events already report each container start and each exit with its code.

## Behavior

- Our Compose apps ship no healthcheck and no ready file. A running container counts as started.
- Crashes and exits come from Docker `die` events and their exit codes, as today.
- Healthchecks a customer writes in their own Compose files keep their native meaning.
- systemd and supervisord apps are unchanged; they never used probes.
- Start order between our demo services uses `service_started`.

## Tradeoffs

- Console no longer waits for an app's own initialization before starting dependents.
  Dora apps attach through the daemon, so start order between them does not affect delivery.
- Apps report their own health later through the fault watchdog, which Console will show live.
