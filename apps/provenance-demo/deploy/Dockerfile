FROM node:22-bookworm-slim AS node_runtime
FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates socat \
    && rm -rf /var/lib/apt/lists/*
COPY --from=node_runtime /usr/local/bin/node /usr/local/bin/node
COPY dora provenance-demo /usr/local/bin/
COPY dist/ /opt/provenance/dist/
COPY flow.yml measurements.pbtxt launch.ts /opt/provenance/
WORKDIR /tmp
ENV HOME=/tmp
ENTRYPOINT ["node", "--experimental-strip-types", "/opt/provenance/launch.ts"]
