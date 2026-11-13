# doramcap
[![Console release](https://github.com/Cutuy/doramcap/actions/workflows/console-release.yml/badge.svg)](https://github.com/Cutuy/doramcap/actions/workflows/console-release.yml)
[![Rust CI](https://github.com/Cutuy/doramcap/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Cutuy/doramcap/actions/workflows/ci.yml)
[![Node CI](https://github.com/Cutuy/doramcap/actions/workflows/node.yml/badge.svg)](https://github.com/Cutuy/doramcap/actions/workflows/node.yml)
[![Rust 1.99.1](https://img.shields.io/badge/Rust-1.99.1-orange?logo=rust)](rust-toolchain.toml)
[![Dependabot Updates](https://github.com/Cutuy/doramcap/actions/workflows/dependabot/dependabot-updates/badge.svg)](https://github.com/Cutuy/doramcap/actions/workflows/dependabot/dependabot-updates)

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

The project currently provides reusable Rust libraries, runnable Linux demos and
the Console app.
Jetson qualification and integrated runtime capabilities follow the
platform roadmap.

## Run your project in Console

[Console](console/) runs your existing Docker Compose project with no application
changes. Follow each step with live CPU, keep the recordings, and compare any two
past runs to see what changed in the setup, the machine and the results.

[![Console demo](https://github.com/Cutuy/doramcap/releases/download/console-demo-2026-11-12/console-demo.jpg)](https://github.com/Cutuy/doramcap/releases/download/console-demo-2026-11-12/console-demo.mp4)

[Download Console](https://github.com/Cutuy/doramcap/releases/latest)

## Explore a run

The [dataflow tracing demo](apps/dataflow-tracing-demo/) follows inputs through a
four-node Dora pipeline. Select an output, inspect the inputs that produced it,
and compare processing spans with measured latency on linked timelines.

Explore [live CPU activity](apps/cpu-viewer/docs/running.md) or
[recording and storage behavior](apps/storage-demo/) for a closer look at resource
contention and capture. Each demo includes its own setup and validation guide.

The [GPU storage demo](apps/gpudirect-demo/) records GPU-produced payloads
through pinned host memory to NVMe.

## Build with doramcap

Use the [project explorer](apps/project-explorer/) to browse applications,
libraries, documentation, and dependencies. Start with the
[library guide](libs/README.md) to find an integration point.

[Architecture and roadmap](docs/roadmap.md) ·
[Repository conventions](docs/repository-layout.md) ·
[CI and validation](scripts/ci/README.md)
