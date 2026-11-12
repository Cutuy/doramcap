use super::super::reset::tests::test_engine_status;
use super::*;
use compose_apps::compose::{ComposeService, ResolvedComposeProject, Shutdown};
use host_filesystem::{
    filesystem::{self, FileChanges, SourcesInput},
    filesystem_proto::{MetadataSource, StorageCategory},
};
use orchestrator::{Event, State};
use serde_json::json;
use std::{cell::RefCell, collections::BTreeMap};

#[tokio::test]
async fn docker_observations_record_outcomes_and_gate_collection() {
    for failure in [None, Some("inspect"), Some("diff")] {
        // Arrange: a newly created volume has no pre-launch path or exclusion reason.
        let root = tempfile::tempdir().unwrap();
        let volume = root.path().join("volume");
        std::fs::create_dir(&volume).unwrap();
        std::fs::write(volume.join("seeded"), b"image contents").unwrap();
        let baseline = filesystem::prepare_sources(SourcesInput {
            overlays: true,
            excluded: vec![],
            artifacts: root.path().into(),
            bind_mounts: vec![],
            metadata_sources: vec![MetadataSource {
                category: StorageCategory::NamedVolume as i32,
                identity: "data".into(),
                reason: "not tracked: Docker named volume".into(),
                consumer: "project/app:/data".into(),
                ..Default::default()
            }],
        })
        .await
        .unwrap();
        let labels = json!({
            "io.doramcap.run":"run", "com.docker.compose.project":"project",
            "com.docker.compose.service":"app"
        });
        let responses = if failure == Some("inspect") {
            vec![(
                "/containers/json",
                500,
                json!({"message":"inspection failed"}),
            )]
        } else {
            vec![
                (
                    "/containers/json",
                    200,
                    json!([{"Id":"owned","Labels":labels}]),
                ),
                (
                    "/containers/owned/json",
                    200,
                    json!({
                        "Config":{"Labels":labels},
                        "State":{"Pid":0,"Status":"exited","ExitCode":0}
                    }),
                ),
                (
                    "/containers/owned/json",
                    200,
                    json!({"State":{"Pid":0,"Status":"exited"}}),
                ),
                (
                    "/containers/owned/changes",
                    if failure.is_some() { 500 } else { 200 },
                    if failure.is_some() {
                        json!({"message":"diff failed"})
                    } else {
                        json!([{"Path":"/layer-result","Kind":1}])
                    },
                ),
            ]
        };
        let (adapter, server) = test_engine_status(root.path(), responses).await;
        let diagnostics = Diagnostics::default();
        let events = Rc::new(RefCell::new(Vec::<Event>::new()));
        let sink = events.clone();
        let executor = crate::workflows::steps::executor(
            Rc::new(adapter),
            diagnostics.clone(),
            move |event: &Event| sink.borrow_mut().push(event.clone()),
            "run",
        );
        let project = ResolvedComposeProject {
            run: "run".into(),
            name: "project".into(),
            services: BTreeMap::from([(
                "app".into(),
                ComposeService {
                    completed: true,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };

        // Act: the enclosing finalizer retains diagnostics and returns its cleanup gate.
        let finalized = executor
            .run::<FinalizeRun>(FinalizeInput {
                id: "run".into(),
                artifacts: root.path().into(),
                sources: baseline,
                overlays: None,
                shutdown: Shutdown {
                    stopped: true,
                    ..Default::default()
                },
                project,
            })
            .await
            .unwrap()
            .into_value();

        // Assert: Docker failure has its own terminal event and never starts collection.
        server.await.unwrap();
        let events = events.borrow().clone();
        let observation = events
            .iter()
            .find(|event| {
                event.invocation.step == ObserveDockerFilesystem::ID.unwrap()
                    && event.state != State::Started
            })
            .unwrap();
        let finalizer = events
            .iter()
            .find(|event| {
                event.invocation.step == FinalizeRun::ID.unwrap() && event.state == State::Succeeded
            })
            .unwrap();
        assert_eq!(observation.invocation.parent, Some(finalizer.invocation.id));
        assert_eq!(finalized.can_remove, failure.is_none());
        if failure.is_some() {
            assert_eq!(observation.state, State::Failed);
            assert!(observation.error.as_ref().unwrap().contains("failed"));
            assert!(finalized.preserve_volumes);
            assert!(finalized.filesystem_changes.is_none());
            assert!(!diagnostics.borrow().is_empty());
            assert!(
                !events
                    .iter()
                    .any(|event| event.invocation.step.ends_with("CollectFilesystemChanges"))
            );
            assert!(!root.path().join("filesystem/changes.json").exists());
        } else {
            assert_eq!(observation.state, State::Succeeded);
            assert!(!finalized.preserve_volumes);
            let collected = events
                .iter()
                .find(|event| {
                    event.invocation.step.ends_with("CollectFilesystemChanges")
                        && event.state == State::Started
                })
                .unwrap();
            assert_eq!(collected.invocation.parent, observation.invocation.parent);
            assert!(
                collected
                    .invocation
                    .dependencies
                    .contains(&observation.invocation.id)
            );
            assert!(collected.elapsed_ns >= observation.elapsed_ns);
            let changes = finalized.filesystem_changes.unwrap();
            let mut volumes = FileChanges::open(changes.named_volumes.manifest)
                .await
                .unwrap();
            let mut seeded = false;
            while let Some(change) = volumes.next_change().await.unwrap() {
                seeded |= change.path == b"seeded";
            }
            assert!(!seeded);
            assert!(!changes.named_volumes.coverage_complete);
            assert!(
                changes
                    .named_volumes
                    .warnings
                    .iter()
                    .any(|note| note.contains("not tracked: Docker named volume"))
            );
            let mut layers = FileChanges::open(changes.container_layers.manifest)
                .await
                .unwrap();
            assert_eq!(
                layers.next_change().await.unwrap().unwrap().path,
                b"/layer-result"
            );
        }
    }
}
