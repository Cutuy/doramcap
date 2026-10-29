FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends libelf1t64 \
    && rm -rf /var/lib/apt/lists/*
COPY system-monitor /usr/local/bin/system-monitor
ENTRYPOINT ["/usr/local/bin/system-monitor"]
