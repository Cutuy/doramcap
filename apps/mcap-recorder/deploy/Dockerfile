FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends libelf1t64 libcap2-bin \
    && rm -rf /var/lib/apt/lists/*
COPY system-monitor mcap-recorder /usr/local/bin/
RUN setcap cap_bpf,cap_perfmon,cap_syslog=ep /usr/local/bin/system-monitor
WORKDIR /tmp
ENV HOME=/tmp
