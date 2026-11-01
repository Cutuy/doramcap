//! Compare the effective Compose configuration across independently prepared runs.
use super::*;
use crate::compose::adapter::{ARTIFACTS_LABEL, RUN_LABEL};

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
    dora: &crate::workflows::dora::DoraRuntime,
    automatic_ports: [bool; 2],
) -> Value {
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
    runtime_paths(
        &mut model,
        &dora.runtime_directory,
        &relative(&dora.runtime_directory, artifacts),
    );
    if let Some(services) = model.get_mut("services").and_then(Value::as_object_mut) {
        for (name, service) in services {
            let runtime = [
                crate::workflows::shared_dora::COORDINATOR,
                crate::workflows::shared_dora::DAEMON,
            ]
            .contains(&name.as_str());
            if let Some(mounts) = service.get_mut("volumes").and_then(Value::as_array_mut) {
                for mount in mounts {
                    let frozen = mount["source"].as_str().is_some_and(|source| {
                        Path::new(source).starts_with(artifacts.join("inputs"))
                    });
                    let attachment = mount["target"] == dora.zenoh_overlay
                        || mount["source"] == dora.zenoh_overlay;
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
                    if value == &dora.zenoh_overlay {
                        path_value(value, artifacts);
                    }
                }
            }
            if runtime
                && let Some(command) = service.get_mut("command").and_then(Value::as_array_mut)
            {
                for index in 1..command.len() {
                    let port = match command[index - 1].as_str() {
                        Some("--port" | "--coordinator-port") if automatic_ports[0] => {
                            Some("@dora/coordinator-port")
                        }
                        Some("--local-listen-port") if automatic_ports[1] => {
                            Some("@dora/daemon-port")
                        }
                        _ => None,
                    };
                    if let Some(port) = port {
                        command[index] = port.into();
                    } else {
                        path_value(&mut command[index], artifacts);
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

pub(super) async fn capture(
    spec: &ComposeRun,
    artifacts: &Path,
    plan: &ResolvedComposeProject,
    system_services: &[String],
    user_services: &[String],
    dora: &crate::workflows::dora::DoraRuntime,
) -> Result<ConfigurationSnapshot> {
    let bytes = crate::evidence::read_regular(Path::new(&plan.manifest), 64 * 1024 * 1024).await?;
    if digest(&bytes) != plan.sha256 {
        return Err(invalid("execution manifest changed during resolution"));
    }
    let model = manifest(
        serde_json::from_slice(&bytes)?,
        artifacts,
        system_services,
        dora,
        [spec.dora.coordinator_port == 0, spec.dora.daemon_port == 0],
    );
    let recordings: Vec<_> = spec
        .recordings
        .iter()
        .map(|recording| {
            (
                relative(&recording.stream.to_string_lossy(), artifacts),
                relative(&recording.recording.to_string_lossy(), artifacts),
            )
        })
        .collect();
    let settings = serde_json::to_vec(&(
        &spec.filesystem,
        spec.readiness_seconds,
        spec.stop_seconds,
        recordings,
    ))?;
    Ok(ConfigurationSnapshot {
        manifest_sha256: digest(&serde_json::to_vec(&model)?),
        system_services: system_services.to_vec(),
        user_services: user_services.to_vec(),
        settings_sha256: digest(&settings),
        graph_sha256: dora.graph_sha256.clone(),
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
    fn generated_paths_compare_equal_but_commands_images_and_host_binds_remain_exact() {
        // Arrange
        let dora = Default::default();
        let initial = manifest(
            model("/review"),
            Path::new("/review"),
            &["collector".into()],
            &dora,
            [false; 2],
        );
        // Act / Assert
        assert_eq!(
            initial,
            manifest(
                model("/execution"),
                Path::new("/execution"),
                &["collector".into()],
                &dora,
                [false; 2]
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
                    [false; 2]
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
                [false; 2]
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
        let mut spec = ComposeRun {
            filesystem: None,
            id: "compare".into(),
            artifacts: root.path().into(),
            repository_root: root.path().into(),
            projects: vec![],
            system_apps: vec![],
            dora: Default::default(),
            recordings: vec![],
            readiness_seconds: 60,
            stop_seconds: 15,
        };
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
            &spec.dora,
        )
        .await
        .unwrap();
        // Act / Assert: policy, stages and graph contents are compared independently.
        spec.stop_seconds += 1;
        assert_ne!(
            original,
            capture(
                &spec,
                root.path(),
                &plan,
                &["collector".into()],
                &["workload".into()],
                &spec.dora
            )
            .await
            .unwrap()
        );
        spec.stop_seconds -= 1;
        assert_ne!(
            original,
            capture(
                &spec,
                root.path(),
                &plan,
                &["workload".into()],
                &["collector".into()],
                &spec.dora
            )
            .await
            .unwrap()
        );
        spec.dora.graph_sha256 = "changed".into();
        assert_ne!(
            original,
            capture(
                &spec,
                root.path(),
                &plan,
                &["collector".into()],
                &["workload".into()],
                &spec.dora
            )
            .await
            .unwrap()
        );
        spec.dora.graph_sha256.clear();
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
                &spec.dora
            )
            .await
            .unwrap()
        );
        // Act / Assert: changed frozen bytes must not produce a trusted snapshot.
        tokio::fs::write(path, b"{}").await.unwrap();
        assert!(
            capture(&spec, root.path(), &plan, &[], &[], &spec.dora)
                .await
                .is_err()
        );
    }
    fn resolved_model(root: &str, ports: [u32; 2]) -> (Value, crate::workflows::dora::DoraRuntime) {
        let dora = crate::workflows::dora::DoraRuntime {
            image: "sha256:dora".into(),
            coordinator_port: ports[0],
            daemon_port: ports[1],
            runtime_directory: format!("{root}/dora/runtime"),
            zenoh_overlay: format!("{root}/dora/zenoh.json5"),
            participant_uid: Some(1000),
            participant_gid: Some(1000),
            ..Default::default()
        };
        let directory = &dora.runtime_directory;
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
        crate::workflows::recorded_compose::preparation::model::configure_dora(
            &mut model,
            &dora,
            "owner".into(),
        )
        .unwrap();
        (model, dora)
    }

    #[test]
    fn generated_runtime_paths_and_automatic_ports_survive_independent_resolution() {
        // Arrange
        let (review, review_dora) = resolved_model("/review", [6000, 6001]);
        let (execution, execution_dora) = resolved_model("/execution", [7000, 7001]);

        // Act
        let normalize = |model, root: &str, dora: &crate::workflows::dora::DoraRuntime| {
            manifest(model, Path::new(root), &["worker".into()], dora, [true; 2])
        };
        let inspected = normalize(review, "/review", &review_dora);
        let prepared = normalize(execution.clone(), "/execution", &execution_dora);

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
            assert_ne!(inspected, normalize(changed, "/execution", &execution_dora));
        }
    }

    #[test]
    fn explicitly_configured_dora_ports_remain_part_of_configuration_identity() {
        // Arrange
        let (initial, initial_dora) = resolved_model("/review", [6000, 6001]);
        let normalize =
            |model, root: &str, dora: &crate::workflows::dora::DoraRuntime, automatic| {
                manifest(model, Path::new(root), &["worker".into()], dora, automatic)
            };

        // Act / Assert: each explicitly configured port remains exact independently.
        for automatic in [[false, true], [true, false], [false, false]] {
            let inspected = normalize(initial.clone(), "/review", &initial_dora, automatic);
            for ports in [[7000, 6001], [6000, 7001]] {
                let (changed, changed_dora) = resolved_model("/execution", ports);
                let prepared = normalize(changed, "/execution", &changed_dora, automatic);
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
