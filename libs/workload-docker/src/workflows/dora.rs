//! Dora control over ordinary, resolved Compose app definitions.
use super::experiment::ExperimentOwner;
use crate::compose::{error::invalid, *};
use dora_control::DoraCoordinatorClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path, time::Duration};
use tokio::time::Instant;

/// Native Compose Dora service definitions used by the Docker adapter.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DoraApps {
    pub services: BTreeMap<String, Value>,
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
    pub(super) async fn start_runtime(&mut self) -> Result<()> {
        if !self.apps.enabled() {
            return Ok(());
        }
        let deadline = Instant::now() + Duration::from_secs(self.startup_seconds);
        self.startup_deadline = Some(deadline);
        let app = self.apps.coordinator()?.1;
        let argv = DoraApps::argv(app)?;
        let endpoint = workload_lifecycle::dora::endpoint(
            &argv,
            app["environment"]["DORA_COORDINATOR_INTERFACE"].as_str(),
        )
        .map_err(invalid)?;
        let authenticated = argv.iter().any(|arg| arg == "--auth");
        let expected_token = app["environment"]["DORA_AUTH_TOKEN"]
            .as_str()
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        let directory = app["working_dir"]
            .as_str()
            .ok_or_else(|| invalid("Dora coordinator needs working_dir"))?;
        let source = DoraApps::host_path(app, directory)?;
        tokio::time::timeout_at(
            deadline,
            self.adapter.project(&self.plan).start(
                &self.apps.services.keys().cloned().collect::<Vec<_>>(),
                self.startup_seconds,
            ),
        )
        .await
        .map_err(|_| invalid("Dora native startup deadline expired"))??;
        let token = workload_lifecycle::dora::token(Path::new(&source), authenticated, deadline)
            .await
            .map_err(invalid)?;
        if authenticated
            && expected_token
                .as_deref()
                .is_some_and(|value| token.as_deref() != Some(value))
        {
            return Err(invalid(
                "native Dora token override disagrees with the run-local credential",
            ));
        }
        // Retain identity before any graph mutation can be awaited or cancelled.
        self.client = Some(DoraCoordinatorClient::new(endpoint, token));
        self.client
            .as_mut()
            .unwrap()
            .wait_for_daemon(deadline)
            .await
            .map_err(|e| invalid(e.to_string()))?;
        tokio::time::timeout_at(deadline, self.inspect_runtime())
            .await
            .map_err(|_| invalid("Dora native inspection deadline expired"))?
    }
    pub(super) async fn inspect_runtime(&self) -> Result<()> {
        if !self.apps.enabled() {
            return Ok(());
        }
        for (name, port) in [
            (
                self.apps.coordinator()?.0,
                self.dora_runtime()?.coordinator_port as u16,
            ),
            (
                self.apps.daemon()?.0,
                self.dora_runtime()?.daemon_port as u16,
            ),
        ] {
            self.adapter
                .project(&self.plan)
                .endpoint(name, "host", port)
                .await?;
        }
        Ok(())
    }
    /// Workloads must have drained and stopped before native graph/runtime teardown.
    pub(super) async fn stop_runtime(&mut self) -> Result<Shutdown> {
        if !self.apps.enabled() {
            return Ok(Shutdown {
                stopped: true,
                ..Default::default()
            });
        }
        let deadline = self.cleanup_deadline();
        let control_error = if let Some(client) = &mut self.client {
            client.shutdown(deadline).await.err()
        } else {
            None
        };
        let mut stopped = self
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
        if let Some(error) = control_error {
            stopped
                .diagnostics
                .push(invalid(error.to_string()).diagnostic("Dora control shutdown"));
        }
        Ok(stopped)
    }
}

#[cfg(test)]
pub(crate) fn fixture(
    root: &str,
    ports: [u16; 2],
) -> (DoraApps, workload_lifecycle::launch::DoraRuntime) {
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
    let mut apps = DoraApps { services };
    let runtime = super::shared_dora::resolve(&mut apps, format!("{root}/dora/runtime"))
        .unwrap()
        .unwrap();
    (apps, runtime)
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
