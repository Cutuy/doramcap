Single-node high-perf runtime
Lifecycle: persisted (systemd), transient
Failure: built-in heartbeat and watchdogs
CPU placement/scheduling: affinity, rtprio, and observability out of configured prio vs reality (the CPU workload distribution plot), scheduling delay eBPFs
CPU/GPU scheduling/isolation: cores/SM, green context
Observability: OT, stdout/journal
Recording: Efficient DMA-based MCAP (CPU/GPU->NVMe/PCIe SSD)
Transport: PCIe, networking, CXL?
Latency mon: chain of msgs or seq ids, execution traces
Profiling/benchmarking:
Agentic debug interface:
Graph-less: post-mortem analysis & tracing for graphs
Multi-node assembling in a process
Copy-free iGPU buffers
Cross containers/sessions/users (agentic isolation)
Config layer: yaml, proto, CLI flags

Dora/MCap/Scheduling/SMM/LMM/Blackbox/Watchdog/CommsRelay

Applications: high-perf AV/robotics; “local” agentic?
