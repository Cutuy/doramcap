FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY activity-demo /usr/local/bin/activity-demo
COPY dist/ /opt/activity-lab/dist/
ENTRYPOINT ["/usr/local/bin/activity-demo"]
