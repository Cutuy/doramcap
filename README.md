# doramcap

[![Rust CI](https://github.com/Cutuy/doramcap/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Cutuy/doramcap/actions/workflows/ci.yml)

We help robotics teams diagnose and fix performance failures on onboard computers,
starting with NVIDIA Jetson.

When perception, control, and recording share a computer, a missed deadline can
come from CPU contention, a delayed message, or a storage backlog. doramcap is an
open-source runtime in development that connects execution policy, traces, and
recordings so engineers can understand what happened and test a fix.

The goal: **configured policy, measured reality, actionable evidence.**

- **See where time went.** Inspect CPU activity, message lineage, and processing
  latency through live timelines.
- **Keep evidence from a run.** Capture telemetry and activity traces in MCAP for
  later investigation.
- **Compare intent with execution.** Build on typed resource policies, workload
  supervision, and fault reporting to investigate behavior under competing load.

The project currently provides reusable Rust libraries and runnable Linux demos.
Jetson qualification and integrated runtime capabilities follow the
[platform roadmap](docs/roadmap.md).

## Explore a run

The [dataflow tracing demo](apps/dataflow-tracing/) follows inputs through a
four-node Dora pipeline. Select an output, inspect the inputs that produced it,
and compare processing spans with measured latency on linked timelines.

Explore [live CPU activity](apps/cpu-viewer/docs/running.md) or
[recording and storage behavior](apps/storage-demo/) for a closer look at resource
contention and capture. Each demo includes its own setup and validation guide.

## Build with doramcap

Use the [project explorer](apps/project-explorer/) to browse applications,
libraries, documentation, and dependencies. Start with the
[library guide](libs/README.md) to find an integration point.

[Architecture and roadmap](docs/roadmap.md) ·
[Repository conventions](docs/repository-layout.md) ·
[CI and validation](scripts/ci/README.md)
