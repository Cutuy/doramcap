![Profile resolution on the host](110326-host-profile-resolution.svg)

# Resolved launch profiles

Created: 2026-11-02. Status: Milestones 1-4 implemented; [Milestone 5](#milestone-5-host-resolution) approved and replaces the container resolver.

## Approved architecture

- **Resolver:** `preparation::resolve` in Rust on the host, for the picked profile, at Review and again in "Prepare project".
- **Merge:** schema-free: a field set in an overlay replaces that top-level payload field.
- **Raw profiles:** stay on the host, outside the images.
- **Console:** names the picked profile's mount in `RunRequest`.
- **Apps:** load flat files only and run app-specific validation in their containers.
- **Every app with a config:** launches from a profile; an app with no profiles launches as Compose defines it.

## Implemented container resolver

![Container resolver](110226-resolved-launch-profiles.svg)

Launch profiles mount one pbtxt, so each profile repeats its base config. Review
resolves the selected profile once, and the run launches exactly that result.

- **Resolver:** the app, running from its own image. Include order, overlays, defaults and validation are app code.
- **Trigger:** Review, after images are present and before inspection.
- **Output:** one complete pbtxt, frozen with the run's inputs and mounted at `config_mount`.
- **Format:** `fault-watchdog`, `system-monitor` and `mcap-recorder` all load `pbtxt-config` fragments.
- **Opt-in:** an app without a resolver keeps today's mount, including the single-config `cpu-recorder`.

```protobuf
config_mount { service: "fault-watchdog" target: "/etc/fault-watchdog/watchdog.pbtxt" }
config_resolver { command: "/usr/local/bin/fault-watchdog" command: "--resolve-config" }
launch_profiles { id: "mcap" label: "Live and MCAP" config: "/absolute/config/watchdog-mcap.pbtxt" }
```

## File mounts

| Mount | Source | Frozen |
|---|---|---|
| Resolver input, one-shot only | profile's directory, read-only | No |
| `config_mount` | resolver output in the review workspace, mode 0644 | Yes, keeping its mode |
| Standalone Compose `/etc/fault-watchdog` | `config/`, read-only | No; Console covers the launched file |
| Release `cpu-monitor` `/config/monitor.pbtxt` | `live.pbtxt`, self-contained | Replaced by the resolved file at the same target |

## Alternatives

| Option | Outcome |
|---|---|
| Resolve in Console | Rejected: `Payload::overlay` and `normalize` are app code, so Console would need app types or a second merge that can drift |
| Mount and freeze the config directory | Rejected: Review would validate nothing, and the record would hold sources instead of the launched config |
| Flatten at build time | Rejected: operator-written profiles would need a separate step |

## Milestone 1: App resolver

Complete.

**Work items**
- [x] Add a `pbtxt-config` function that formats a resolved app as Fragment text, loadable by `resolve`: `Resolved::fragment_text`.
- [x] Add `fault-watchdog --resolve-config FILE`: print the resolved pbtxt to stdout; on failure print the error to stderr and exit nonzero.

**Validation**
- [x] Resolving a base plus overlay and loading the output returns the same config as loading the original: `fragment_text_resolves_to_the_same_deployment_without_its_includes`, `resolved_profile_loads_alone_with_the_config_of_its_includes`.
- [x] Resolving the output again returns identical bytes: the same two tests.
- [x] A profile requesting a sink the binary was built without fails to resolve: the second test, built without `otel`.

## Milestone 2: Console resolution

**Work items**
- [x] Add `config_resolver` to `SystemAppDefinition`; it requires `config_mount` and `launch_profiles`.
- [x] Run the resolver once per selected profile with `docker run`, using the image of the `config_mount` service.
- [x] Sandbox it: no network, read-only root, all capabilities dropped, `no-new-privileges`, the profile's directory mounted read-only, and bounded time and output.
- [x] Write the output to the review workspace and mount that file; Start launches the same file.
- [x] Report a failure with its stderr at Review.

**Validation**
- [x] Server tests cover resolver success, failure, timeout, oversized output and apps without a resolver: `config-resolver.test.ts`, using a fake Docker client.
- [x] A changed source file after Review leaves the launched config unchanged: `service.test.ts`, using a fake engine.
- [ ] A real Console run launches the resolved file, and the frozen input matches the resolver output. Pending: needs Docker access.

## Milestone 3: Watchdog profiles

**Work items**
- [x] Restore `include: "watchdog.pbtxt"` in the MCAP and OTLP standalone profiles, and delete the repeated fields.
- [x] Mount `config/` in the standalone Compose file, so direct Compose users select a profile through the service command.
- [x] Declare `config_resolver` in the [configuration example](configuration.md) and document the contract.
- [x] Replace the standalone test that loads each profile alone with one that loads it beside its base.

**Validation**
- [ ] Real Console runs of the live, MCAP and OTLP profiles finish clean with their expected outputs. Pending: needs Docker access.
- [ ] A direct Compose launch resolves a profile's include. Pending: needs Docker access.
- [x] The Console release session's watchdog app uses the resolver, or its plan records why it waits: the app is unmerged in [#96](https://github.com/Cutuy/doramcap/pull/96), which adopts `config_resolver` when it rebases.

## Milestone 4: Unified config format

**Work items**
- [x] Load `MonitorConfig` and `RecorderConfig` through `pbtxt_config::resolve`, under app IDs `monitor` and `recorder`.
- [x] Add `--resolve-config` to `system-monitor` and `mcap-recorder`, with every app sharing `pbtxt_config::resolve_flag`.
- [x] Make `task_capacity` and `samples` optional, so an overlay can set either back to zero.
- [x] Migrate every shipped and test-written monitor and recorder pbtxt to the shared format.
- [x] Make the release `record` profile include `live`, declare its resolver, and mount `live.pbtxt` by default in its Compose file.

**Validation**
- [x] An overlay inherits its base, and an explicit `task_capacity: 0` clears it: `a_profile_overrides_its_base_and_resolves_to_one_complete_file`.
- [x] Each resolved file loads alone to the same config and resolves to identical bytes: the same test, and `pbtxt_paths_resolve_relative_to_config_and_invalid_settings_fail`.
- [x] Relative paths keep resolving beside the entry file: both tests above and `relative_paths_resolve_beside_the_config`.
- [x] Shipped configs still load: `shipped_launches_select_their_documented_outputs`, and the release test checks the `record` profile's include and resolver.
- [ ] BPF, all-feature and native runs parse the new format. Pending: the local host has no `clang`, and these need CI and root.
- [ ] A real Console run of the `record` and `live` profiles finishes clean. Pending: needs Docker access.

`dataflow-tracing` measurements, Supervisor `MonitorRun` and project registrations stay as they are: each is a single document with no launch profiles.

## Milestone 5: Host resolution

Approved 2026-11-03. Not started.

**Work items**
- [ ] Present reputable schema-free pbtxt parser crates for evaluation; otherwise add a small parser to `pbtxt-config`.
- [ ] Add a schema-free flatten to `pbtxt-config`, keeping its cycle and size limits.
- [ ] Flatten named mounts in `freeze_files`, writing mode 0644, and snapshot other binds as they are.
- [ ] Add the profile mount field to `RunRequest`, and have Console bind the raw profile and name its mount.
- [ ] Delete `config-resolver.ts`, `config_resolver`, the resolver's image lookup, `resolve_flag` and `fragment_text`.
- [ ] Give `cpu-recorder` a profile and `config_mount`, and remove the release Compose config mounts.
- [ ] Reject `include` when an app loads its config; move `fault-watchdog` standalone direct Compose to flat files.
- [ ] Fail a release test when an app with a config has no profile.

**Validation**
- [ ] Flattening matches the existing overlay results for the shipped `cpu-monitor` and `fault-watchdog` profiles.
- [ ] A bad include fails Review, and fails "Prepare project" when it first appears at Start.
- [ ] A raw profile changed after Review fails the Start snapshot comparison.
- [ ] An app rejects a flat config with a bad value at Start, as a failed Compose workflow step.
- [ ] A real Console run of every release profile launches the frozen flat file and finishes clean.
