//! Generated Compose launch, execution, observation and cleanup contracts.
include!(concat!(env!("OUT_DIR"), "/doramcap.compose.v1.rs"));

impl ResolvedBind {
    /// Read every bind mount from a rendered Compose manifest, in service and mount order.
    /// Errors: `Invalid` when `services` or a bind `source` is missing.
    pub(crate) fn from_manifest(model: &serde_json::Value) -> super::Result<Vec<Self>> {
        use super::error::invalid;
        let services = model["services"]
            .as_object()
            .ok_or_else(|| invalid("missing services"))?;
        let mut binds = Vec::new();
        for (service, value) in services {
            let mounts = value["volumes"].as_array().into_iter().flatten();
            for mount in mounts.filter(|mount| mount["type"] == "bind") {
                let source = mount["source"]
                    .as_str()
                    .ok_or_else(|| invalid(format!("{service}: bind source missing")))?;
                let options = &mount["bind"];
                binds.push(Self {
                    service: service.clone(),
                    source: source.into(),
                    target: mount["target"].as_str().unwrap_or_default().into(),
                    read_only: mount["read_only"] == true,
                    create_host_path: options["create_host_path"] != false,
                    propagation: options["propagation"].as_str().unwrap_or_default().into(),
                    selinux: options.get("selinux").is_some(),
                });
            }
        }
        Ok(binds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolved_binds_read_rendered_compose_mounts() {
        // Arrange: mount forms rendered by `docker compose config --format json`.
        let model = json!({"services":{
            "app":{"volumes":[
                {"type":"bind","source":"/long","target":"/a"},
                {"type":"volume","source":"data","target":"/b"},
                {"type":"bind","source":"/short","target":"/c","bind":{"create_host_path":true}}
            ]},
            "worker":{"volumes":[{
                "type":"bind","source":"/strict","target":"/d","read_only":true,
                "bind":{"create_host_path":false,"propagation":"rshared","selinux":"z"}
            }]},
            "idle":{}
        }});

        // Act
        let binds = ResolvedBind::from_manifest(&model).unwrap();

        // Assert
        let bind = |service: &str, source: &str, target: &str, create_host_path| ResolvedBind {
            service: service.into(),
            source: source.into(),
            target: target.into(),
            create_host_path,
            ..Default::default()
        };
        assert_eq!(
            binds,
            [
                bind("app", "/long", "/a", true),
                bind("app", "/short", "/c", true),
                ResolvedBind {
                    read_only: true,
                    propagation: "rshared".into(),
                    selinux: true,
                    ..bind("worker", "/strict", "/d", false)
                },
            ]
        );
    }

    #[test]
    fn resolved_binds_reject_manifests_without_services_or_sources() {
        // Arrange
        let unsourced = json!({"services":{"app":{"volumes":[{"type":"bind"}]}}});

        // Act
        let error = ResolvedBind::from_manifest(&unsourced).unwrap_err();

        // Assert
        assert_eq!(error.to_string(), "app: bind source missing");
        assert!(ResolvedBind::from_manifest(&json!({})).is_err());
    }
}
