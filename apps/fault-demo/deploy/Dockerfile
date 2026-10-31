FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY fault-demo-source fault-demo-controls fault-demo-dashboard fault-watchdog /usr/local/bin/
WORKDIR /tmp
ENV HOME=/tmp
