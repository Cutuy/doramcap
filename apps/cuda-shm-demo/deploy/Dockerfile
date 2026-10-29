FROM node:22-bookworm-slim AS node_runtime
FROM nvidia/cuda:12.9.1-runtime-ubuntu24.04
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates cuda-nvrtc-12-9 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=node_runtime /usr/local/bin/node /usr/local/bin/node
COPY dora /usr/local/bin/dora
COPY cuda-shm-demo /usr/local/bin/cuda-shm-demo
COPY run.mjs viewer-cli.mjs /opt/cuda-shm-demo/
COPY dist/ /opt/cuda-shm-demo/dist/
ENV HOME=/tmp CUDA_SHM_BINARY=/usr/local/bin/cuda-shm-demo
WORKDIR /tmp
ENTRYPOINT ["node", "/opt/cuda-shm-demo/viewer-cli.mjs", "--hosted", "--dora", "/usr/local/bin/dora", "--assets", "/opt/cuda-shm-demo/dist"]
