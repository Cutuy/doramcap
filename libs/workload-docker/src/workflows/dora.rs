//! Dora control over ordinary, resolved Compose app definitions.
use super::experiment::ExperimentOwner;
use crate::compose::{error::invalid, *};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, time::Duration};
use workload_lifecycle::dora::{option, port};

// Approved polling exception until Dora reports daemon readiness.
const DORA_STARTUP_CHECK_INTERVAL: Duration = Duration::from_millis(100);

/// Resolved native Compose inputs and immutable dataflow evidence used by control steps.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DoraApps {
    pub services: BTreeMap<String, Value>,
    pub dataflow_file: String,
    pub dataflow_sha256: String,
    pub automatic_ports: [bool; 2],
}
impl DoraApps {
    pub fn enabled(&self) -> bool {
        !self.services.is_empty()
    }
    pub fn argv(service: &Value) -> Result<Vec<String>> {
        ["entrypoint", "command"]
            .into_iter()
            .try_fold(Vec::new(), |mut argv, key| {
                if let Some(parts) = service.get(key) {
                    for part in parts.as_array().ok_or_else(|| {
                        invalid("Dora app entrypoint and command must be argument lists")
                    })? {
                        argv.push(
                            part.as_str()
                                .ok_or_else(|| invalid("Dora app argument must be a string"))?
                                .to_owned(),
                        );
                    }
                }
                Ok(argv)
            })
    }
    fn app(&self, command: &str) -> Result<(&str, &Value)> {
        let mut matches = self.services.iter().filter(|(_, service)| {
            Self::argv(service).is_ok_and(|argv| argv.get(1).is_some_and(|arg| arg == command))
        });
        let (name, app) = matches
            .next()
            .ok_or_else(|| invalid(format!("Dora {command} app missing")))?;
        if matches.next().is_some() {
            return Err(invalid(format!("duplicate Dora {command} apps")));
        }
        Ok((name, app))
    }
    pub fn coordinator(&self) -> Result<(&str, &Value)> {
        self.app("coordinator")
    }
    pub fn daemon(&self) -> Result<(&str, &Value)> {
        self.app("daemon")
    }
    pub fn coordinator_port(&self) -> Result<u16> {
        port(&Self::argv(self.coordinator()?.1)?, "--port").map_err(invalid)
    }
    pub fn daemon_port(&self) -> Result<u16> {
        port(&Self::argv(self.daemon()?.1)?, "--local-listen-port").map_err(invalid)
    }
    pub fn directory(&self) -> Result<&str> {
        self.coordinator()?.1["working_dir"]
            .as_str()
            .ok_or_else(|| invalid("Dora coordinator needs a working_dir"))
    }
    pub fn overlay(&self) -> Result<String> {
        Ok(
            option(&Self::argv(self.daemon()?.1)?, "--zenoh-config-overlay")
                .map_err(invalid)?
                .to_owned(),
        )
    }
    /// Resolve a file in an app's view through its ordinary bind mounts.
    pub fn host_path(service: &Value, target: &str) -> Result<String> {
        let path = std::path::Path::new(target);
        let mount = service["volumes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|mount| mount["type"] == "bind")
            .filter_map(|mount| Some((mount["source"].as_str()?, mount["target"].as_str()?)))
            .filter(|(_, target)| path.starts_with(target))
            .max_by_key(|(_, target)| target.len())
            .ok_or_else(|| invalid(format!("Dora app needs a bind for {target}")))?;
        let suffix = path.strip_prefix(mount.1).unwrap();
        if suffix.as_os_str().is_empty() {
            return Ok(mount.0.to_owned());
        }
        Ok(std::path::Path::new(mount.0)
            .join(suffix)
            .to_string_lossy()
            .into_owned())
    }
}

impl ExperimentOwner {
    pub(crate) async fn cli(&self, args: &[&str]) -> Result<String> {
        let argv = DoraApps::argv(self.apps.coordinator()?.1)?;
        let mut command: Vec<_> = [argv[0].as_str()]
            .into_iter()
            .chain(args.iter().copied())
            .map(str::to_owned)
            .collect();
        command.extend([
            "--coordinator-port".into(),
            self.apps.coordinator_port()?.to_string(),
        ]);
        self.adapter
            .project(&self.plan)
            .exec(
                self.apps.coordinator()?.0,
                &command,
                Duration::from_secs(self.startup_seconds),
            )
            .await
    }
    pub(super) async fn start_runtime(&self) -> Result<()> {
        if !self.apps.enabled() {
            return Ok(());
        }
        self.adapter
            .project(&self.plan)
            .start(
                &self.apps.services.keys().cloned().collect::<Vec<_>>(),
                self.startup_seconds,
            )
            .await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(self.startup_seconds);
        loop {
            if self.inspect_runtime().await.is_ok()
                && let Ok(status) = self.cli(&["status", "--format", "json"]).await
            {
                let status: serde_json::Value = serde_json::from_str(&status)?;
                if status["daemon"]["status"] == "running" {
                    return Ok(());
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(invalid("Dora daemon did not connect to coordinator"));
            }
            tokio::time::sleep(DORA_STARTUP_CHECK_INTERVAL).await;
        }
    }
    pub(super) async fn inspect_runtime(&self) -> Result<()> {
        if !self.apps.enabled() {
            return Ok(());
        }
        for (name, port) in [
            (self.apps.coordinator()?.0, self.apps.coordinator_port()?),
            (self.apps.daemon()?.0, self.apps.daemon_port()?),
        ] {
            self.adapter
                .project(&self.plan)
                .endpoint(name, "host", port)
                .await?;
        }
        Ok(())
    }
    /// Workloads must have drained and stopped before native graph/runtime teardown.
    pub(super) async fn stop_runtime(&self) -> Result<Shutdown> {
        if !self.apps.enabled() {
            return Ok(Shutdown {
                stopped: true,
                ..Default::default()
            });
        }
        if self
            .adapter
            .project(&self.plan)
            .endpoint(
                self.apps.coordinator()?.0,
                "host",
                self.apps.coordinator_port()?,
            )
            .await
            .is_ok()
            && let Err(error) = self.cli(&["down", "--force"]).await
        {
            eprintln!(
                "warning: Dora control shutdown failed; verifying container shutdown: {error}"
            );
        }
        let stopped = self
            .adapter
            .project(&self.plan)
            .quiesce(&self.apps.services.keys().cloned().collect::<Vec<_>>())
            .await?;
        if !stopped.stopped {
            return Err(invalid(format!(
                "Dora shutdown incomplete: {:?}",
                stopped.diagnostics
            )));
        }
        Ok(stopped)
    }
}

#[cfg(test)]
pub(crate) fn fixture(root: &str, ports: [u16; 2]) -> DoraApps {
    let mut services = BTreeMap::new();
    for role in ["coordinator", "daemon"] {
        let name = format!("dora-{role}");
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("deploy")
            .join(&name);
        let mut model = crate::compose::adapter::fixture_model(ComposeProject {
            key: name.clone(),
            directory: directory.to_string_lossy().into_owned(),
            files: vec!["compose.yaml".into()],
            manifest: format!("{name}.pbtxt"),
            ..Default::default()
        });
        let app = &mut model["services"][&name];
        app["volumes"] = serde_json::json!([{"type":"bind","source":format!("{root}/dora/runtime"),"target":"/runtime"}]);
        if role == "daemon" {
            app["volumes"].as_array_mut().unwrap().push(serde_json::json!({"type":"bind","source":format!("{root}/dora/zenoh.json5"),"target":"/config/zenoh.json5","read_only":true}));
        }
        let args = app["command"].as_array_mut().unwrap();
        for index in 1..args.len() {
            if args[index - 1] == "--port" || args[index - 1] == "--coordinator-port" {
                args[index] = ports[0].to_string().into();
            }
            if args[index - 1] == "--local-listen-port" {
                args[index] = ports[1].to_string().into();
            }
        }
        app["environment"][dora_app::environment::DAEMON_PORT] = ports[1].to_string().into();
        services.insert(name, app.clone());
    }
    DoraApps {
        services,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_file_and_directory_binds_resolve_to_readable_host_files() {
        // Arrange
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("zenoh.json5");
        std::fs::write(&file, b"{}").unwrap();
        let service = serde_json::json!({"volumes":[
            {"type":"bind","source":file,"target":"/config/zenoh.json5"},
            {"type":"bind","source":directory.path(),"target":"/settings"}
        ]});
        for target in ["/config/zenoh.json5", "/settings/zenoh.json5"] {
            // Act
            let source = DoraApps::host_path(&service, target).unwrap();
            // Assert
            assert_eq!(std::fs::read(source).unwrap(), b"{}");
        }
    }
}
