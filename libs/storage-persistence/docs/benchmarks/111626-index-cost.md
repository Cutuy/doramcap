# MCAP index cost samples

Created: 2026-11-16. Linux x86_64, release builds, locked dependencies.
Base: `e3fc8911`; implemented runtime: `94058395`.

## Workloads

Each run records through the existing APIs and finishes with truncation and sync.
Payload filling, admission, backpressure, finalization and pacing are timed; setup and wire inspection are excluded.
Low rate uses eight messages with 20 ms pauses. Bursts use four groups of sixteen messages, with 20 ms between groups.
Saturation sends 8,192 combining messages, 128 large messages or 512 slabs without pacing.
Payloads are 256 bytes, 1 MiB + 7 bytes and 64 KiB + 7 bytes, respectively.
Each writer has four initial 64-KiB managed buffers, sixteen pending slots and two declared channels. The slab pool has four caller-owned buffers.
One managed allocation is warmed; later pool growth remains in the measurements.
Base/current runs alternate. The driver checks the first data opcode before accepting each sample.

## Median timings

P99 measures individual message admission, including explicit flushes at low rate and burst ends. Wall time includes the final sync.
The short combining saturation runs varied; that row uses ten additional isolated alternating pairs. Other rows use three pairs.

| Path / load | Pairs | Wall ms, base / current | CPU ms, base / current | P99 us, base / current |
| --- | ---: | ---: | ---: | ---: |
| combine / low | 3 | 169.943 / 169.518 | 2.814 / 2.887 | 536.782 / 442.931 |
| combine / burst | 3 | 66.798 / 66.794 | 1.800 / 1.924 | 410.660 / 434.200 |
| combine / saturated | 10 | 22.306 / 22.521 | 7.182 / 6.894 | 0.957 / 0.900 |
| large / low | 3 | 173.457 / 172.776 | 6.473 / 6.759 | 1252.501 / 1074.470 |
| large / burst | 3 | 105.445 / 100.278 | 60.736 / 44.282 | 6449.252 / 5279.770 |
| large / saturated | 3 | 95.337 / 93.404 | 113.452 / 107.410 | 3266.711 / 1385.989 |
| slab / low | 3 | 170.079 / 168.949 | 3.202 / 2.881 | 749.381 / 566.266 |
| slab / burst | 3 | 75.284 / 72.701 | 11.769 / 9.564 | 878.595 / 926.069 |
| slab / saturated | 3 | 53.678 / 55.580 | 52.148 / 55.472 | 519.308 / 542.791 |

## Bytes and allocations

Optional indexes remain inside the selected data extent. All sampled data extents match the base except saturated combining: the mandatory Chunk header changes batch packing, adding 8,192 data bytes in that workload.
The resulting 39 combining chunks omit all Message Index records. Low-rate combining has eight indexed chunks; burst combining has four chunks with two indexes each.
Large-message and slab samples index every chunk. Their data extents match the base at all three rates.

| Saturated path | Data bytes, base / current | Added summary bytes |
| --- | ---: | ---: |
| combine | 2,355,200 / 2,363,392 | 3,010 |
| large | 134,742,016 / 134,742,016 | 10,787 |
| slab | 35,651,584 / 35,651,584 | 42,659 |

Allocation counts include buffer growth, slab sealing and per-file summary growth. Low-rate combining and large writes change from zero to fourteen allocations in these eight-message samples. Low-rate slabs change from eight to twenty-two. Stage metrics add no allocations against the same writer workload.
The existing warmed 64-MiB large-message benchmark records sixteen allocations with summaries, compared with zero in the base. Its verified alternating runs measured roughly 1.3 GiB/s for both revisions on this host.

## Reproduce

The [standalone driver](111626-index-cost.rs) uses the public storage APIs and `libc`, already present in the crate. Copy it to a temporary `storage-persistence/examples/index_cost.rs` in each checkout, build unprivileged with `cargo build --release --locked -p storage-persistence --example index_cost`, and elevate only the executable when registered I/O needs capabilities.
Run the base binary with `base` and the implementation binary with `current`. Add `combine-saturated` to isolate that case. Use separate Cargo target directories and save distinct binaries before alternating runs.

[Architecture](../111626-opportunistic-mcap-indexes.md) · [Implementation milestones](../111626-opportunistic-mcap-indexes-impl.md).
