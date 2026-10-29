FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY dora fault-demo-source fault-demo-dashboard fault-watchdog /usr/local/bin/
COPY flow.yml /opt/dora-flow/flow.yml
COPY watchdog.pbtxt /opt/fault-demo/watchdog.pbtxt
WORKDIR /tmp
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/dora"]
CMD ["daemon", "--zenoh-no-multicast", "--worker-threads", "2"]
