# Research backlog

Unscheduled directions beyond the [current roadmap](roadmap.md). Component plans
own implementation details and validation evidence.

| Direction | Questions to resolve |
|---|---|
| GPU scheduling | Green contexts, hardware partitions, admission and effective allocation |
| GPU-to-storage recording | DMA paths, buffer lifetime, fallback copies and durability |
| Transport | PCIe endpoints, cross-machine correlation and deployment-driven CXL support |
| Composition | Bounded multithreaded executors, inbox policy and component failure domains |
| Diagnostics | Permissioned local agent APIs, profiler budgets and audited remediation |
| Deployment | Persistent boot policy, broader process adoption and fleet-system integration |
| Platform integration | NvSci/NvMedia SDK selection and Jetson qualification |

The [original architecture sketch](image.png) records early module labels.
SMM, LMM and CommsRelay still need agreed public meanings before becoming APIs.
