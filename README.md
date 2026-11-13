# doramcap
[![Console release](https://github.com/Cutuy/doramcap/actions/workflows/console-release.yml/badge.svg)](https://github.com/Cutuy/doramcap/actions/workflows/console-release.yml)
[![Rust CI](https://github.com/Cutuy/doramcap/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Cutuy/doramcap/actions/workflows/ci.yml)
[![Node CI](https://github.com/Cutuy/doramcap/actions/workflows/node.yml/badge.svg)](https://github.com/Cutuy/doramcap/actions/workflows/node.yml)
[![Rust 1.99.1](https://img.shields.io/badge/Rust-1.99.1-orange?logo=rust)](rust-toolchain.toml)
[![Dependabot Updates](https://github.com/Cutuy/doramcap/actions/workflows/dependabot/dependabot-updates/badge.svg)](https://github.com/Cutuy/doramcap/actions/workflows/dependabot/dependabot-updates)

doramcap runs a Docker Compose robotics stack on one Linux host and records what
the machine did during the run: per-cgroup CPU time from BPF, Dora messages and
faults in MCAP, and filesystem writes from OverlayFS. Target platforms are NVIDIA
Jetson Orin and Thor; see the [roadmap](docs/roadmap.md) for qualification status.

[![Console demo](https://github.com/Cutuy/doramcap/releases/download/console-demo-2026-11-12/console-demo.jpg)](https://github.com/Cutuy/doramcap/releases/download/console-demo-2026-11-12/console-demo.mp4)

## Console

[Console](console/) runs an unmodified Compose project and keeps each run.

| Stage | Contents |
|---|---|
| Review | Resolved services, image IDs, system apps, diff against the previous run |
| Run | Workflow step timeline, CPU total and top cgroups, Fault Watchdog state |
| Result | CPU, message and fault MCAP; file changes in host folders, image layers and volumes |
| History | A/B diff of setup, machine configuration, machine condition and results |

![Console overview](docs/console-overview.svg)

Ubuntu 24.04+ on x86_64 or ARM64 with Docker Engine, Compose v2, cgroup v2 and
OverlayFS ([requirements](console/scripts/release-readme.md)):

```sh
curl -fLO https://github.com/Cutuy/doramcap/releases/latest/download/install.sh
sh install.sh
cd /path/to/compose/project && doramcap-console start
```

## Components

[Rust libraries](libs/README.md) for CPU policy and accounting, activity tracing,
fault capture, io_uring MCAP writing and CUDA host memory. Demos:
[dataflow tracing](apps/dataflow-tracing-demo/),
[CPU activity](apps/cpu-viewer/docs/running.md),
[storage](apps/storage-demo/), [GPU storage](apps/gpudirect-demo/).

[Project explorer](apps/project-explorer/) ·
[Repository conventions](docs/repository-layout.md) ·
[CI and validation](scripts/ci/README.md)
