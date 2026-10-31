FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY fault-watchdog /usr/local/bin/fault-watchdog
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/fault-watchdog"]
CMD ["/etc/fault-watchdog/watchdog.pbtxt"]
