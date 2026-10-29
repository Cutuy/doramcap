FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl libelf1t64 libcap2-bin \
    && rm -rf /var/lib/apt/lists/*
COPY storage-demo /usr/local/bin/storage-demo
COPY system-monitor /usr/local/bin/system-monitor
RUN setcap cap_bpf,cap_perfmon,cap_syslog=ep /usr/local/bin/system-monitor
COPY dist/ /opt/storage-observatory/dist/
ENTRYPOINT ["/usr/local/bin/storage-demo"]
