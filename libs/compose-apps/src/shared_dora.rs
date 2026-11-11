//! Dora preparation from ordinary native app inputs and a dataflow artifact.
use super::dora::DoraApps;
use crate::compose::{
    Result,
    adapter::{digest, private_write},
    error::invalid,
};
use serde_json::json;
use std::path::Path;
use workload_lifecycle::{
    dora::{option, port},
    launch::{DoraRuntime, ResolvedFileBinding},
};

pub(crate) const MAX_GRAPH_BYTES: u64 = 2 * 1024 * 1024;
pub(crate) const GRAPH_FILE: &str = "dataflow.yml";

/// Return the stable daemon ownership identity for one canonical repository.
pub fn owner(repository: &Path) -> String {
    format!(
        "{}-dora",
        crate::compose::adapter::project_name("experiment", repository)
    )
}

/// Resolve native zero-valued ports consistently across the two declared apps.
pub fn resolve(apps: &mut DoraApps, runtime_directory: String) -> Result<Option<DoraRuntime>> {
    if !apps.enabled() {
        return Ok(None);
    }
    if apps.services.len() != 2 {
        return Err(invalid(
            "Dora needs exactly one coordinator and one daemon service",
        ));
    }
    let coordinator = apps.coordinator()?.0.to_owned();
    let daemon = apps.daemon()?.0.to_owned();
    let mut coordinator_args = DoraApps::argv(&apps.services[&coordinator])?;
    let mut daemon_args = DoraApps::argv(&apps.services[&daemon])?;
    let automatic =
        workload_lifecycle::dora::resolve_ports(&mut coordinator_args, &mut daemon_args)
            .map_err(invalid)?;
    workload_lifecycle::dora::endpoint(
        &coordinator_args,
        apps.services[&coordinator]["environment"]["DORA_COORDINATOR_INTERFACE"].as_str(),
    )
    .map_err(invalid)?;
    let overlay = option(&daemon_args, "--zenoh-config-overlay").map_err(invalid)?;
    let runtime = DoraRuntime {
        coordinator_port: port(&coordinator_args, "--port").map_err(invalid)?.into(),
        daemon_port: port(&daemon_args, "--local-listen-port")
            .map_err(invalid)?
            .into(),
        automatic_coordinator_port: automatic[0],
        automatic_daemon_port: automatic[1],
        runtime_directory,
        zenoh: Some(ResolvedFileBinding {
            source: DoraApps::host_path(&apps.services[&daemon], overlay)?,
            target: overlay.to_owned(),
        }),
        ..Default::default()
    };
    for (name, args) in [(&coordinator, coordinator_args), (&daemon, daemon_args)] {
        let app = apps.services.get_mut(name).unwrap();
        app["entrypoint"] = json!([args[0]]);
        app["command"] = json!(args[1..]);
    }
    for app in apps.services.values_mut() {
        app["environment"][dora_app::environment::DAEMON_PORT] =
            runtime.daemon_port.to_string().into();
    }
    Ok(Some(runtime))
}

pub fn validate_endpoints(runtime: Option<&DoraRuntime>) -> Result<()> {
    let Some(runtime) = runtime else {
        return Ok(());
    };
    let mut listeners = Vec::new();
    for port in [runtime.coordinator_port as u16, runtime.daemon_port as u16] {
        listeners.push(
            std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
                .map_err(|e| invalid(format!("Dora endpoint {port} unavailable: {e}")))?,
        );
    }
    Ok(())
}

/// Freeze the dataflow and mount it into the coordinator's existing working directory.
pub(crate) async fn prepare(
    apps: &mut DoraApps,
    runtime: Option<&mut DoraRuntime>,
    dataflow_file: &str,
    artifacts: &Path,
) -> Result<()> {
    let Some(runtime) = runtime.filter(|_| !dataflow_file.is_empty()) else {
        return Ok(());
    };
    let bytes = crate::evidence::read_regular(Path::new(dataflow_file), MAX_GRAPH_BYTES).await?;
    let bytes = serde_json::to_vec(
        &workload_lifecycle::dora::graph(dataflow_file, &bytes).map_err(invalid)?,
    )?;
    let source = artifacts.join(GRAPH_FILE).to_string_lossy().into_owned();
    runtime.dataflow_sha256 = digest(&bytes);
    private_write(Path::new(&source), &bytes).await?;
    let name = apps.coordinator()?.0.to_owned();
    let directory = apps.coordinator()?.1["working_dir"]
        .as_str()
        .ok_or_else(|| invalid("Dora coordinator needs a working_dir"))?;
    let target = Path::new(directory)
        .join(GRAPH_FILE)
        .to_string_lossy()
        .into_owned();
    apps.services.get_mut(&name).unwrap()["volumes"]
        .as_array_mut()
        .ok_or_else(|| invalid("Dora coordinator needs native volumes"))?
        .push(json!({"type":"bind","source":source,"target":target,"read_only":true}));
    runtime.dataflow = Some(ResolvedFileBinding { source, target });
    Ok(())
}
