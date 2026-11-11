//! Docker storage discovery and stopped-container layer observations.
use crate::{
    Engine, Result,
    docker_proto::{OwnedContainer, VolumeMount},
    error::{absent, engine, invalid},
};
use host_filesystem::{
    filesystem::{CategoryChanges, ChangeWriter, FilesystemBaseline, HOST_SHM_SOURCE},
    filesystem_proto::{
        ChangeKind, FileChange, MetadataSource, SourceObservation, StorageCategory,
    },
};
use std::path::{Path, PathBuf};

async fn volume_path(adapter: &Engine, name: &str) -> Result<Option<PathBuf>> {
    match adapter.docker.inspect_volume(name).await {
        Ok(v)
            if v.driver == "local"
                && v.options.is_empty()
                && Path::new(&v.mountpoint).is_absolute() =>
        {
            Ok(Some(PathBuf::from(v.mountpoint)))
        }
        Ok(v) if v.driver == "local" => {
            if v.options.get("o").is_some_and(|options| {
                options
                    .split(',')
                    .any(|option| matches!(option, "bind" | "rbind"))
            }) && let Some(source) = v.options.get("device")
                && host_filesystem::is_host_shm_source(Path::new(source))?
            {
                return Ok(Some(PathBuf::from(source)));
            }
            Err(invalid(
                "volume driver/options do not support direct local metadata inspection",
            ))
        }
        Ok(_) => Err(invalid(
            "volume driver/options do not support direct local metadata inspection",
        )),
        Err(e) if absent(&e) => Ok(None),
        Err(e) => Err(engine("inspect volume metadata", e)),
    }
}

/// Resolve native volume metadata sources before the shared baseline step.
/// Unsupported drivers retain their diagnostic as incomplete coverage.
pub async fn volume_sources(
    adapter: &Engine,
    mounts: Vec<VolumeMount>,
) -> Result<Vec<MetadataSource>> {
    let mut sources = Vec::new();
    for mount in mounts {
        let (path, reason) = match volume_path(adapter, &mount.name).await {
            Ok(Some(path)) if host_filesystem::is_host_shm_source(&path)? => {
                (Some(path), Some(HOST_SHM_SOURCE.into()))
            }
            Ok(path) => (path, None),
            Err(error) => (None, Some(error.to_string())),
        };
        sources.push(MetadataSource {
            category: host_filesystem::filesystem_proto::StorageCategory::NamedVolume as i32,
            identity: mount.name,
            path: path
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            reason: reason.unwrap_or_default(),
            read_only: mount.read_only,
            consumer: format!("{}:{}", mount.consumer, mount.target),
        });
    }
    Ok(sources)
}
/// Re-resolve complete named-volume baselines after writers stop.
/// Missing and unsupported sources remain explicit observations.
pub async fn volume_observations(
    adapter: &Engine,
    baseline: &FilesystemBaseline,
) -> Result<Vec<SourceObservation>> {
    let mut observations = Vec::new();
    for identity in baseline
        .pending_sources(StorageCategory::NamedVolume)
        .await?
    {
        let (path, reason) = match volume_path(adapter, &identity).await {
            Ok(Some(path)) if host_filesystem::is_host_shm_source(&path)? => {
                (Some(path), Some(HOST_SHM_SOURCE.into()))
            }
            Ok(path) => (path, None),
            Err(error) => (None, Some(error.to_string())),
        };
        observations.push(SourceObservation {
            identity,
            path: path
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            reason: reason.unwrap_or_default(),
        });
    }
    Ok(observations)
}
/// Verify all supplied owned containers are stopped, then stream their Engine diffs.
/// Errors preserve runtime resources; the caller has already verified ownership.
pub async fn container_changes(
    adapter: &Engine,
    containers: &[OwnedContainer],
    directory: &Path,
) -> Result<CategoryChanges> {
    // The backend verifies ownership; collection verifies stopped writers before diffing.
    for container in containers {
        let info = adapter
            .docker
            .inspect_container(&container.id, None)
            .await
            .map_err(|e| engine(format!("inspect {}", container.id), e))?;
        let state = info
            .state
            .ok_or_else(|| invalid("Engine omitted container state"))?;
        if state.pid.unwrap_or(0) != 0
            || !matches!(
                state.status.map(|s| s.to_string()).as_deref(),
                Some("exited" | "created")
            )
        {
            return Err(invalid("filesystem collection requires stopped writers"));
        }
    }
    let mut output = ChangeWriter::create(
        directory,
        StorageCategory::ContainerLayer,
        "container-layers.pb",
    )
    .await?;
    for container in containers {
        let changes = adapter
            .docker
            .container_changes(&container.id)
            .await
            .map_err(|e| engine("container filesystem diff", e))?
            // Moby ChangesDirs returns a nil slice when no paths changed;
            // the successful HTTP response encodes it as JSON null. Engine
            // and decoding failures were propagated above, not converted.
            // github.com/moby/moby/blob/v28.0.0/pkg/archive/changes.go
            .unwrap_or_default();
        for change in changes {
            let kind = match change.kind {
                bollard::models::ChangeType::_0 => ChangeKind::Modified,
                bollard::models::ChangeType::_1 => ChangeKind::Added,
                bollard::models::ChangeType::_2 => ChangeKind::Deleted,
            };
            output
                .emit(FileChange {
                    category: StorageCategory::ContainerLayer as i32,
                    source: container.id.clone(),
                    consumers: vec![container.consumer.clone()],
                    path: change.path.into_bytes(),
                    kind: kind as i32,
                    before: None,
                    after: None,
                })
                .await?;
        }
    }

    output.finish().await.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use host_filesystem::filesystem;
    use host_filesystem::{
        filesystem::{BaselineInput, CollectInput, FileChanges},
        filesystem_proto::DirectoryMount as BindMount,
    };
    async fn capture(adapter: &Engine, input: BaselineInput) -> Result<FilesystemBaseline> {
        let _ = adapter;
        Ok(filesystem::capture(input).await?)
    }
    async fn collect(
        adapter: &Engine,
        mut input: CollectInput,
        containers: Vec<OwnedContainer>,
    ) -> Result<filesystem::FilesystemChanges> {
        input.container_layers =
            Some(container_changes(adapter, &containers, &input.baseline.directory).await?);
        input.observations = volume_observations(adapter, &input.baseline).await?;
        Ok(filesystem::collect(input).await?)
    }
    #[tokio::test]
    async fn mount_inputs_preserve_change_record_consumers() {
        use crate::tests::test_engine;
        use serde_json::json;
        // Arrange: read-only and writable aliases share the same baseline.
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let artifacts = root.path().join("artifacts");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&artifacts).unwrap();
        let (adapter, server) = test_engine(
            root.path(),
            vec![
                (
                    "/containers/owned/json",
                    json!({"State":{"Pid":0,"Status":"exited"}}),
                ),
                (
                    "/containers/owned/changes",
                    json!([{"Path":"/result","Kind":1}]),
                ),
            ],
        );
        let bind = |consumer: &str, target: &str, read_only| BindMount {
            consumer: consumer.into(),
            source: source.to_string_lossy().into(),
            target: target.into(),
            read_only,
        };
        let baseline = capture(
            &adapter,
            BaselineInput {
                artifacts,
                bind_mounts: vec![
                    bind("dc-project/reader", "/in", true),
                    bind("dc-project/writer", "/out", false),
                ],
                metadata_sources: vec![],
            },
        )
        .await
        .unwrap();
        std::fs::write(source.join("result"), b"saved").unwrap();

        // Act
        let changes = collect(
            &adapter,
            CollectInput {
                host_views: vec![],
                baseline,
                observations: vec![],
                container_layers: None,
            },
            vec![OwnedContainer {
                consumer: "dc-project/app".into(),
                id: "owned".into(),
            }],
        )
        .await
        .unwrap();

        // Assert: successful collection publishes the same summaries before finalization.
        use prost::Message;
        let reference = changes.reference();
        let checkpoint = host_filesystem::filesystem_proto::FileChangeReport::decode(
            tokio::fs::read(root.path().join("artifacts").join(&reference.checkpoint))
                .await
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        assert_eq!(
            checkpoint
                .categories
                .iter()
                .map(|c| c.summary.clone().unwrap())
                .collect::<Vec<_>>(),
            reference.categories
        );
        assert!(
            !root
                .path()
                .join("artifacts/filesystem/report.pending.pb")
                .exists()
        );
        let mut host = FileChanges::open(changes.host_binds.manifest)
            .await
            .unwrap();
        let mut result = None;
        while let Some(change) = host.next_change().await.unwrap() {
            if change.path == b"result" {
                result = Some(change);
            }
        }
        let result = result.unwrap();
        assert_eq!(result.kind(), ChangeKind::Added);
        assert_eq!(
            result.consumers,
            ["dc-project/reader:/in", "dc-project/writer:/out"]
        );
        let mut layers = FileChanges::open(changes.container_layers.manifest)
            .await
            .unwrap();
        let result = layers.next_change().await.unwrap().unwrap();
        assert_eq!(result.consumers, ["dc-project/app"]);
        assert_eq!(result.path, b"/result");
        assert!(layers.next_change().await.unwrap().is_none());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn live_container_rejects_collection_before_requesting_diff() {
        use crate::tests::test_engine;
        use serde_json::json;
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let (adapter, server) = test_engine(
            root.path(),
            vec![(
                "/containers/owned/json",
                json!({"State":{"Pid":123,"Status":"running"}}),
            )],
        );
        let baseline = capture(
            &adapter,
            BaselineInput {
                artifacts: root.path().into(),
                bind_mounts: vec![],
                metadata_sources: vec![],
            },
        )
        .await
        .unwrap();

        // Act
        let error = collect(
            &adapter,
            CollectInput {
                host_views: vec![],
                baseline,
                observations: vec![],
                container_layers: None,
            },
            vec![OwnedContainer {
                consumer: "dc-project/app".into(),
                id: "owned".into(),
            }],
        )
        .await
        .unwrap_err();

        // Assert
        assert_eq!(
            error.to_string(),
            "filesystem collection requires stopped writers"
        );
        server.await.unwrap();
    }
}
