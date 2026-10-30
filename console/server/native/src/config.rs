use crate::protocol::ConsoleConfig;
use prost_reflect::{DescriptorPool, DynamicMessage};
use std::{path::Path, sync::OnceLock};

/// Load and validate a Console configuration from protobuf text.
pub fn load(path: &Path) -> Result<ConsoleConfig, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse(&text)
}

/// Parse and validate a Console configuration from protobuf text.
pub fn parse(text: &str) -> Result<ConsoleConfig, String> {
    static POOL: OnceLock<DescriptorPool> = OnceLock::new();
    let pool = POOL.get_or_init(|| {
        DescriptorPool::decode(include_bytes!(concat!(env!("OUT_DIR"), "/console.bin")).as_slice())
            .expect("compiled Console descriptor")
    });
    let descriptor = pool
        .get_message_by_name("doramcap.console.v1.ConsoleConfig")
        .expect("compiled Console configuration");
    let config: ConsoleConfig = DynamicMessage::parse_text_format(descriptor, text)
        .map_err(|e| e.to_string())?
        .transcode_to()
        .map_err(|e| e.to_string())?;
    for (name, value) in [
        ("project_root", &config.project_root),
        ("data_root", &config.data_root),
        ("docker_socket", &config.docker_socket),
        ("access_secret_file", &config.access_secret_file),
    ] {
        if !Path::new(value).is_absolute() {
            return Err(format!("{name} must be absolute"));
        }
    }
    if config.public_origin.is_empty() {
        return Err("public_origin required".into());
    }
    config
        .listen_address
        .parse::<std::net::SocketAddr>()
        .map_err(|e| format!("invalid listen_address: {e}"))?;
    let apps = config.system_apps.as_ref().ok_or("CPU monitor required")?;
    if apps
        .cpu_monitor
        .as_ref()
        .is_none_or(|monitor| monitor.image.is_empty())
    {
        return Err("CPU monitor image required".into());
    }
    if !apps.dora_flows.is_empty() {
        return Err("Dora flows are not supported by Console MVP".into());
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    const VALID: &str = r#"
        listen_address: "127.0.0.1:8080"
        public_origin: "http://127.0.0.1:8080"
        project_root: "/projects"
        data_root: "/data"
        docker_socket: "/var/run/docker.sock"
        access_secret_file: "/secrets/console"
        system_apps { cpu_monitor { image: "monitor:local" } }
    "#;

    #[test]
    fn accepts_required_monitor_configuration() {
        // Arrange / Act
        let config = parse(VALID).unwrap();
        // Assert
        assert_eq!(
            config.system_apps.unwrap().cpu_monitor.unwrap().image,
            "monitor:local"
        );
    }

    #[test]
    fn rejects_missing_monitor_relative_paths_and_dora() {
        // Arrange
        let missing = VALID.replace("cpu_monitor { image: \"monitor:local\" }", "");
        let relative = VALID.replace("/projects", "projects");
        let dora = VALID.replace(
            "system_apps {",
            "system_apps { dora_flows { name: \"flow\" }",
        );
        // Act / Assert
        assert!(parse(&missing).is_err());
        assert!(parse(&relative).is_err());
        assert!(parse(&dora).is_err());
        assert!(parse(&format!("{VALID} unknown_field: 1")).is_err());
    }
}
