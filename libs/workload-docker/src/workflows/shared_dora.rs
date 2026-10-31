//! Native Dora graph validation and immutable communication inputs.
use super::dora::DoraRuntime;
use crate::compose::{
    Result,
    adapter::{digest, private_write},
    error::invalid,
};
use dora_core::{
    config::InputMapping,
    descriptor::{Descriptor, DescriptorExt, validate::check_dataflow_static},
};
use std::{collections::HashSet, path::Path};

pub(crate) const COORDINATOR: &str = "dora-coordinator";
pub(crate) const DAEMON: &str = "dora-daemon";
const OVERLAY: &[u8] = b"{transport:{shared_memory:{enabled:true}}}\n";

/// Resolve local ports and private paths. No runtime resources are created.
pub fn resolve(runtime: &mut DoraRuntime, artifacts: &Path) -> Result<()> {
    if runtime.image.is_empty() {
        runtime.image = "doramcap/dora:local".into();
    }
    let mut listeners = Vec::new();
    for port in [&mut runtime.coordinator_port, &mut runtime.daemon_port] {
        if *port == 0 {
            let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
            *port = listener.local_addr()?.port().into();
            listeners.push(listener);
        }
    }
    if runtime.runtime_directory.is_empty() {
        runtime.runtime_directory = artifacts
            .join("dora/runtime")
            .to_string_lossy()
            .into_owned();
    }
    runtime.zenoh_overlay = artifacts
        .join("dora/zenoh.json5")
        .to_string_lossy()
        .into_owned();
    runtime.shm_bytes = runtime.shm_bytes.max(128 * 1024 * 1024);
    runtime.memlock_bytes = runtime.memlock_bytes.max(64 * 1024 * 1024);
    // SAFETY: effective identity access has no side effects.
    runtime
        .participant_uid
        .get_or_insert_with(|| unsafe { libc::geteuid() });
    runtime
        .participant_gid
        .get_or_insert_with(|| unsafe { libc::getegid() });
    validate_config(runtime)
}

pub fn validate_config(runtime: &DoraRuntime) -> Result<()> {
    if runtime.coordinator_port == 0
        || runtime.daemon_port == 0
        || runtime.coordinator_port > 65535
        || runtime.daemon_port > 65535
        || runtime.coordinator_port == runtime.daemon_port
    {
        return Err(invalid("Dora ports must be distinct and in 1..65535"));
    }
    if !Path::new(&runtime.runtime_directory).is_absolute()
        || (!runtime.graph.is_empty() && !Path::new(&runtime.graph).is_absolute())
    {
        return Err(invalid("Dora paths must be absolute"));
    }
    Ok(())
}

/// Reject occupied endpoints before starting any experiment container.
pub fn validate_endpoints(runtime: &DoraRuntime) -> Result<()> {
    validate_config(runtime)?;
    let mut held = Vec::new();
    for port in [runtime.coordinator_port, runtime.daemon_port] {
        held.push(
            std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port as u16))
                .map_err(|e| invalid(format!("Dora endpoint {port} unavailable: {e}")))?,
        );
    }
    Ok(())
}

/// Expand native modules and require externally launched nodes with a drainable graph.
pub fn graph(runtime: &DoraRuntime, bytes: &[u8]) -> Result<Descriptor> {
    let graph = Descriptor::parse(bytes.to_vec())
        .map_err(|e| invalid(format!("invalid Dora graph: {e:#}")))?
        .expand(
            Path::new(&runtime.graph)
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
    let mut remaining: HashSet<_> = graph.nodes.iter().map(|n| n.id.to_string()).collect();
    while !remaining.is_empty() {
        let before = remaining.len();
        for node in &graph.nodes {
            if node.inputs.values().all(|i| !matches!(&i.mapping, InputMapping::User(m) if remaining.contains(m.source.as_ref()))) {
                remaining.remove(node.id.as_ref());
            }
        }
        if remaining.len() == before {
            return Err(invalid("Dora graph cycles prevent ordered drain"));
        }
    }
    Ok(graph)
}

/// Freeze the expanded graph and SHM settings into the private run directory.
pub(crate) async fn prepare(runtime: &DoraRuntime, artifacts: &Path) -> Result<DoraRuntime> {
    let directory = artifacts.join("dora");
    crate::evidence::create_private_dir(&directory).await?;
    if !Path::new(&runtime.runtime_directory).exists() {
        crate::evidence::create_private_dir(Path::new(&runtime.runtime_directory)).await?;
    }
    let mut frozen = runtime.clone();
    if !runtime.graph.is_empty() {
        let bytes =
            crate::evidence::read_regular(Path::new(&runtime.graph), 2 * 1024 * 1024).await?;
        let bytes = serde_json::to_vec(&graph(runtime, &bytes)?)?;
        frozen.graph = directory.join("graph.yml").to_string_lossy().into_owned();
        frozen.graph_sha256 = digest(&bytes);
        private_write(Path::new(&frozen.graph), &bytes).await?;
    }
    private_write(Path::new(&frozen.zenoh_overlay), OVERLAY).await?;
    Ok(frozen)
}
