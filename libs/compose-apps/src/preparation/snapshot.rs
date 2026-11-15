//! Compare the effective Compose configuration across independently prepared runs.
use super::*;
use crate::compose::adapter::{ARTIFACTS_LABEL, RUN_LABEL};
use prost::Message;
use workload_lifecycle::launch::DoraRuntime;

fn relative(path: &str, artifacts: &Path) -> String {
    Path::new(path).strip_prefix(artifacts).map_or_else(
        |_| path.to_owned(),
        |path| format!("@artifacts/{}", path.to_string_lossy()),
    )
}

fn path_value(value: &mut Value, artifacts: &Path) {
    if let Some(path) = value.as_str() {
        *value = relative(path, artifacts).into();
    }
}

// Generated runtime paths have the same meaning in every Compose field.
fn runtime_paths(value: &mut Value, runtime: &str, replacement: &str) {
    if runtime.is_empty() || runtime == replacement {
        return;
    }
    match value {
        Value::String(path) if path == runtime => *path = replacement.into(),
        Value::String(text) => {
            *text = text.replace(&format!("{runtime}/"), &format!("{replacement}/"));
        }
        Value::Array(values) => {
            for value in values {
                runtime_paths(value, runtime, replacement);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                runtime_paths(value, runtime, replacement);
            }
        }
        _ => {}
    }
}

fn manifest(
    mut model: Value,
    artifacts: &Path,
    systems: &[String],
    dora: &crate::dora::DoraApps,
    runtime: Option<&DoraRuntime>,
) -> Value {
    let automatic_ports = runtime.map_or([false; 2], |runtime| {
        [
            runtime.automatic_coordinator_port,
            runtime.automatic_daemon_port,
        ]
    });
    // Ownership labels and generated volume names identify a run rather than its configuration.
    for kind in ["services", "networks", "volumes"] {
        if let Some(items) = model.get_mut(kind).and_then(Value::as_object_mut) {
            for item in items.values_mut() {
                let owned = item["labels"].get(RUN_LABEL).is_some();
                if let Some(labels) = item.get_mut("labels").and_then(Value::as_object_mut) {
                    labels.remove(RUN_LABEL);
                    labels.remove(ARTIFACTS_LABEL);
                }
                if kind == "volumes" && owned {
                    item.as_object_mut().unwrap().remove("name");
                }
            }
        }
    }
    // Normalize generated runtime paths and automatically assigned Dora endpoints.
    let runtime_directory = artifacts
        .join("dora/runtime")
        .to_string_lossy()
        .into_owned();
    let runtime_directory = runtime.map_or(runtime_directory.as_str(), |runtime| {
        runtime.runtime_directory.as_str()
    });
    runtime_paths(&mut model, runtime_directory, "@artifacts/dora/runtime");
    let overlay = runtime
        .and_then(|runtime| runtime.zenoh.as_ref())
        .map_or("", |zenoh| zenoh.target.as_str());
    if let Some(services) = model.get_mut("services").and_then(Value::as_object_mut) {
        for (name, service) in services {
            let runtime = dora.services.contains_key(name);
            // Generated groups name the run; user-set parents remain significant.
            if let Some(parent) = service.get_mut("cgroup_parent")
                && let Some((_, rest)) = parent
                    .as_str()
                    .and_then(crate::cgroup::split_service_parent)
            {
                let root = if rest.starts_with('/') { "/" } else { "" };
                *parent = format!("{root}{RUN_CGROUP}{rest}").into();
            }
            if let Some(mounts) = service.get_mut("volumes").and_then(Value::as_array_mut) {
                for mount in mounts {
                    let frozen = mount["source"].as_str().is_some_and(|source| {
                        Path::new(source).starts_with(artifacts.join("inputs"))
                    });
                    let attachment = mount["target"] == overlay || mount["source"] == overlay;
                    if frozen || runtime || systems.contains(name) || attachment {
                        path_value(&mut mount["source"], artifacts);
                    }
                    if runtime || attachment {
                        path_value(&mut mount["target"], artifacts);
                    }
                }
            }
            if let Some(environment) = service
                .get_mut("environment")
                .and_then(Value::as_object_mut)
            {
                for (key, value) in environment.iter_mut() {
                    if key == dora_app::environment::DAEMON_PORT && automatic_ports[1] {
                        *value = "@dora/daemon-port".into();
                    }
                    if value == overlay {
                        path_value(value, artifacts);
                    }
                }
            }
            if runtime {
                for field in ["entrypoint", "command"] {
                    if let Some(command) = service.get_mut(field).and_then(Value::as_array_mut) {
                        let options = [
                            ("--port", automatic_ports[0], "@dora/coordinator-port"),
                            (
                                "--coordinator-port",
                                automatic_ports[0],
                                "@dora/coordinator-port",
                            ),
                            (
                                "--local-listen-port",
                                automatic_ports[1],
                                "@dora/daemon-port",
                            ),
                        ];
                        for index in 0..command.len() {
                            for (flag, automatic, replacement) in options {
                                if !automatic {
                                    continue;
                                }
                                if command[index] == flag && index + 1 < command.len() {
                                    command[index + 1] = replacement.into();
                                } else if command[index]
                                    .as_str()
                                    .is_some_and(|arg| arg.starts_with(&format!("{flag}=")))
                                {
                                    command[index] = format!("{flag}={replacement}").into();
                                }
                            }
                            path_value(&mut command[index], artifacts);
                        }
                    }
                }
            }
        }
    }
    for kind in ["configs", "secrets"] {
        if let Some(items) = model.get_mut(kind).and_then(Value::as_object_mut) {
            for item in items.values_mut() {
                path_value(&mut item["file"], artifacts);
            }
        }
    }
    model
}

/// Run-independent stand-in for the generated run group name.
const RUN_CGROUP: &str = "@run-cgroup";

/// Largest resolved execution manifest captured into a configuration snapshot.
const MAX_MANIFEST_BYTES: u64 = 64 * 1024 * 1024;

/// Safe-read budget shared with Console; oversized setup evidence remains unavailable.
const MAX_SETUP_BYTES: usize = 4 * 1024 * 1024;
const SETUP_VERSION: u32 = 2;

fn invocation(service: &Value) -> Result<Invocation> {
    use invocation_value::Value as Field;
    let field = |name: &str| -> Result<Option<InvocationValue>> {
        let value = match service.get(name) {
            None | Some(Value::Null) => return Ok(None),
            Some(Value::String(text)) if text.is_empty() => {
                Field::Arguments(InvocationArguments::default())
            }
            Some(Value::String(text)) => Field::Text(text.clone()),
            Some(Value::Array(args)) => Field::Arguments(InvocationArguments {
                values: args
                    .iter()
                    .map(|arg| {
                        arg.as_str()
                            .map(str::to_owned)
                            .ok_or_else(|| invalid(format!("{name}: expected string arguments")))
                    })
                    .collect::<Result<_>>()?,
            }),
            _ => return Err(invalid(format!("{name}: invalid invocation value"))),
        };
        Ok(Some(InvocationValue { value: Some(value) }))
    };
    Ok(Invocation {
        command: field("command")?,
        entrypoint: field("entrypoint")?,
        stop_signal: match service.get("stop_signal") {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(text)) => text.clone(),
            _ => return Err(invalid("stop_signal: expected a string")),
        },
    })
}

/// Project supported comparison facts from the native model; JSON stays at the Compose boundary.
fn setup_record(captured: &Value, normalized: &Value, run: &str) -> Result<SetupRecord> {
    let services = captured["services"]
        .as_object()
        .ok_or_else(|| invalid("setup services missing"))?;
    let files = |kind: &str| {
        captured[kind]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(name, file)| {
                file["file"]
                    .as_str()
                    .map(|path| (name.clone(), path.to_owned()))
            })
            .collect()
    };
    Ok(SetupRecord {
        version: SETUP_VERSION,
        run_id: run.into(),
        services: services
            .iter()
            .map(|(name, service)| {
                let normalized = normalized["services"]
                    .get(name)
                    .ok_or_else(|| invalid("normalized service missing"))?;
                Ok((
                    name.clone(),
                    SetupService {
                        environment: service["environment"]
                            .as_object()
                            .into_iter()
                            .flatten()
                            .map(|(name, value)| {
                                let value = match value {
                                    Value::String(text) => text.clone(),
                                    Value::Null => String::new(),
                                    _ => {
                                        return Err(invalid(format!(
                                            "environment.{name}: expected rendered string"
                                        )));
                                    }
                                };
                                Ok((name.clone(), value))
                            })
                            .collect::<Result<_>>()?,
                        binds: service["volumes"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|mount| mount["type"] == "bind")
                            .map(|mount| {
                                Ok(SetupBind {
                                    source: mount["source"]
                                        .as_str()
                                        .ok_or_else(|| invalid("setup bind source missing"))?
                                        .into(),
                                    target: mount["target"]
                                        .as_str()
                                        .ok_or_else(|| invalid("setup bind target missing"))?
                                        .into(),
                                })
                            })
                            .collect::<Result<_>>()?,
                        captured: Some(invocation(service)?),
                        normalized: Some(invocation(normalized)?),
                    },
                ))
            })
            .collect::<Result<_>>()?,
        configs: files("configs"),
        secrets: files("secrets"),
        complete: true,
    })
}

/// Publish one private protobuf snapshot atomically; a save failure leaves execution available.
async fn save_setup(record: SetupRecord, artifacts: &Path) -> Result<()> {
    if record.encoded_len() > MAX_SETUP_BYTES {
        return Err(invalid("setup record exceeds 4 MiB"));
    }
    let bytes = record.encode_to_vec();
    let artifacts = artifacts.to_owned();
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        let mut temporary = tempfile::NamedTempFile::new_in(&artifacts)?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        temporary
            .persist(artifacts.join(SETUP_RECORD))
            .map_err(|error| error.error)?;
        Ok(())
    })
    .await
    .map_err(|error| invalid(format!("setup record writer: {error}")))?
}

pub(super) async fn capture(
    spec: &PrepareInput,
    artifacts: &Path,
    plan: &ResolvedComposeProject,
    system_services: &[String],
    user_services: &[String],
    dora: &crate::dora::DoraApps,
    runtime: Option<&DoraRuntime>,
) -> Result<ConfigurationSnapshot> {
    let bytes =
        crate::evidence::read_regular(Path::new(&plan.manifest), MAX_MANIFEST_BYTES).await?;
    if digest(&bytes) != plan.sha256 {
        return Err(invalid("execution manifest changed during resolution"));
    }
    let captured: Value = serde_json::from_slice(&bytes)?;
    let model = manifest(captured.clone(), artifacts, system_services, dora, runtime);
    let record = setup_record(&captured, &model, &spec.run_plan.id);
    let saved = match record {
        Ok(record) => save_setup(record, artifacts).await,
        Err(error) => Err(error),
    };
    if let Err(error) = saved {
        eprintln!("Could not save setup comparison: {error}");
    }
    let recordings: Vec<_> = spec
        .run_plan
        .recordings
        .iter()
        .map(|recording| {
            (
                relative(&recording.stream, artifacts),
                relative(&recording.recording, artifacts),
            )
        })
        .collect();
    let settings = serde_json::to_vec(&(
        &spec.filesystem,
        &spec.run_plan.private_records,
        spec.run_plan.host_shm_access,
        spec.run_plan.readiness_seconds,
        spec.run_plan.stop_seconds,
        recordings,
        dora.services.keys().collect::<Vec<_>>(),
    ))?;
    Ok(ConfigurationSnapshot {
        manifest_sha256: digest(&serde_json::to_vec(&model)?),
        system_services: system_services.to_vec(),
        user_services: user_services.to_vec(),
        settings_sha256: digest(&settings),
        graph_sha256: runtime.map_or_else(String::new, |runtime| runtime.dataflow_sha256.clone()),
        services: plan.services.clone(),
        remove_volumes: plan.remove_volumes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model(root: &str) -> Value {
        json!({"services": {
            "workload": {"image":"sha256:pinned", "command":["hello"], "labels":{RUN_LABEL:"run", ARTIFACTS_LABEL:root},
                "volumes":[{"type":"bind", "source":format!("{root}/inputs/abc-444"), "target":"/settings"}]},
            "collector": {"volumes":[{"type":"bind", "source":format!("{root}/recordings"), "target":"/results"}]}
        }, "volumes":{"data":{"name":format!("run_{root}"), "labels":{RUN_LABEL:"run"}}}})
    }
    #[test]
    fn generated_cgroup_parents_vary_by_run_and_preserve_user_configuration_identity() {
        // Arrange
        let dora = Default::default();
        let model = |run: &str| {
            json!({"services":{
                "collector":{"cgroup_parent":format!("doramcap-compose-0123456789ab-{run}-system-collector.slice")},
                "camera":{"cgroup_parent":format!("/doramcap-compose-0123456789ab/{run}/user/camera")},
                "workload":{"cgroup_parent":"/user"}
            }})
        };
        let first = "000000000001";
        let second = "000000000002";
        let normalize = |model| {
            manifest(
                model,
                Path::new("/artifacts"),
                &["collector".into()],
                &dora,
                None,
            )
        };
        // Act / Assert
        assert_eq!(normalize(model(first)), normalize(model(second)));
        // Act / Assert: native user placement remains a meaningful reviewed setting.
        let mut changed = model(first);
        changed["services"]["workload"]["cgroup_parent"] = "/different-user".into();
        assert_ne!(normalize(model(first)), normalize(changed));
    }
    #[test]
    fn generated_paths_compare_equal_but_commands_images_and_host_binds_remain_exact() {
        // Arrange
        let dora = Default::default();
        let initial = manifest(
            model("/review"),
            Path::new("/review"),
            &["collector".into()],
            &dora,
            None,
        );
        // Act / Assert
        assert_eq!(
            initial,
            manifest(
                model("/execution"),
                Path::new("/execution"),
                &["collector".into()],
                &dora,
                None,
            )
        );
        for (field, value) in [
            ("command", json!(["changed"])),
            ("image", json!("sha256:new")),
            ("environment", json!({"CUSTOM":"/execution"})),
        ] {
            let mut changed = model("/execution");
            changed["services"]["workload"][field] = value;
            assert_ne!(
                initial,
                manifest(
                    changed,
                    Path::new("/execution"),
                    &["collector".into()],
                    &dora,
                    None,
                )
            );
        }
        let mut changed = model("/execution");
        changed["services"]["workload"]["volumes"][0]["source"] =
            "/execution/live-host-directory".into();
        assert_ne!(
            initial,
            manifest(
                changed,
                Path::new("/execution"),
                &["collector".into()],
                &dora,
                None,
            )
        );
    }
    #[tokio::test]
    async fn snapshot_detects_policy_graph_service_and_manifest_changes() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("experiment.json");
        let bytes = serde_json::to_vec(&model(root.path().to_str().unwrap())).unwrap();
        tokio::fs::write(&path, &bytes).await.unwrap();
        let mut spec = PrepareInput {
            run_plan: workload_lifecycle::launch::RunPlan {
                host_shm_access: workload_lifecycle::launch::HostShmAccess::Passthrough as i32,
                id: "compare".into(),
                artifacts: root.path().to_string_lossy().into_owned(),
                repository_root: root.path().to_string_lossy().into_owned(),
                dataflow_file: String::new(),
                recordings: vec![],
                readiness_seconds: 60,
                stop_seconds: 15,
                ..Default::default()
            },
            extra_apps: vec![],
            system_apps: vec![],
            dora_apps: vec![],
            filesystem: None,
            mcap_writers: Default::default(),
        };
        let dora = crate::dora::DoraApps::default();
        let mut runtime = DoraRuntime::default();
        let mut plan = ResolvedComposeProject {
            manifest: path.to_string_lossy().into(),
            sha256: digest(&bytes),
            ..Default::default()
        };
        let original = capture(
            &spec,
            root.path(),
            &plan,
            &["collector".into()],
            &["workload".into()],
            &dora,
            Some(&runtime),
        )
        .await
        .unwrap();
        assert_eq!(
            original.settings_sha256,
            "74a4ed7003026f69d1a90243f398cac4f7d03b04aab04863bbf1a49f9afdcced"
        );
        // Act / Assert: changing caller-owned private records invalidates the reviewed input.
        spec.run_plan
            .private_records
            .push("caller-record.json".into());
        assert_ne!(
            original,
            capture(
                &spec,
                root.path(),
                &plan,
                &["collector".into()],
                &["workload".into()],
                &dora,
                Some(&runtime)
            )
            .await
            .unwrap()
        );
        spec.run_plan.private_records.clear();
        // Act / Assert: policy, stages and graph contents are compared independently.
        spec.run_plan.stop_seconds += 1;
        assert_ne!(
            original,
            capture(
                &spec,
                root.path(),
                &plan,
                &["collector".into()],
                &["workload".into()],
                &dora,
                Some(&runtime)
            )
            .await
            .unwrap()
        );
        spec.run_plan.stop_seconds -= 1;
        assert_ne!(
            original,
            capture(
                &spec,
                root.path(),
                &plan,
                &["workload".into()],
                &["collector".into()],
                &dora,
                Some(&runtime)
            )
            .await
            .unwrap()
        );
        runtime.dataflow_sha256 = "changed".into();
        assert_ne!(
            original,
            capture(
                &spec,
                root.path(),
                &plan,
                &["collector".into()],
                &["workload".into()],
                &dora,
                Some(&runtime)
            )
            .await
            .unwrap()
        );
        runtime.dataflow_sha256.clear();
        plan.services.insert(
            "workload".into(),
            ComposeService {
                replicas: 1,
                completed: true,
                ..Default::default()
            },
        );
        assert_ne!(
            original,
            capture(
                &spec,
                root.path(),
                &plan,
                &["collector".into()],
                &["workload".into()],
                &dora,
                Some(&runtime)
            )
            .await
            .unwrap()
        );
        // Act / Assert: changed frozen bytes must not produce a trusted snapshot.
        tokio::fs::write(path, b"{}").await.unwrap();
        assert!(
            capture(&spec, root.path(), &plan, &[], &[], &dora, Some(&runtime))
                .await
                .is_err()
        );
    }
    fn resolved_model(root: &str, ports: [u32; 2]) -> (Value, crate::dora::DoraApps, DoraRuntime) {
        let (dora, runtime) = crate::dora::fixture(root, [ports[0] as u16, ports[1] as u16]);
        let directory = format!("{root}/dora/runtime");
        let mut model = json!({"services":{"worker":{
            "image":"sha256:worker",
            "working_dir":directory,
            "command":["--state",format!("{directory}/state.json"),format!("--log={directory}/worker.log")],
            "entrypoint":[format!("{directory}/worker")],
            "healthcheck":{"test":["CMD", "test", "-s", format!("{directory}/state.json")]},
            "volumes":[{"type":"bind", "source":directory, "target":directory}]
        }}});
        for (name, service) in &dora.services {
            model["services"][name] = service.clone();
        }
        crate::preparation::model::configure_dora(
            &mut model,
            &dora,
            Some(&runtime),
            "owner".into(),
        )
        .unwrap();
        (model, dora, runtime)
    }

    #[test]
    fn generated_runtime_paths_and_automatic_ports_survive_independent_resolution() {
        // Arrange
        let (review, review_dora, review_runtime) = resolved_model("/review", [6000, 6001]);
        let (execution, execution_dora, execution_runtime) =
            resolved_model("/execution", [7000, 7001]);

        // Act
        let normalize = |model, root: &str, dora: &crate::dora::DoraApps, runtime: &DoraRuntime| {
            manifest(
                model,
                Path::new(root),
                &["worker".into()],
                dora,
                Some(&DoraRuntime {
                    automatic_coordinator_port: true,
                    automatic_daemon_port: true,
                    ..runtime.clone()
                }),
            )
        };
        let inspected = normalize(review, "/review", &review_dora, &review_runtime);
        let prepared = normalize(
            execution.clone(),
            "/execution",
            &execution_dora,
            &execution_runtime,
        );

        // Assert
        assert_eq!(inspected, prepared);

        // Act / Assert: user values that happen to contain an assigned port stay exact.
        for (field, value) in [
            ("command", json!(["--port", "7000"])),
            ("environment", json!({"CUSTOM":"7001"})),
            ("working_dir", json!("/different-directory")),
            ("working_dir", json!("/execution/dora/runtime-unrelated")),
            (
                "healthcheck",
                json!({"test":["CMD", "test", "-s", "/different.ready"]}),
            ),
        ] {
            let mut changed = execution.clone();
            changed["services"]["worker"][field] = value;
            assert_ne!(
                inspected,
                normalize(changed, "/execution", &execution_dora, &execution_runtime)
            );
        }
    }

    #[test]
    fn explicitly_configured_dora_ports_remain_part_of_configuration_identity() {
        // Arrange
        let (initial, initial_dora, initial_runtime) = resolved_model("/review", [6000, 6001]);
        let normalize = |model,
                         root: &str,
                         dora: &crate::dora::DoraApps,
                         runtime: &DoraRuntime,
                         automatic: [bool; 2]| {
            manifest(
                model,
                Path::new(root),
                &["worker".into()],
                dora,
                Some(&DoraRuntime {
                    automatic_coordinator_port: automatic[0],
                    automatic_daemon_port: automatic[1],
                    ..runtime.clone()
                }),
            )
        };

        // Act / Assert: each explicitly configured port remains exact independently.
        for automatic in [[false, true], [true, false], [false, false]] {
            let inspected = normalize(
                initial.clone(),
                "/review",
                &initial_dora,
                &initial_runtime,
                automatic,
            );
            for ports in [[7000, 6001], [6000, 7001]] {
                let (changed, changed_dora, changed_runtime) = resolved_model("/execution", ports);
                let prepared = normalize(
                    changed,
                    "/execution",
                    &changed_dora,
                    &changed_runtime,
                    automatic,
                );
                assert_eq!(
                    inspected == prepared,
                    if ports[0] != 6000 {
                        automatic[0]
                    } else {
                        automatic[1]
                    }
                );
            }
        }
    }

    async fn capture_invocations(
        root: &Path,
        model: &Value,
        dora: &crate::dora::DoraApps,
        runtime: Option<&DoraRuntime>,
    ) -> ConfigurationSnapshot {
        let path = root.join("experiment.json");
        let bytes = serde_json::to_vec(model).unwrap();
        tokio::fs::write(&path, &bytes).await.unwrap();
        capture(
            &PrepareInput {
                run_plan: workload_lifecycle::launch::RunPlan {
                    id: "a".repeat(32),
                    ..Default::default()
                },
                dora_apps: vec![],
                system_apps: vec![],
                extra_apps: vec![],
                filesystem: None,
                mcap_writers: Default::default(),
            },
            root,
            &ResolvedComposeProject {
                manifest: path.to_string_lossy().into_owned(),
                sha256: digest(&bytes),
                ..Default::default()
            },
            &["worker".into()],
            &[],
            dora,
            runtime,
        )
        .await
        .unwrap()
    }

    #[test]
    fn invocation_defaults_and_empty_overrides_follow_compose() {
        // Arrange
        let inherited = invocation(&json!({})).unwrap();
        let cleared = invocation(&json!({"command":[],"entrypoint":[]})).unwrap();
        // Act / Assert
        assert_eq!(
            inherited,
            invocation(&json!({"command":null,"entrypoint":null,"stop_signal":null})).unwrap()
        );
        assert_eq!(
            cleared,
            invocation(&json!({"command":"","entrypoint":""})).unwrap()
        );
        assert_ne!(inherited, cleared);
    }

    async fn saved_setup(root: &Path) -> SetupRecord {
        SetupRecord::decode(
            tokio::fs::read(root.join(SETUP_RECORD))
                .await
                .unwrap()
                .as_slice(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn invocation_copy_preserves_configured_changes_and_canonicalizes_generated_values() {
        // Arrange
        let roots = [tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap()];
        let mut records = Vec::new();
        for (root, ports) in roots.iter().zip([[6000, 6001], [7000, 7001]]) {
            let (model, dora, runtime) = resolved_model(root.path().to_str().unwrap(), ports);

            // Act
            capture_invocations(
                root.path(),
                &model,
                &dora,
                Some(&DoraRuntime {
                    automatic_coordinator_port: true,
                    automatic_daemon_port: true,
                    ..runtime
                }),
            )
            .await;
            records.push(saved_setup(root.path()).await);
        }

        // Assert
        let normalized = |record: &SetupRecord| {
            record
                .services
                .iter()
                .map(|(name, service)| (name.clone(), service.normalized.clone()))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        assert_eq!(normalized(&records[0]), normalized(&records[1]));
        assert!(format!("{:?}", normalized(&records[0])).contains("@dora/coordinator-port"));
        assert!(
            format!("{:?}", normalized(&records[0])).contains("@artifacts/dora/runtime/worker")
        );

        // Act / Assert: explicitly configured Dora ports and ordinary app arguments stay exact.
        let root = &roots[1];
        let (mut model, dora, runtime) =
            resolved_model(root.path().to_str().unwrap(), [7000, 7001]);
        model["services"]["worker"]["command"] = json!(["--port", "7000"]);
        model["services"]["worker"]["entrypoint"] = json!(["/changed-worker"]);
        model["services"]["worker"]["stop_signal"] = json!("SIGINT");
        capture_invocations(root.path(), &model, &dora, Some(&runtime)).await;
        let configured = saved_setup(root.path()).await;
        assert_ne!(normalized(&records[0]), normalized(&configured));
        assert!(!format!("{:?}", normalized(&configured)).contains("@dora/coordinator-port"));
        assert_eq!(configured.services["worker"].normalized, Some(invocation(&json!({
            "command":["--port","7000"], "entrypoint":["/changed-worker"], "stop_signal":"SIGINT"
        })).unwrap()));
    }

    #[tokio::test]
    async fn setup_record_is_private_and_preserves_typed_settings_and_presence() {
        use std::os::unix::fs::PermissionsExt;
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let model = json!({"services": {
            "worker": {"image":"app", "command":null, "entrypoint":[], "environment":{"TOKEN":"secret"}, "volumes":[{"type":"bind","source":"/input","target":"/config"}]},
            "empty": {"command":"", "stop_signal":"SIGTERM"},
            "default": {"image":"other"}
        }, "configs":{"settings":{"file":format!("/inputs/{}-444", "c".repeat(32))}},
           "secrets":{"credential":{"file":format!("/inputs/{}-400", "d".repeat(32))}}});

        // Act
        capture_invocations(root.path(), &model, &Default::default(), None).await;
        let record = saved_setup(root.path()).await;

        // Assert
        assert_eq!(record.version, SETUP_VERSION);
        assert!(record.complete);
        assert_eq!(
            record.configs["settings"],
            format!("/inputs/{}-444", "c".repeat(32))
        );
        assert_eq!(
            record.secrets["credential"],
            format!("/inputs/{}-400", "d".repeat(32))
        );
        assert_eq!(record.services.len(), 3);
        let worker = &record.services["worker"];
        assert_eq!(worker.environment["TOKEN"], "secret");
        assert_eq!(
            worker.binds,
            [SetupBind {
                source: "/input".into(),
                target: "/config".into()
            }]
        );
        let captured = worker.captured.as_ref().unwrap();
        assert_eq!(captured.command, None);
        assert_eq!(
            captured.entrypoint.as_ref().unwrap().value,
            Some(invocation_value::Value::Arguments(
                InvocationArguments::default()
            ))
        );
        let empty = record.services["empty"].captured.as_ref().unwrap();
        assert_eq!(
            empty.command.as_ref().unwrap().value,
            Some(invocation_value::Value::Arguments(
                InvocationArguments::default()
            ))
        );
        assert_eq!(empty.entrypoint, None);
        assert_eq!(empty.stop_signal, "SIGTERM");
        assert_eq!(
            record.services["default"].captured,
            Some(Invocation::default())
        );
        assert_eq!(
            std::fs::metadata(root.path().join(SETUP_RECORD))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[tokio::test]
    async fn invocation_save_failure_does_not_fail_resolution_or_publish_partial_bytes() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        tokio::fs::create_dir(root.path().join(SETUP_RECORD))
            .await
            .unwrap();

        // Act
        let snapshot = capture_invocations(
            root.path(),
            &json!({"services":{"worker":{"command":["worker"]}}}),
            &Default::default(),
            None,
        )
        .await;

        // Assert
        assert!(!snapshot.manifest_sha256.is_empty());
        assert!(root.path().join(SETUP_RECORD).is_dir());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[tokio::test]
    async fn oversized_setup_evidence_leaves_execution_available_without_partial_publication() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let model = json!({"services":{"worker":{"command":["x".repeat(MAX_SETUP_BYTES)]}}});
        // Act
        let snapshot = capture_invocations(root.path(), &model, &Default::default(), None).await;
        // Assert
        assert!(!snapshot.manifest_sha256.is_empty());
        assert!(!root.path().join(SETUP_RECORD).exists());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
