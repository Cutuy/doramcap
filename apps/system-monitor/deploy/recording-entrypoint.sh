#!/bin/sh
# Record a completion marker only after the collector finalizes successfully.
/usr/local/bin/system-monitor --check > /results/capabilities.json 2>&1 || true
/usr/local/bin/system-monitor "$@" > /results/monitor.log 2>&1 &
collector=$!
interrupted=0
trap 'interrupted=1; kill -INT "$collector" 2>/dev/null || true' INT TERM
while :; do
    interrupted=0
    wait "$collector"
    status=$?
    if [ "$interrupted" -eq 0 ]; then
        break
    fi
done
if [ "$status" -eq 0 ]; then
    printf 'complete\n' > /results/shutdown.complete
fi
exit "$status"
