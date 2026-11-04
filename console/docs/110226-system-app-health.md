![System app health](110226-system-app-health.svg)

# Console system-app release

Created: 2026-11-02. PR 96 integrates the prerequisites below; all PRs remain open for review.

- [plain-pbtxt-apps](https://github.com/Cutuy/doramcap/pull/132)
- [direct-resource-policy](https://github.com/Cutuy/doramcap/pull/133)
- [system-app-health](https://github.com/Cutuy/doramcap/pull/134)
- [live-bridge-pbtxt](https://github.com/Cutuy/doramcap/pull/135)
- [recorder-launch-profiles](https://github.com/Cutuy/doramcap/pull/136)

## Milestone 1: Wire and package system apps

**Work items**

- [x] Wire selected health outputs to consumers through bounded queues.
- [x] Package CPU monitoring, watchdog and generic recording with pbtxt launches.
- [x] Add live, MCAP and OTLP watchdog profiles and watchdog recording inputs.

**Validation**

- [x] Graph and release tests pass.
- [x] Six container scenarios and CPU recording equality pass.
- [x] Full local CI and ARM64 worker checks pass.
- [x] Hosted Node checks pass on x86 and ARM64.
- [ ] Hosted Rust checks pass on x86 and ARM64 (running).
