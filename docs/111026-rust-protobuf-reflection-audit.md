# Rust protobuf reflection audit

Reviewed: 2026-11-10. Baseline: `4e69fd91e750f30b2722f9cae6e3e079bf3926b3` (`origin/main`).
Reference: [typed cgroup artifact correction](https://github.com/Cutuy/doramcap/commit/4a1296f4bf301b5ecf502cafa5d6469d354ff507).

Rechecked against `095fcca18a33a8ba07538452c6b3249e7c9d2dde`; source links stay pinned to the baseline.

Two libraries carried text parsing that only their examples used. This change moves
it into the examples. The recorder report stays pbtxt. Runtime behavior and artifact
formats are unchanged.

## Change: example-only text parsers leave the libraries

[workload-systemd](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-systemd/src/lib.rs#L35) exported `parse_plan` and `parse_run`;
[workload-supervisor](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-supervisor/src/lib.rs#L28) exported `parse_run`.
Their only callers were the `systemd_run` and `supervisor_monitor` examples and the
libraries' unit tests. Console builds `RunPlan` and `SystemdPlan` in TypeScript and
the native addon passes Rust the generated messages (D-03, D-05).

- Each example now parses its own pbtxt arguments. Usage and input files are unchanged.
- `prost-reflect` is a dev-dependency of both libraries. `workload-supervisor` no longer
  links it in production. `workload-systemd` still links it through `resource-policy`,
  which parses the operator's Deployment file.
- Unit tests build generated messages directly. The two assertions that exercised
  text-parser strictness left with the parsers.

## Reviewed: recorder report stays pbtxt

[report_text](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/mcap-recorder/src/config.rs#L76) receives a generated
`RecordingReport`, transcodes it to a `DynamicMessage` and formats pbtxt.
[save_report](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/mcap-recorder/src/recording.rs#L27) writes it three times per
recording, outside the per-message path.

- The report is the operator's result file beside the MCAP: state, error and per-input counts.
- No program reads it. The only reader is the [recorder unit test](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/mcap-recorder/src/tests.rs#L36).
- The recorder keeps `prost-reflect` for `RecorderConfig` loading either way.

The [typed cgroup artifact](https://github.com/Cutuy/doramcap/commit/4a1296f4bf301b5ecf502cafa5d6469d354ff507)
differs: code writes and reads it back. Revisit binary encoding when a program consumes the report.

## Reviewed text-format boundaries

Generated Rust messages provide binary encoding. The existing pbtxt contracts use
`prost-reflect` for text parsing/formatting. Replacing those calls with binary
encoding would change their file or API contracts.

| Location | Observed use and disposition |
| --- | --- |
| [Recorder config](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/mcap-recorder/src/config.rs#L14) | Parses `RecorderConfig` pbtxt into the generated type and writes the pbtxt report reviewed above; retain. |
| [Console native API](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/console/server/native/src/lib.rs#L55) | Formats ConsoleConfig bytes as editable pbtxt and parses config/manifests; retain. [Release initialization](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/console/server/src/release.ts#L77) writes `console.pbtxt`; [CLI help](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/console/server/src/cli.ts#L60) explicitly supports editing it. |
| [Console bridge](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/console/bridge/src/config.rs#L7) | Parses BridgeConfig text, then validates the generated message; retain. |
| [System monitor](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/system-monitor/src/config.rs#L15) | Parses MonitorConfig pbtxt and validates settings; retain. |
| [Fault watchdog](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/fault-watchdog/src/config.rs#L153) | Parses WatchdogConfig pbtxt and validates settings; retain. |
| [Resource policy](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/resources/resource-policy/src/config.rs#L11) | Parses Deployment text, then validates IDs and policies; retain. |
| [Project registry](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/project-explorer/server/src/registry.rs#L123) | Parses text registrations into generated AppRegistration; retain. |
| [Provenance demo](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/dataflow-tracing-demo/server/src/monitor.rs#L86) | Parses the user-selected measurement definitions file; retain. |
| [Docker app overlay](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-docker/src/compose/adapter/configuration.rs#L425) | Parses shipped app-manifest pbtxt before creating native Compose settings; retain. |
| [Systemd example](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-systemd/src/lib.rs#L35) | Parsed SystemdPlan/RunPlan text in the library for the example; moved into `systemd_run`. |
| [Supervisor example](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-supervisor/src/lib.rs#L28) | Parsed MonitorRun text in the library for the example; moved into `supervisor_monitor`. |
| [Recorder tests](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/mcap-recorder/src/tests.rs#L36) | Reads the pbtxt report; retain. |
| [Compose contract tests](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-docker/tests/compose_contract.rs#L30) | Exercises real pbtxt/wire/serde input contracts; retain. |
| [Docker Dora fixture](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-docker/tests/docker.rs#L3147) | Reads a shipped manifest, edits the generated message and writes pbtxt for the production parser; retain the text-format integration coverage. |
| [Systemd Dora test](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-systemd/tests/native.rs#L1330) | Reads shipped app-manifest pbtxt into a generated message; retain. |

Descriptor export and `prost_types::FileDescriptorSet` checks in fault schema,
fault protocol and telemetry test schema compatibility/MCAP metadata. They operate
on descriptors intentionally and are outside the runtime-message cleanup candidate.

## Coverage and validation

- Searched all 281 tracked Rust files at the baseline: 15 files use message reflection,
  with 16 text parses and three text writes. All 15 owners and their callers were inspected.
- The three text writers are the recorder report, the editable Console config and the
  Docker test manifest fixture.
- Checked the locked `prost` 0.14.4 and `prost-reflect` 0.16.5 sources for encoding/transcoding behavior.
- Local x86: formatting, library boundaries, Clippy on all targets of both libraries and
  of the Console native addon, both libraries' unit tests, and both examples accepting
  valid pbtxt and rejecting an unknown field.
- Not run: full workspace CI, root-only native tests, ARM64 worker jobs.
