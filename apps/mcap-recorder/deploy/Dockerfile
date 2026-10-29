FROM ubuntu:24.04
COPY dora system-monitor mcap-recorder /usr/local/bin/
COPY flow.yml /opt/dora-flow/flow.yml
COPY dora-supervise.sh /usr/local/bin/dora-supervise
WORKDIR /tmp
ENV HOME=/tmp DORA_PRODUCER_PROCESS=system-monitor DORA_RECORDER_PROCESS=mcap-recorder DORA_CAPTURE_COUNT=true
ENTRYPOINT ["/bin/sh", "/usr/local/bin/dora-supervise"]
