//! Dora preparation from ordinary native app inputs and a dataflow artifact.
use super::dora::DoraApps;
use crate::compose::{
    Result,
    adapter::{digest, private_write},
    error::invalid,
};
use dora_core::descriptor::{Descriptor, DescriptorExt, validate::check_dataflow_static};
use serde_json::json;
use std::path::Path;

pub(crate) const MAX_GRAPH_BYTES: u64 = 2 * 1024 * 1024;
pub(crate) const GRAPH_FILE: &str = "dataflow.yml";

pub(crate) fn owner(repository: &Path) -> String {
    format!(
        "{}-dora",
        crate::compose::adapter::project_name("experiment", repository)
    )
}

/// Resolve native zero-valued ports consistently across the two declared apps.
pub fn resolve(apps: &mut DoraApps) -> Result<()> {
    if !apps.enabled() {
        return Ok(());
    }
    if apps.services.len() != 2 {
        return Err(invalid(
            "Dora needs exactly one coordinator and one daemon service",
        ));
    }
    let coordinator = apps.coordinator()?.0.to_owned();
    let daemon = apps.daemon()?.0.to_owned();
    let read = |name: &str, flag| -> Result<u16> {
        workload_lifecycle::dora::option(&DoraApps::argv(&apps.services[name])?, flag)
            .map_err(invalid)?
            .parse()
            .map_err(|_| invalid("invalid Dora port"))
    };
    let mut ports = [
        read(&coordinator, "--port")?,
        read(&daemon, "--local-listen-port")?,
    ];
    if read(&daemon, "--coordinator-port")? != ports[0] {
        return Err(invalid("Dora app coordinator endpoints disagree"));
    }
    let mut listeners = Vec::new();
    for (index, port) in ports.iter_mut().enumerate() {
        apps.automatic_ports[index] = *port == 0;
        if *port == 0 {
            let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
            *port = listener.local_addr()?.port();
            listeners.push(listener);
        }
    }
    if ports[0] == ports[1] {
        return Err(invalid("Dora ports must differ"));
    }
    for (name, flag, port) in [
        (&coordinator, "--port", ports[0]),
        (&daemon, "--coordinator-port", ports[0]),
        (&daemon, "--local-listen-port", ports[1]),
    ] {
        for field in ["entrypoint", "command"] {
            if let Some(args) = apps.services.get_mut(name).unwrap()[field].as_array_mut() {
                for index in 0..args.len() {
                    if args[index] == flag && index + 1 < args.len() {
                        args[index + 1] = port.to_string().into();
                    } else if args[index]
                        .as_str()
                        .is_some_and(|arg| arg.starts_with(&format!("{flag}=")))
                    {
                        args[index] = format!("{flag}={port}").into();
                    }
                }
            }
        }
    }
    for app in apps.services.values_mut() {
        if app["environment"]
            .get(dora_app::environment::DAEMON_PORT)
            .is_some()
        {
            app["environment"][dora_app::environment::DAEMON_PORT] = ports[1].to_string().into();
        }
    }
    Ok(())
}

pub fn validate_endpoints(apps: &DoraApps) -> Result<()> {
    if !apps.enabled() {
        return Ok(());
    }
    let mut listeners = Vec::new();
    for port in [apps.coordinator_port()?, apps.daemon_port()?] {
        listeners.push(
            std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
                .map_err(|e| invalid(format!("Dora endpoint {port} unavailable: {e}")))?,
        );
    }
    Ok(())
}

/// Expand native modules and require externally launched nodes in the graph.
pub fn graph(dataflow_file: &str, bytes: &[u8]) -> Result<Descriptor> {
    let graph = Descriptor::parse(bytes.to_vec())
        .map_err(|e| invalid(format!("invalid Dora graph: {e:#}")))?
        .expand(
            Path::new(dataflow_file)
                .parent()
                .ok_or_else(|| invalid("graph parent missing"))?,
        )
        .map_err(|e| invalid(format!("Dora module expansion: {e:#}")))?;
    check_dataflow_static(&graph).map_err(|e| invalid(format!("invalid Dora graph: {e:#}")))?;
    if graph.nodes.iter().any(|n| {
        n.path.as_deref() != Some("dynamic")
            || n.operators.is_some()
            || n.operator.is_some()
            || n.ros2.is_some()
            || n.build.is_some()
            || n.git.is_some()
            || n.hub.is_some()
    }) {
        return Err(invalid(
            "Dora graph must use externally launched dynamic nodes",
        ));
    }
    Ok(graph)
}

/// Freeze the dataflow and mount it into the coordinator's existing working directory.
pub(crate) async fn prepare(
    apps: &mut DoraApps,
    dataflow_file: &str,
    artifacts: &Path,
) -> Result<()> {
    if !apps.enabled() || dataflow_file.is_empty() {
        return Ok(());
    }
    let bytes = crate::evidence::read_regular(Path::new(dataflow_file), MAX_GRAPH_BYTES).await?;
    let bytes = serde_json::to_vec(&graph(dataflow_file, &bytes)?)?;
    apps.dataflow_file = artifacts.join(GRAPH_FILE).to_string_lossy().into_owned();
    apps.dataflow_sha256 = digest(&bytes);
    private_write(Path::new(&apps.dataflow_file), &bytes).await?;
    let name = apps.coordinator()?.0.to_owned();
    let target = Path::new(apps.directory()?)
        .join(GRAPH_FILE)
        .to_string_lossy()
        .into_owned();
    apps.services.get_mut(&name).unwrap()["volumes"]
        .as_array_mut()
        .ok_or_else(|| invalid("Dora coordinator needs native volumes"))?
        .push(json!({"type":"bind","source":apps.dataflow_file,"target":target,"read_only":true}));
    Ok(())
}
