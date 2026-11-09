# Compose type evaluation: probe and validation

Created: 2026-11-08. [Findings and options](110826-compose-types.md).

## Milestone 1 — Inventory and publish

**Work items**

- [x] Inspect Compose `Value` owners/callers at `2b900e5691a3c65467162682396c29b6467985c9`.
- [x] Check maintenance and published implementations.
- [x] Add isolated Cargo package, pinned candidates and lockfile.
- [x] Probe sanitized fixtures and both repository Dora Compose files.
- [ ] Publish evaluation PR after first successful probe.

**Validation**

- [x] Initial locked release build and ten-fixture run.
- [x] Raw JSON control passes before each candidate comparison.
- [x] Separate parse failures, serialization changes, Compose rejection and changes
  after Compose re-rendering. Report JSON pointers without fixture values.
- [x] Format probe and check whitespace.

## Milestone 2 — Confirm blockers and review

**Work items**

- [ ] Isolate strict binds, tmpfs mode, configs and external resources.
- [ ] Exercise typed service mutation on a supported fixture.
- [ ] Record final observations and update compatibility table.
- [ ] Review implementation/design guidance with Scanner.

**Validation**

- [ ] Final locked release probe, formatting and release Clippy.
- [ ] Repeat changed probe to verify recorded observations.
- [ ] Check documentation links and SVG structure.
- [ ] Mark PR ready and release lease after recording results.

Full application CI, Docker lifecycle tests, ARM64 and GPU execution are deferred:
this package only runs `docker compose config` and adds no application changes.
These checks do not establish runtime, GPU or platform qualification. No containers,
networks or volumes are created by the probe.

## Reproduce

From repository root:

```sh
cargo run --release --locked --manifest-path libs/workload-docker/evaluations/compose-types/Cargo.toml
cargo fmt --check --manifest-path libs/workload-docker/evaluations/compose-types/Cargo.toml
cargo clippy --release --locked --manifest-path libs/workload-docker/evaluations/compose-types/Cargo.toml -- -D warnings
```

Requires installed Docker Compose. Candidates: `docker-compose-types = 0.24.0`
with `indexmap,norway`, and `compose_spec = 0.3.0`. This is an independent Cargo
workspace; application manifest/lockfile are unchanged. Nonzero exit means a broken
fixture, raw control or probe; successful execution may report incompatibilities.

`serialization_changes` compares resolved JSON to crate output.
`normalized_changes` compares it to Compose's rendering of that output. `null`
means the comparison was not reached; `[]` means equality for that case.
This targeted set does not establish full schema coverage or compatibility with
other Compose versions.
