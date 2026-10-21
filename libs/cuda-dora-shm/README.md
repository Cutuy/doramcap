# CUDA host-memory IPC for Dora

Bounded, fixed-slot SPMC payloads. Every process uses one `mmap → mlock → cudaHostRegister` for the entire ring.
Independent slot leases share the registered backing. Lock failure aborts setup.
Dora carries control messages; CUDA Ready/Done events order GPU work.

![Architecture](docs/fixed-slot-ipc.svg)

```rust,ignore
let Some(mut frame) = producer.try_acquire(stream)? else { return Ok(()) };
frame.with_cpu_write(|bytes| bytes.fill(42))?;
let descriptor = frame.publish(4096, sequence)?;
cuda_dora_shm::send(&mut node, "frames", &descriptor)?;
```

- Same host/GPU, fixed consumer membership; managed memory is rejected.
- Set `PoolConfig::slot_alignment` explicitly; `DEFAULT_SLOT_ALIGNMENT` is 4096.
  Slot padding preserves the logical payload length.
- Each process needs memlock allowance for the entire page-rounded ring.
- Full pool returns `None`. Call `reclaim_completed` while processing returns.
- Metadata publication is asynchronous; slot retirement queries completion.
- Trusted peers and kernel launches have explicit `unsafe` contracts.
- This is an application adapter. Dora/Zenoh owns message storage; the adapter
  tracks shared payload lifetime. Remote payload copying is not implemented.

[SPMC demo](../../apps/cuda-shm-demo/README.md) ·
[Design and validation](docs/102126-fixed-slot-ipc.md) ·
[API and safety contracts](src/lib.rs)
