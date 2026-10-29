#!/bin/sh
# One container owns one Dora flow. Stop workloads before monitoring and recording.
set -u

# Keep launcher and node diagnostics after the disposable container is removed.
exec >> /results/dora.log 2>&1
cp /opt/dora-flow/flow.yml /tmp/flow.yml || exit 1
stopping=0
trap 'stopping=1' INT TERM
/usr/local/bin/dora run /tmp/flow.yml &
flow_pid=$!
flow_status=0
wait "$flow_pid" || flow_status=$?
if [ "$stopping" -eq 0 ]; then
    exit "$flow_status"
fi

wait_for_exit() {
    process_name=$1
    deadline=$(($(date +%s) + $2))
    while pidof "$process_name" >/dev/null; do
        if [ "$(date +%s)" -ge "$deadline" ]; then
            echo "timed out waiting for $process_name" >&2
            return 1
        fi
        sleep 0.1
    done
}

stop_process() {
    process_name=$1
    process_ids=$(pidof "$process_name") || {
        echo "$process_name is absent during shutdown" >&2
        return 1
    }
    echo "stopping $process_name"
    # The container has its own PID namespace; these are trusted flow binaries.
    kill -INT $process_ids || return 1
    wait_for_exit "$process_name" "$2"
}

failed=0
if ! pidof "$DORA_RECORDER_PROCESS" >/dev/null; then
    echo "$DORA_RECORDER_PROCESS is absent during shutdown" >&2
    failed=1
fi
if [ -n "${DORA_SNAPSHOT_URL:-}" ]; then
    if ! curl --fail --silent --max-time 3 "$DORA_SNAPSHOT_URL" > "/results/$DORA_SNAPSHOT_FILE"; then
        echo "could not retain the application snapshot" >&2
        failed=1
    fi
fi
if [ -n "${DORA_WORKLOAD_PROCESS:-}" ]; then
    stop_process "$DORA_WORKLOAD_PROCESS" 6 || failed=1
fi
if [ -n "${DORA_AFTER_WORKLOAD_PROCESS:-}" ] && pidof "$DORA_AFTER_WORKLOAD_PROCESS" >/dev/null; then
    stop_process "$DORA_AFTER_WORKLOAD_PROCESS" 6 || failed=1
fi
if [ -n "${DORA_PRODUCER_PROCESS:-}" ]; then
    stop_process "$DORA_PRODUCER_PROCESS" 12 || failed=1
fi
if [ "$failed" -eq 0 ]; then
    echo "waiting for $DORA_RECORDER_PROCESS to drain"
    wait_for_exit "$DORA_RECORDER_PROCESS" 18 || failed=1
fi
if [ "$failed" -ne 0 ]; then
    echo "ordered shutdown failed; terminating the Dora flow" >&2
    kill -TERM "$flow_pid" 2>/dev/null || true
    wait "$flow_pid" 2>/dev/null || true
    exit 1
fi

flow_status=0
wait "$flow_pid" || flow_status=$?
if [ "$flow_status" -ne 0 ]; then
    echo "Dora exited with status $flow_status" >&2
    exit "$flow_status"
fi
if [ "${DORA_CAPTURE_COUNT:-false}" = true ]; then
    counts=$(sed -n 's/.*session complete: collected=\([0-9][0-9]*\) published=\([0-9][0-9]*\) dropped=\([0-9][0-9]*\).*/\1 \2 \3/p' /results/dora.log | tail -n 1)
    # The extracted fields contain only decimal digits and spaces.
    set -- $counts
    if [ "$#" -ne 3 ]; then
        echo "collector did not report complete counts" >&2
        exit 1
    fi
    if [ "$3" != 0 ] || [ "$1" != "$2" ]; then
        echo "collector lost samples: collected=$1 published=$2 dropped=$3" >&2
        exit 1
    fi
    printf '%s\n' "$2" > /results/published.count || exit 1
fi
printf 'complete\n' > /results/shutdown.complete || exit 1
echo "ordered shutdown complete"
