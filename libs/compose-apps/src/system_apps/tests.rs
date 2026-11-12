use super::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[test]
fn selected_services_keep_native_readiness_rules() {
    // Arrange: a ready system app shares a project with an unstarted user app.
    let plan = ResolvedComposeProject {
        services: BTreeMap::from([
            (
                "system".into(),
                ComposeService {
                    replicas: 1,
                    image_id: "image".into(),
                    ..Default::default()
                },
            ),
            (
                "user".into(),
                ComposeService {
                    replicas: 1,
                    ..Default::default()
                },
            ),
        ]),
        ..Default::default()
    };
    let state = ComposeProjectState {
        plan: Some(plan),
        containers: vec![Container {
            service: "system".into(),
            image_id: "image".into(),
            status: "running".into(),
            ..Default::default()
        }],
    };
    assert!(!state.ready().unwrap());

    // Act / Assert: watching only the ready service ignores unstarted extra services.
    let mut selected = state.clone();
    select_services(&mut selected, &["system".into()]).unwrap();
    assert!(selected.ready().unwrap());

    // Act / Assert: a missing selected service remains unready.
    let mut selected = state.clone();
    select_services(&mut selected, &["user".into()]).unwrap();
    assert!(!selected.ready().unwrap());

    // Act / Assert: native restart and unhealthy checks still fail for selected services.
    for failed in ["restart", "health"] {
        let mut selected = state.clone();
        select_services(&mut selected, &["system".into()]).unwrap();
        if failed == "restart" {
            selected.containers[0].restart_count = 1;
        } else {
            selected.containers[0].health = Some("unhealthy".into());
        }
        assert!(selected.ready().is_err());
    }
}

#[tokio::test]
async fn disconnected_event_producer_ends_watching() {
    // Arrange: the project is ready, but its Docker event producer has disconnected.
    let root = tempfile::tempdir().unwrap();
    let (adapter, server) = crate::compose::adapter::tests::test_engine(
        root.path(),
        vec![("/containers/json", serde_json::json!([]))],
    );
    let adapter = Rc::new(adapter);
    let plan = ResolvedComposeProject::default();
    let resources = Rc::new(tokio::sync::Mutex::new(
        crate::experiment::ExperimentOwner::new(
            Default::default(),
            Default::default(),
            adapter.clone(),
            plan.clone(),
            1,
            1,
        ),
    ));
    let mut executor = LocalExecutor::default();
    crate::steps::register(&mut executor, adapter, Rc::new(RefCell::new(Vec::new())));
    let (producer, changes) = tokio::sync::watch::channel(());
    drop(producer);

    // Act
    let error = watch(&executor, &plan, &[], &resources, changes)
        .await
        .unwrap_err();

    // Assert
    assert!(
        error
            .to_string()
            .contains("Docker event notifications ended")
    );
    server.await.unwrap();
}

#[test]
fn removal_retains_the_dora_reservation_and_treats_shm_as_an_ordinary_container() {
    use crate::compose::adapter::EXPERIMENT_OWNER_LABEL;
    use bollard::models::ContainerSummary;
    use std::collections::HashMap;
    // Arrange: the holder is a separate service; only the daemon carries the reservation label.
    let holder = ContainerSummary {
        id: Some("shm".into()),
        ..Default::default()
    };
    let owner = ContainerSummary {
        id: Some("daemon".into()),
        labels: Some(HashMap::from([(
            EXPERIMENT_OWNER_LABEL.into(),
            "true".into(),
        )])),
        ..Default::default()
    };

    // Act / Assert: list order does not determine the final container.
    assert_eq!(removal_owner([&owner, &holder]).unwrap(), Some("daemon"));
    assert_eq!(removal_owner([&holder, &owner]).unwrap(), Some("daemon"));
    assert_eq!(removal_owner([&holder]).unwrap(), None);

    // Act / Assert: ambiguous or incomplete reservation evidence blocks cleanup.
    assert!(removal_owner([&owner, &owner]).is_err());
    let missing = ContainerSummary { id: None, ..owner };
    assert!(removal_owner([&missing]).is_err());
}

#[tokio::test]
async fn watching_an_unknown_service_fails_before_engine_access() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let (adapter, server) = crate::compose::adapter::tests::test_engine(root.path(), vec![]);
    let adapter = Rc::new(adapter);
    let plan = ResolvedComposeProject::default();
    let resources = Rc::new(tokio::sync::Mutex::new(
        crate::experiment::ExperimentOwner::new(
            Default::default(),
            Default::default(),
            adapter,
            plan.clone(),
            1,
            1,
        ),
    ));
    let (_producer, changes) = tokio::sync::watch::channel(());

    // Act
    let error = watch(
        &LocalExecutor::default(),
        &plan,
        &["missing".into()],
        &resources,
        changes,
    )
    .await
    .unwrap_err();

    // Assert
    assert_eq!(
        error.to_string(),
        "unknown watched Compose service: missing"
    );
    server.await.unwrap();
}
