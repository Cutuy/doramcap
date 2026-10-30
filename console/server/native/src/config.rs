use crate::{
    policy,
    protocol::{ConsoleConfig, RuntimeConfig},
};
use prost_reflect::{DescriptorPool, DynamicMessage};
use std::{path::Path, sync::OnceLock};

/// Load and validate a Console configuration from protobuf text.
pub fn load(path: &Path) -> Result<ConsoleConfig, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let config = parse(&text)?;
    let secret = std::fs::read_to_string(&config.access_secret_file)
        .map_err(|e| format!("access_secret_file: {e}"))?;
    if secret
        .strip_suffix("\r\n")
        .or_else(|| secret.strip_suffix('\n'))
        .unwrap_or(&secret)
        .is_empty()
    {
        return Err("access_secret_file must contain a nonempty secret".into());
    }
    Ok(config)
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
    let origin = url::Url::parse(&config.public_origin)
        .map_err(|e| format!("invalid public_origin: {e}"))?;
    if !matches!(origin.scheme(), "http" | "https")
        || origin.origin().ascii_serialization() != config.public_origin
    {
        return Err(
            "public_origin must be a canonical HTTP(S) origin without a path or credentials".into(),
        );
    }
    let runtime = runtime(&config);
    for (name, value) in [
        ("readiness_seconds", runtime.readiness_seconds),
        ("stop_seconds", runtime.stop_seconds),
        ("sample_interval_ms", runtime.sample_interval_ms),
    ] {
        if value == Some(0) {
            return Err(format!("runtime.{name} must be positive"));
        }
    }
    config
        .listen_address
        .parse::<std::net::SocketAddr>()
        .map_err(|e| format!("invalid listen_address: {e}"))?;
    if let Some(apps) = &config.system_apps {
        if apps
            .cpu_monitor
            .as_ref()
            .is_some_and(|monitor| monitor.image.is_empty())
        {
            return Err("CPU monitor image required".into());
        }
        if !apps.dora_flows.is_empty() {
            return Err("Dora flows are not supported by Console MVP".into());
        }
    }
    Ok(config)
}

/// Resolve omitted policy fields using the original defaults in generated types.
pub fn runtime(config: &ConsoleConfig) -> RuntimeConfig {
    let mut runtime = config.runtime.unwrap_or_default();
    runtime
        .readiness_seconds
        .get_or_insert(policy::READINESS_SECONDS);
    runtime.stop_seconds.get_or_insert(policy::STOP_SECONDS);
    runtime
        .sample_interval_ms
        .get_or_insert(policy::DEFAULT_SAMPLE_INTERVAL_MS);
    runtime
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
    fn runtime_defaults_overrides_and_zero_constraints_are_explicit() {
        // Arrange
        let default = parse(VALID).unwrap();
        let custom = format!(
            "{VALID} runtime {{ readiness_seconds: 90 stop_seconds: 25 sample_interval_ms: 250 }}"
        );
        // Act
        let custom = runtime(&parse(&custom).unwrap());
        // Assert
        assert_eq!(runtime(&default).readiness_seconds, Some(60));
        assert_eq!(runtime(&default).stop_seconds, Some(15));
        assert_eq!(runtime(&default).sample_interval_ms, Some(100));
        assert_eq!(custom.readiness_seconds, Some(90));
        assert_eq!(custom.stop_seconds, Some(25));
        assert_eq!(custom.sample_interval_ms, Some(250));
        for field in ["readiness_seconds", "stop_seconds", "sample_interval_ms"] {
            // Act / Assert
            let error = parse(&format!("{VALID} runtime {{ {field}: 0 }}")).unwrap_err();
            assert!(error.contains(field));
        }
    }

    #[test]
    fn origin_requires_canonical_http_authority() {
        for origin in [
            "https://example.com",
            "http://[::1]:8080",
            "http://localhost:8080",
        ] {
            // Arrange / Act
            let result = parse(&VALID.replace("http://127.0.0.1:8080", origin));
            // Assert
            assert!(result.is_ok(), "{origin}");
        }
        for origin in [
            "",
            "https://",
            "ftp://example.com",
            "http://example.com/",
            "http://example.com/path",
            "http://user:pass@example.com",
            "http://EXAMPLE.com",
            "http://example.com:80",
            "http://example.com?x",
            "http://example.com#x",
        ] {
            // Arrange / Act
            let result = parse(&VALID.replace("http://127.0.0.1:8080", origin));
            // Assert
            assert!(result.unwrap_err().contains("public_origin"), "{origin}");
        }
    }

    #[test]
    fn loading_rejects_missing_or_empty_secret_without_creating_data_root() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let secret = root.path().join("secret");
        let data = root.path().join("data");
        let config = root.path().join("console.pbtxt");
        std::fs::write(
            &config,
            VALID
                .replace("/secrets/console", secret.to_str().unwrap())
                .replace("/data", data.to_str().unwrap()),
        )
        .unwrap();
        // Act / Assert
        assert!(load(&config).unwrap_err().contains("access_secret_file"));
        assert!(!data.exists());
        for contents in ["", "\n", "\r\n"] {
            std::fs::write(&secret, contents).unwrap();
            assert!(load(&config).unwrap_err().contains("nonempty secret"));
            assert!(!data.exists());
        }
        // Act
        std::fs::write(&secret, "secret\r\n").unwrap();
        // Assert
        assert!(load(&config).is_ok());
        assert!(!data.exists());
    }

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
    fn accepts_no_apps_and_rejects_relative_paths_empty_images_and_dora() {
        // Arrange
        let missing = VALID.replace("cpu_monitor { image: \"monitor:local\" }", "");
        let relative = VALID.replace("/projects", "projects");
        let dora = VALID.replace(
            "system_apps {",
            "system_apps { dora_flows { name: \"flow\" }",
        );
        // Act / Assert
        assert!(parse(&missing).is_ok());
        assert!(parse(&VALID.replace("monitor:local", "")).is_err());
        assert!(parse(&relative).is_err());
        assert!(parse(&dora).is_err());
        assert!(parse(&format!("{VALID} unknown_field: 1")).is_err());
    }
}
