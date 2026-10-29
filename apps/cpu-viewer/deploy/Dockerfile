FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl libelf1t64 libcap2-bin \
    && rm -rf /var/lib/apt/lists/*
COPY dora cpu-viewer system-monitor mcap-recorder /usr/local/bin/
RUN setcap cap_bpf,cap_perfmon,cap_syslog=ep /usr/local/bin/system-monitor
COPY dist/ /opt/cpu-viewer/dist/
COPY flow.yml /opt/dora-flow/flow.yml
WORKDIR /tmp
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/dora"]
CMD ["daemon", "--zenoh-no-multicast", "--worker-threads", "2"]
