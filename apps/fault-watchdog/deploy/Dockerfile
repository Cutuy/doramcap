FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY dora fault-demo-source fault-demo-dashboard fault-watchdog /usr/local/bin/
COPY flow.yml /opt/dora-flow/flow.yml
COPY watchdog.pbtxt /opt/fault-demo/watchdog.pbtxt
COPY dora-supervise.sh /usr/local/bin/dora-supervise
WORKDIR /tmp
ENV HOME=/tmp DORA_WORKLOAD_PROCESS=fault-demo-source DORA_AFTER_WORKLOAD_PROCESS=fault-demo-dashboard DORA_RECORDER_PROCESS=fault-watchdog DORA_SNAPSHOT_URL=http://127.0.0.1:8088/api/state DORA_SNAPSHOT_FILE=state.json
ENTRYPOINT ["/bin/sh", "/usr/local/bin/dora-supervise"]
