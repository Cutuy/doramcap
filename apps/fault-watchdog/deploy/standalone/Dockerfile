FROM ubuntu:24.04
COPY fault-watchdog /usr/local/bin/fault-watchdog
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/fault-watchdog"]
CMD ["/etc/fault-watchdog/watchdog.pbtxt"]
