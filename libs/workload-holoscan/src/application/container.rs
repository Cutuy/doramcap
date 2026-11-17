//! Actual Engine evidence at the native application boundary.
use bollard::models::ContainerInspectResponse;
use docker_engine::{ARTIFACTS_LABEL, CONTROLLER_LABEL, RUN_LABEL};
use host_filesystem::{
    filesystem_proto::{AppScope, DirectoryMount},
    overlay::OverlaySet,
};
use std::path::Path;
use workload_lifecycle::launch::RunPlan;

/// Ownership is established before retaining an ID for stop/removal; other checks may still fail.
pub(super) fn verify_ownership(
    info: &ContainerInspectResponse,
    run: &RunPlan,
) -> Result<(), String> {
    let controller =
        compose_apps::compose::adapter::repository_scope(Path::new(&run.repository_root));
    let expected = [
        (
            CONTROLLER_LABEL,
            controller.to_str().ok_or("controller path must be UTF-8")?,
        ),
        (RUN_LABEL, run.id.as_str()),
        (ARTIFACTS_LABEL, run.artifacts.as_str()),
    ];
    let labels = info
        .config
        .as_ref()
        .and_then(|config| config.labels.as_ref())
        .ok_or("native application container has no ownership labels")?;
    for (key, expected) in expected {
        if labels.get(key).map(String::as_str) != Some(expected) {
            return Err(format!(
                "native application container ownership differs at {key}"
            ));
        }
    }
    if info.id.as_deref().is_none_or(str::is_empty) {
        return Err("native application container identity is unavailable".into());
    }
    Ok(())
}

/// Verify actual image, placement and every controlled writable source before readiness.
/// Project destination stays an observed native fact; unrelated native mounts remain observable.
pub(super) async fn validate(
    info: &ContainerInspectResponse,
    run: &RunPlan,
    image: &str,
    cgroup_parent: &str,
    views: &OverlaySet,
    plan: &crate::HoloscanPlan,
) -> Result<Vec<DirectoryMount>, String> {
    verify_ownership(info, run)?;
    if image.is_empty() || info.image.as_deref() != Some(image) {
        return Err("native application image differs from the recorded Build image".into());
    }
    if info
        .host_config
        .as_ref()
        .and_then(|config| config.cgroup_parent.as_deref())
        != Some(cgroup_parent)
    {
        return Err("native application cgroup parent differs from Console placement".into());
    }
    let native = info
        .mounts
        .as_ref()
        .ok_or("native application mount evidence is unavailable")?;
    let mappings = super::views::mappings(views, run, plan)?;
    let consumer = &mappings[0].consumer;
    let mut observed = Vec::new();
    for mount in native {
        if mount.typ.as_ref().is_some_and(|kind| kind == "bind") {
            observed.push(DirectoryMount {
                consumer: consumer.clone(),
                source: mount
                    .source
                    .clone()
                    .ok_or("native bind source is unavailable")?,
                target: mount
                    .destination
                    .clone()
                    .ok_or("native bind destination is unavailable")?,
                read_only: !mount.rw.ok_or("native bind access is unavailable")?,
                app_scope: AppScope::User as i32,
            });
        }
    }
    for (index, expected) in mappings.iter().enumerate() {
        let source = tokio::fs::canonicalize(&expected.effective)
            .await
            .map_err(error)?;
        let mut matched = false;
        for actual in &observed {
            if actual.read_only {
                continue;
            }
            if index != 0 && actual.target != expected.target {
                continue;
            }
            if tokio::fs::canonicalize(&actual.source).await.ok().as_ref() == Some(&source) {
                matched = true;
                break;
            }
        }
        if !matched {
            return Err(format!(
                "native application does not mount controlled writable source {} at its required destination",
                expected.original.display()
            ));
        }
    }
    Ok(observed)
}

fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HoloscanPlan, Selection};
    use host_filesystem::overlay::{MountMapping, OverlayPolicy};
    use serde_json::json;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct Fixture {
        directory: PathBuf,
        run: RunPlan,
        plan: HoloscanPlan,
        views: OverlaySet,
        info: ContainerInspectResponse,
    }
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let directory = std::env::temp_dir().join(format!(
                "holoscan-container-check-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let project = directory.join("native-project");
            let extra = directory.join("named-data");
            let project_view = directory.join("views/native-project");
            let extra_view = directory.join("views/named-data");
            for path in [&project, &extra, &project_view, &extra_view] {
                fs::create_dir_all(path).unwrap();
            }
            let run = RunPlan {
                id: "attempt".into(),
                repository_root: project.to_str().unwrap().into(),
                artifacts: directory.join("artifacts").to_str().unwrap().into(),
                ..Default::default()
            };
            let plan = HoloscanPlan {
                selection: Some(Selection {
                    application: "sample".into(),
                    ..Default::default()
                }),
                extra_data_folders: vec![extra.to_str().unwrap().into()],
                ..Default::default()
            };
            let mapping = |consumer: &str, target: &str, original: PathBuf, effective: PathBuf| {
                MountMapping {
                    consumer: consumer.into(),
                    target: target.into(),
                    original,
                    effective,
                    read_only: false,
                    reason: "OverlayFS".into(),
                }
            };
            let views = OverlaySet {
                directory: directory.join("records"),
                workspace: directory.join("workspace"),
                trees: vec![],
                policy: OverlayPolicy::default(),
                mappings: vec![
                    mapping("holoscan:sample", "", project.clone(), project_view.clone()),
                    mapping(
                        "compose-key/service",
                        "/system/source",
                        directory.join("foreign"),
                        directory.join("foreign-view"),
                    ),
                    mapping(
                        "holoscan:sample",
                        "/workspace/volumes/named-data",
                        extra.clone(),
                        extra_view.clone(),
                    ),
                ],
            };
            let controller = compose_apps::compose::adapter::repository_scope(&project);
            let info = serde_json::from_value(json!({
                "Id":"owned", "Image":"sha256:built",
                "Config":{"WorkingDir":"/independent/native/workdir", "Labels": {
                    "io.doramcap.controller":controller.to_str().unwrap(),
                    "io.doramcap.run":"attempt", "io.doramcap.artifacts":run.artifacts,
                }},
                "HostConfig":{"CgroupParent":"/existing/app"},
                "Mounts":[
                    {"Type":"bind","Source":project_view,"Destination":"/native/project","RW":true},
                    {"Type":"bind","Source":extra_view,"Destination":"/workspace/volumes/named-data","RW":true}
                ]
            })).unwrap();
            Self {
                directory,
                run,
                plan,
                views,
                info,
            }
        }
        async fn validate(&self) -> Result<Vec<DirectoryMount>, String> {
            validate(
                &self.info,
                &self.run,
                "sha256:built",
                "/existing/app",
                &self.views,
                &self.plan,
            )
            .await
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).unwrap();
        }
    }

    #[tokio::test]
    async fn observed_project_destination_is_native_and_other_backend_mappings_are_independent() {
        // Arrange
        let fixture = Fixture::new();
        // Act
        let observed = fixture.validate().await.unwrap();
        // Assert
        assert_eq!(observed.len(), 2);
        assert_eq!(observed[0].target, "/native/project");
        assert!(
            observed
                .iter()
                .all(|mount| mount.consumer == "holoscan:sample")
        );
    }

    #[tokio::test]
    async fn ownership_image_placement_and_controlled_mount_failures_are_rejected() {
        for mutation in [
            "ownership",
            "image",
            "placement",
            "readonly",
            "source",
            "extra-target",
        ] {
            // Arrange
            let mut fixture = Fixture::new();
            let mut native = serde_json::to_value(&fixture.info).unwrap();
            match mutation {
                "ownership" => {
                    native["Config"]["Labels"]["io.doramcap.run"] = json!("foreign");
                }
                "image" => {
                    native["Image"] = json!("sha256:other");
                }
                "placement" => {
                    native["HostConfig"]["CgroupParent"] = json!("/different/app");
                }
                "readonly" => {
                    native["Mounts"][0]["RW"] = json!(false);
                }
                "source" => {
                    native["Mounts"][0]["Source"] = json!(fixture.directory);
                }
                "extra-target" => {
                    native["Mounts"][1]["Destination"] = json!("/different/data");
                }
                _ => unreachable!(),
            }
            fixture.info = serde_json::from_value(native).unwrap();
            // Act / Assert
            assert!(fixture.validate().await.is_err(), "{mutation}");
        }
    }
}
