FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates zlib1g \
    && rm -rf /var/lib/apt/lists/*
COPY dora /usr/local/bin/dora
RUN test "$(/usr/local/bin/dora --version)" = "dora-cli 1.1.0"
