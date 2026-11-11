//! Translate pinned Compose manifests to the Docker filesystem inputs.
use super::{ResolvedBind, ResolvedComposeProject, Result, adapter::digest, error::invalid};
use docker_engine::docker_proto::{BindMount, VolumeMount};
use serde_json::Value;

/// Read a pinned manifest and reject changes with the operation's diagnostic.
pub(crate) async fn model(project: &ResolvedComposeProject, changed: &str) -> Result<Value> {
    let bytes = tokio::fs::read(&project.manifest).await?;
    if digest(&bytes) != project.sha256 {
        return Err(invalid(changed));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

/// Preserve mount ordering and the project/service text used in change records.
pub(crate) fn binds(project: &str, model: &Value) -> Result<Vec<BindMount>> {
    Ok(ResolvedBind::from_manifest(model)?
        .into_iter()
        .map(|bind| BindMount {
            consumer: format!("{project}/{}", bind.service),
            source: bind.source,
            target: bind.target,
            read_only: bind.read_only,
            propagation: bind.propagation,
            selinux: bind.selinux,
        })
        .collect())
}

/// Resolve each named volume through the rendered Compose model.
/// Errors identify a volume without its resolved name.
pub(crate) fn volumes(project: &ResolvedComposeProject, model: &Value) -> Result<Vec<VolumeMount>> {
    let mut volumes = Vec::new();
    for (service, value) in model["services"].as_object().into_iter().flatten() {
        for mount in value["volumes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|m| m["type"] == "volume")
        {
            let raw = mount["source"].as_str().unwrap_or("");
            let name = model["volumes"][raw]["name"]
                .as_str()
                .ok_or_else(|| invalid("volume has no resolved name"))?;
            volumes.push(VolumeMount {
                consumer: format!("{}/{service}", project.name),
                name: name.into(),
                target: mount["target"].as_str().unwrap_or("?").into(),
                read_only: mount["read_only"] == true,
            });
        }
    }
    Ok(volumes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rendered_mounts_preserve_change_record_consumers_and_resolved_volume_names() {
        // Arrange
        let project = ResolvedComposeProject {
            key: "selection".into(),
            name: "dc-repository-experiment".into(),
            ..Default::default()
        };
        let model = json!({"services":{"app":{"volumes":[
            {"type":"bind","source":"/data","target":"/work","read_only":true,"bind":{"propagation":"rprivate"}},
            {"type":"volume","source":"cache","target":"/cache"}
        ]}},"volumes":{"cache":{"name":"external-cache"}}});

        // Act
        let binds = binds(&project.name, &model).unwrap();
        let volumes = volumes(&project, &model).unwrap();

        // Assert
        assert_eq!(binds[0].consumer, "dc-repository-experiment/app");
        assert_eq!(binds[0].source, "/data");
        assert_eq!(binds[0].target, "/work");
        assert!(binds[0].read_only);
        assert_eq!(binds[0].propagation, "rprivate");
        assert_eq!(volumes[0].consumer, binds[0].consumer);
        assert_eq!(volumes[0].name, "external-cache");
        assert_eq!(volumes[0].target, "/cache");
    }

    #[tokio::test]
    async fn changed_manifest_retains_operation_diagnostic() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let manifest = root.path().join("input.json");
        std::fs::write(&manifest, br#"{"services":{}}"#).unwrap();
        let project = ResolvedComposeProject {
            manifest: manifest.to_string_lossy().into(),
            sha256: "old".into(),
            ..Default::default()
        };

        // Act
        let error = model(&project, "filesystem baseline manifest changed")
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.to_string(), "filesystem baseline manifest changed");
    }
}
