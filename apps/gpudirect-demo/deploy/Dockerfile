FROM node:22-bookworm-slim AS node_runtime
FROM nvidia/cuda:12.9.1-runtime-ubuntu24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates cuda-nvrtc-12-9 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=node_runtime /usr/local/bin/node /usr/local/bin/node
COPY gpudirect-demo /usr/local/bin/gpudirect-demo
COPY serve.mts /opt/gpudirect-demo/deploy/serve.mts
COPY architecture.svg /opt/gpudirect-demo/architecture.svg
ENV HOME=/tmp
ENTRYPOINT ["node", "/opt/gpudirect-demo/deploy/serve.mts", "/usr/local/bin/gpudirect-demo", "/results"]
