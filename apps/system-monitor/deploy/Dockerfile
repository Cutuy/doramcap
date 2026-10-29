FROM ubuntu:24.04
COPY system-monitor /usr/local/bin/system-monitor
COPY entrypoint.sh /usr/local/bin/monitor-entrypoint
ENTRYPOINT ["/bin/sh", "/usr/local/bin/monitor-entrypoint"]
