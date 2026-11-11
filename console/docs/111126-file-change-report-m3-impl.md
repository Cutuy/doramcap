# File changes in Console: milestone 3

Created November 11, 2026. Implements [milestone 3 of #316](https://github.com/Cutuy/doramcap/blob/cb94db8da1014c10aa2bec81857f30adcb80d274/console/docs/111026-file-change-report-impl.md#milestone-3-persist-one-report-and-expose-preview-plus-json-lines).
[Agreed architecture](https://github.com/Cutuy/doramcap/blob/cb94db8da1014c10aa2bec81857f30adcb80d274/console/docs/111026-file-change-report.md).

## Milestone 3: Save and show file changes

**Work items**

- [x] Add generated category summaries, coverage, report state and run-relative references.
- [x] Sync manifests and directories, then atomically publish `filesystem/report.pb` before destructive finalization. Carry its small reference through native execution; retain full source notes on disk.
- [x] Save an 8 KiB encoded preview with the marker/History transaction and in `console-report.json`. Preserve missing, failed, partial-coverage and complete-zero observations.
- [x] Reuse Result for current runs and History, showing counts, coverage, a whole-record preview and available before/after metadata.
- [x] Stream the authenticated `file-changes` artifact from protobuf manifests with backpressure, cancellation and verified per-kind counts. Emit category summaries, full coverage notes, every change, then a completion record. Preserve raw bytes as base64 and 64-bit values as decimal strings.
- [x] Keep recording byte totals and existing artifact names compatible. Reserve the new artifact ID.

The schema stays beside the current collector in `docker-engine`; milestone 1 relocates that owner. Systemd reports unavailable until milestone 2 adds capture. Extraction, systemd capture and keep-with-the-run remain outside this implementation.

**Validation**

- [x] Native release check, existing filesystem capture/codec tests, server and frontend type checks.
- [x] Large manifest readback (3,000 changes), full notes, invalid UTF-8, 64-bit extremes, oversized display records, zero/absent/failed observations, malformed/truncated/missing manifests and per-kind count mismatch.
- [x] Run confinement, traversal/cross-run/symlink escape refusal, session authorization, failed-run persistence, navigation, service restart, HTTP completion/failure and descriptor closure after abort.
- [x] Shared Result/History rendering, path escaping and recording byte totals.
- [ ] Desktop/narrow browser inspection and complete focused Console suite.
- [ ] Final source Clippy/boundaries and ARM64 worker results.
- [ ] Real Compose refresh/restart qualification; real systemd capture waits for milestone 2. Full local CI and packaged mount/GPU qualification remain integration checks.

The preview budget measures the encoded FileChanges section, including summaries, notes and whole change records. The 3,000-record fixture exceeds it while the independent JSON Lines reader retains the full observation. Capture/export has no preview-derived record limit. A completion line confirms export traversal; `coverageComplete` separately describes capture scope.
