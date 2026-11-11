//! Compare the effective Compose configuration across independently prepared runs.
use super::*;
use crate::compose::adapter::{ARTIFACTS_LABEL, RUN_LABEL};
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
            if runtime || systems.contains(name) || name == super::SHM_SERVICE {
                service
                    .as_object_mut()
                    .unwrap()
                    .insert("cgroup_parent".into(), "@system-apps/cgroup-parent".into());
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

/// Largest resolved execution manifest captured into a configuration snapshot.
const MAX_MANIFEST_BYTES: u64 = 64 * 1024 * 1024;

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
    let model = manifest(
        serde_json::from_slice(&bytes)?,
        artifacts,
        system_services,
        dora,
        runtime,
    );
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
        let first = json!({"services":{"collector":{"cgroup_parent":"/first"}, "workload":{"cgroup_parent":"/user"}}});
        let second = json!({"services":{"collector":{"cgroup_parent":"/second"}, "workload":{"cgroup_parent":"/user"}}});
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
        assert_eq!(normalize(first.clone()), normalize(second));
        // Act / Assert: native user placement remains a meaningful reviewed setting.
        let mut changed = first.clone();
        changed["services"]["workload"]["cgroup_parent"] = "/different-user".into();
        assert_ne!(normalize(first), normalize(changed));
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
        // serde_json uses sorted keys alone and insertion order in workspace builds.
        assert!(
            [
                "805c5fede8042298f328d2b0d1df0e402074f91db0a723c59b9d492b85fa1701",
                "e56ab24c5c33ae51c0039d6e341d95b914706dedfcea4ae5fffc5fba87092c42",
            ]
            .contains(&original.manifest_sha256.as_str()),
            "unexpected manifest fingerprint: {}",
            original.manifest_sha256
        );
        assert_eq!(
            original.settings_sha256,
            "084a4357a51e35bf319c39e8e6a7691e1810bf09522274a448fe51f13d5b2d17"
        );
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
                Some(&runtime),
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
                Some(&runtime),
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
                Some(&runtime),
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
                Some(&runtime),
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
        let ready = format!("{directory}/worker.ready");
        let mut model = json!({"services":{"worker":{
            "image":"sha256:worker",
            "working_dir":directory,
            "command":["--state",format!("{directory}/state.json"),format!("--log={directory}/worker.log")],
            "entrypoint":[format!("{directory}/worker")],
            "healthcheck":{"test":["CMD", "test", "-s", ready]},
            "volumes":[{"type":"bind", "source":directory, "target":directory}],
            "environment":{dora_app::environment::READY_FILE:ready}
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
        // Pin both existing serde_json key orders without changing runtime hashing.
        let manifest_sha256 = digest(&serde_json::to_vec(&inspected).unwrap());
        assert!(
            [
                "84376c6ae73d93d22c9f79160fe011c54b6a5b42831d59cf0254ba4a1c8f9777",
                "2dc3fe4e1ccaa7e6fd1a846cbb7f5b51790fb080ff22130ccaf39b0d12a0f31e",
            ]
            .contains(&manifest_sha256.as_str()),
            "unexpected manifest fingerprint: {manifest_sha256}"
        );
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
}
