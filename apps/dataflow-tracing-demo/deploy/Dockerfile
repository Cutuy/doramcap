FROM node:22-bookworm-slim AS node_runtime
FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates socat \
    && rm -rf /var/lib/apt/lists/*
COPY --from=node_runtime /usr/local/bin/node /usr/local/bin/node
COPY dora dataflow-tracing-demo /usr/local/bin/
COPY dist/ /opt/dataflow-tracing-demo/dist/
COPY flow.yml measurements.pbtxt launch.ts /opt/dataflow-tracing-demo/
WORKDIR /tmp
ENV HOME=/tmp
ENTRYPOINT ["node", "--experimental-strip-types", "/opt/dataflow-tracing-demo/launch.ts"]
