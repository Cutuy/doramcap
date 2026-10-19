# Measured MessagePack → Protobuf size difference

A live eBPF sample measured on 2026-10-18 contained 3,080 CPU/cgroup rows,
zero task records and zero inline dictionary entries. The comparison used the
same values in both formats: Protobuf was serialized after removing viewer-only
ingress metrics; MessagePack size was calculated using the previous v1
positional-array layout and compact integer/string/array encoding rules.

| Payload | Previous MessagePack | Protobuf v2 |
|---|---:|---:|
| One numeric snapshot | 58,727 bytes | 64,026 bytes |
| Snapshot payload rate at exactly 10 Hz | 587,270 bytes/s | 640,260 bytes/s |
| Dictionary checkpoint (153 entries, about every 2 s) | 10,179 bytes | 10,442 bytes |

For this sample, snapshots grew **9.02%**, adding **52,990 bytes/s** at 10 Hz.
This explains the observed dashboard increase from roughly 580 KB/s to
630 KB/s; live rates also vary with row counts, counter values and publication
timing. Rates above exclude dictionary traffic, transport framing and MCAP
container/schema overhead; KB denotes 1,000 bytes.

Across the CPU rows, Protobuf used 39,944 bytes for numeric varints, 17,839
bytes for field tags and 6,160 bytes for nested-message headers. MessagePack
used 55,562 bytes for numeric values and 3,080 bytes for row-array headers.
Protobuf's integer savings partially offset its per-field and per-row overhead.
The migration provides explicit schemas, additive-field compatibility and
embedded MCAP descriptors; payload size depends on the representation and data.
Packed numeric columns are a possible future optimization. This measurement
covers payload size only; encoding/decoding CPU performance was not benchmarked.
