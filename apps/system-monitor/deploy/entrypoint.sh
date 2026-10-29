#!/bin/sh
# Preserve startup capabilities and collector diagnostics with the result.
/usr/local/bin/system-monitor --check > /results/capabilities.json 2>&1 || true
exec /usr/local/bin/system-monitor "$@" > /results/monitor.log 2>&1
