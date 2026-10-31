use bollard::{Docker, query_parameters::*};
use std::collections::HashMap;
use workload_docker::compose::DockerAdapter;

pub async fn assert_no_run_resources(adapter: &DockerAdapter, prefix: &str) {
    let docker = Docker::connect_with_unix(
        adapter.socket_path().to_str().unwrap(),
        120,
        bollard::API_DEFAULT_VERSION,
    )
    .unwrap()
    .negotiate_version()
    .await
    .unwrap();
    let filters = Some(HashMap::from([(
        "label".into(),
        vec!["io.doramcap.run".into()],
    )]));
    let label = |labels: &HashMap<String, String>| {
        labels
            .get("io.doramcap.run")
            .is_some_and(|run| run.starts_with(prefix))
    };
    for container in docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters: filters.clone(),
            ..Default::default()
        }))
        .await
        .unwrap()
    {
        assert!(
            !container.labels.as_ref().is_some_and(label),
            "container remains: {:?}",
            container.id
        );
    }
    for network in docker
        .list_networks(Some(ListNetworksOptions {
            filters: filters.clone(),
        }))
        .await
        .unwrap()
    {
        assert!(
            !network.labels.as_ref().is_some_and(label),
            "network remains: {:?}",
            network.id
        );
    }
    for volume in docker
        .list_volumes(Some(ListVolumesOptions { filters }))
        .await
        .unwrap()
        .volumes
        .unwrap_or_default()
    {
        assert!(!label(&volume.labels), "volume remains: {}", volume.name);
    }
}
