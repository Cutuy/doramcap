# Memory

Memory allocation, mapping, sharing, and access lifetimes across CPUs, devices,
and processes. Backend-specific crates describe their ownership, synchronization,
and platform requirements in their APIs.

| Crate | Responsibility |
|---|---|
| [cuda-host-memory](cuda-host-memory/) | CPU/GPU-accessible storage with runtime borrow checking |
| [cuda-dora-shm](cuda-dora-shm/) | Fixed-slot shared payloads, process leases, and CUDA ordering for Dora |

Future PCIe BAR mappings and NvMedia/NvSciBuf memory implementations belong in
this group. The current implementations use CUDA; those additional backends are
not yet implemented.
