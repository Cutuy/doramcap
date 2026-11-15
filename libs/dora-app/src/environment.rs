//! Approved environment bridge for Compose- and systemd-owned Dora apps.
//! Environment-based configuration is highly discouraged. Every new input or
//! extension of this contract requires the user's explicit approval.

/// Native graph identity; the launch backend supplies the app ID unless overridden.
pub const NODE_ID: &str = "DORA_NODE_ID";
/// Native Dora daemon port on localhost.
pub const DAEMON_PORT: &str = "DORA_DAEMON_LOCAL_LISTEN_PORT";
/// Read-only native Zenoh transport settings.
pub const ZENOH_OVERLAY: &str = "DORA_ZENOH_CONFIG_OVERLAY";

/// Approved connection settings supplied to first-party and adopted apps.
/// Preserve an app's declared NODE_ID; the experiment owns port/overlay values.
pub fn settings(node_id: &str, port: u32, overlay: &str) -> [(&'static str, String); 3] {
    [
        (NODE_ID, node_id.into()),
        (DAEMON_PORT, port.to_string()),
        (ZENOH_OVERLAY, overlay.into()),
    ]
}

/// Attach using NODE_ID and Dora's native DAEMON_PORT/ZENOH_OVERLAY environment.
/// With no explicit ID, use Dora's native invocation context. The returned node
/// owns the connection; retain it while consuming the event stream.
#[cfg(feature = "node")]
pub fn connect() -> eyre::Result<(dora_node_api::DoraNode, dora_node_api::EventStream)> {
    let mut builder = dora_node_api::DoraNode::builder();
    if let Some(id) = std::env::var_os(NODE_ID) {
        builder = builder.node_id(
            id.into_string()
                .map_err(|_| eyre::eyre!("invalid DORA_NODE_ID"))?
                .parse()?,
        );
    }
    Ok(builder.build()?)
}
