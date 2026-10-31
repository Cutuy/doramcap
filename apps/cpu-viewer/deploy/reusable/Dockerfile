FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY cpu-viewer /usr/local/bin/cpu-viewer
ENTRYPOINT ["/usr/local/bin/cpu-viewer"]
CMD ["--headless", "--bind", "127.0.0.1", "--port", "8080"]
