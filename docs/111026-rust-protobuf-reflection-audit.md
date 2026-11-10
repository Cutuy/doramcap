# Rust protobuf reflection audit

Reviewed: 2026-11-10. Baseline: `4e69fd91e750f30b2722f9cae6e3e079bf3926b3` (`origin/main`).
Reference: [typed cgroup artifact correction](https://github.com/Cutuy/doramcap/commit/4a1296f4bf301b5ecf502cafa5d6469d354ff507).

One production cleanup candidate repeats the reference's runtime-artifact pattern.
This audit flags the candidate and its format dependencies; runtime behavior is unchanged.

## Finding: recorder reports take a generated message through reflection

[report_text](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/mcap-recorder/src/config.rs#L76) already receives a generated
`RecordingReport`. Every call decodes the descriptor pool, looks up the message by
name, creates a `DynamicMessage`, transcodes the report, then formats pbtxt.
[save_report](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/mcap-recorder/src/recording.rs#L27) writes that text to the run's report file.

The call runs twice during successful creation and once during finalization;
it is outside the per-message recording path. The locked `prost-reflect` implementation
of `transcode_from` first encodes the generated message to binary, then merges those
bytes into the dynamic message. Direct `prost::Message::encode_to_vec` could provide
the artifact bytes without that descriptor lookup, dynamic decode and text formatting.

This is a design cleanup candidate under D-02/D-05. The audit establishes extra
conversion work; it does not establish a correctness defect or measured performance impact.

### Follow-up requirements

- Agree on changing the report artifact from pbtxt to binary protobuf before implementation.
- Update [the report reader](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/mcap-recorder/src/tests.rs#L36), shipped recorder configs under
  `apps/mcap-recorder/deploy`, `console/release/system-apps/data-recorder`, and the Console example.
- Update the Docker recorder fixture/config and Dora validation scripts that name `.report.pbtxt`.
- Preserve exclusive creation, truncate/seek/write/fsync behavior, report state/counters,
  and independent MCAP-finalization/report-write errors. Verify a replacement reader
  across running, drained, interrupted and failed recordings.
- Keep the descriptor and reflection dependency required by `RecorderConfig` pbtxt loading.
  Changing the report serializer alone does not remove that input requirement.

## Reviewed text-format boundaries

Generated Rust messages provide binary encoding. The existing pbtxt contracts use
`prost-reflect` for text parsing/formatting. Replacing those calls with binary
encoding would change their file or API contracts.

| Location | Observed use and disposition |
| --- | --- |
| [Recorder config](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/mcap-recorder/src/config.rs#L14) | Parses `RecorderConfig` pbtxt into the generated type; retain. Report output is the finding above. |
| [Console native API](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/console/server/native/src/lib.rs#L55) | Formats ConsoleConfig bytes as editable pbtxt and parses config/manifests; retain. [Release initialization](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/console/server/src/release.ts#L77) writes `console.pbtxt`; [CLI help](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/console/server/src/cli.ts#L60) explicitly supports editing it. |
| [Console bridge](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/console/bridge/src/config.rs#L7) | Parses BridgeConfig text, then validates the generated message; retain. |
| [System monitor](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/system-monitor/src/config.rs#L15) | Parses MonitorConfig pbtxt and validates settings; retain. |
| [Fault watchdog](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/fault-watchdog/src/config.rs#L153) | Parses WatchdogConfig pbtxt and validates settings; retain. |
| [Resource policy](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/resources/resource-policy/src/config.rs#L11) | Parses Deployment text, then validates IDs and policies; retain. |
| [Project registry](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/project-explorer/server/src/registry.rs#L123) | Parses text registrations into generated AppRegistration; retain. |
| [Provenance demo](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/dataflow-tracing-demo/server/src/monitor.rs#L86) | Parses the user-selected measurement definitions file; retain. |
| [Docker app overlay](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-docker/src/compose/adapter/configuration.rs#L425) | Parses shipped app-manifest pbtxt before creating native Compose settings; retain. |
| [Systemd parsers](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-systemd/src/lib.rs#L35) | Parses SystemdPlan/RunPlan text into generated messages; retain. |
| [Supervisor parser](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-supervisor/src/lib.rs#L28) | Parses MonitorRun text into its generated type; retain. |
| [Recorder tests](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/apps/mcap-recorder/src/tests.rs#L36) | Reads the current pbtxt report; migrate with the flagged writer. |
| [Compose contract tests](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-docker/tests/compose_contract.rs#L30) | Exercises real pbtxt/wire/serde input contracts; retain. |
| [Docker Dora fixture](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-docker/tests/docker.rs#L3147) | Reads a shipped manifest, edits the generated message and writes pbtxt for the production parser; retain the text-format integration coverage. |
| [Systemd Dora test](https://github.com/Cutuy/doramcap/blob/4e69fd91e750f30b2722f9cae6e3e079bf3926b3/libs/workload-systemd/tests/native.rs#L1330) | Reads shipped app-manifest pbtxt into a generated message; retain. |

Descriptor export and `prost_types::FileDescriptorSet` checks in fault schema,
fault protocol and telemetry test schema compatibility/MCAP metadata. They operate
on descriptors intentionally and are outside the runtime-message cleanup candidate.

## Coverage and validation

- Searched all 281 tracked Rust files: 15 files use message reflection, including
  16 text parses and three text writes. All 15 owners and their relevant callers were inspected.
- Traced the three text writers: recorder report candidate, editable Console config,
  and the Docker test manifest fixture. No additional production match was found.
- Checked the locked `prost` 0.14.4 and `prost-reflect` 0.16.5 sources for encoding/transcoding behavior.
- Checked source references, inventory completeness and the documentation diff.
  Runtime tests, benchmarks, native qualification and worker jobs were not run for this audit.
- Excludes generated build output, external dependencies and unrelated unmerged branches.
  Governing guides and process instructions are unchanged.
