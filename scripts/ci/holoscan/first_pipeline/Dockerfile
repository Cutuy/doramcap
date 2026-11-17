# syntax=docker/dockerfile:1
ARG BASE_IMAGE=nvcr.io/nvidia/cuda@sha256:cf5ab24c3f5040a0ea5931658874c2158e606bf740f006809aa4bb9d3334016b
FROM ${BASE_IMAGE}
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 python3-venv cmake ninja-build libstdc++6 \
    && rm -rf /var/lib/apt/lists/*
RUN python3 -m venv /opt/venv
ENV PATH=/opt/venv/bin:${PATH}
COPY requirements.txt /tmp/requirements.txt
RUN python -m pip install --no-cache-dir -r /tmp/requirements.txt
