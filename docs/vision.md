# Vision

doramcap is a personal research workspace for running and diagnosing workloads
on a single compute system, especially resource-constrained robotics platforms.
The goal is to connect configured execution policy with measured behavior and
recorded evidence.

## Available today

- Reusable Rust libraries for CPU policy/accounting, tracing, fault handling,
  MCAP recording, storage and CUDA-accessible memory.
- Compose experiment workflows and a Supervisor-backed System Monitor runner,
  with readiness, recording checks and cleanup evidence.
- Independently runnable apps and browser demos for inspecting those components.
  Dora provides messaging for integrated apps.

See the [roadmap](roadmap.md) for capability status and remaining qualification.

## Direction

- Let existing applications adopt lifecycle management and observability incrementally.
- Keep essential execution, recording and inspection local.
- Bound instrumentation and storage costs; expose overload and incomplete evidence.
- Share typed Protobuf contracts and compose application configuration in pbtxt.
- Qualify CPU, GPU, memory and storage behavior under competing workloads.

NVIDIA Jetson Orin/Thor with Ubuntu 24.04 are target platforms. A published
Jetson hardware/software qualification profile remains future work.
