//! Dora control steps operating on already-launched app units.
use crate::{
    Result, Scope, error,
    manager::{Manager, Unit},
};
use std::{path::PathBuf, time::Duration};
use workload_lifecycle::dora::port;

// Approved polling exception: Dora reports no daemon readiness event.
const STARTUP_CHECK_INTERVAL: Duration = Duration::from_millis(100);
const GRAPH_FILE: &str = "dataflow.yml";
pub(crate) const CLI_PREFIX: &str = "dora-cli-";

/// Prepare generated Dora programs before any unit starts; native units retain their own settings.
pub(crate) fn prepare(run: &crate::RunPlan, plan: &mut crate::SystemdPlan) -> Result<()> {
    let indices: Vec<_> = plan
        .programs
        .iter()
        .enumerate()
        .filter(|(_, p)| run.dora_apps.contains(&p.id))
        .map(|(i, _)| i)
        .collect();
    if indices.is_empty() {
        return Ok(());
    }
    if indices.len() != 2 || run.dora_apps.len() != 2 {
        return Err("generated Dora apps require one coordinator and one daemon".into());
    }
    let find = |role| {
        indices
            .iter()
            .copied()
            .find(|i| plan.programs[*i].argv.get(1).is_some_and(|arg| arg == role))
    };
    let coordinator = find("coordinator").ok_or("Dora coordinator app missing")?;
    let daemon = find("daemon").ok_or("Dora daemon app missing")?;
    let mut coordinator_args = plan.programs[coordinator].argv.clone();
    let mut daemon_args = plan.programs[daemon].argv.clone();
    workload_lifecycle::dora::resolve_ports(&mut coordinator_args, &mut daemon_args)?;
    let port = port(&daemon_args, "--local-listen-port")?;
    plan.programs[coordinator].argv = coordinator_args;
    plan.programs[daemon].argv = daemon_args;
    // Program-only callers without participants need no Zenoh file binding.
    if !plan.programs.iter().any(|p| !run.dora_apps.contains(&p.id)) {
        return Ok(());
    }
    let overlay =
        workload_lifecycle::dora::option(&plan.programs[daemon].argv, "--zenoh-config-overlay")?
            .to_owned();
    let binding = plan.programs[daemon]
        .binds
        .iter()
        .find(|b| b.target == overlay)
        .ok_or("Dora daemon needs its Zenoh config bound")?
        .clone();
    for program in &mut plan.programs {
        if run.dora_apps.contains(&program.id) {
            continue;
        }
        for (name, value) in dora_app::environment::settings(&program.id, port.into(), &overlay) {
            if name == dora_app::environment::NODE_ID
                && program.environment.iter().any(|e| e.name == name)
            {
                continue;
            }
            program.environment.retain(|e| e.name != name);
            program.environment.push(crate::EnvironmentVariable {
                name: name.into(),
                value,
            });
        }
        if program.binds.iter().any(|b| b.target == overlay) {
            return Err(format!(
                "{}: app mount overrides shared Dora config",
                program.id
            ));
        }
        program.binds.push(binding.clone());
    }
    Ok(())
}

/// Observed coordinator handle and dataflow identity; launch configuration remains in its unit.
pub(crate) struct Dora {
    coordinator: zbus_systemd::systemd1::ServiceProxy<'static>,
    executable: String,
    dataflow_file: String,
    run: String,
    commands: u32,
    pub registered: Option<String>,
}
impl Dora {
    /// Find the coordinator and daemon from the commands of the first-phase units.
    pub async fn from_units(units: &[Unit], run: &crate::RunPlan) -> Result<Self> {
        let mut coordinator = None;
        let mut daemon = None;
        for unit in units {
            let commands = unit.service.exec_start().await.map_err(error)?;
            let [command] = commands.as_slice() else {
                return Err("Dora app needs one ExecStart".into());
            };
            match command.1.get(1).map(String::as_str) {
                Some("coordinator") if coordinator.is_none() => {
                    coordinator = Some((unit.service.clone(), port(&command.1, "--port")?))
                }
                Some("daemon") if daemon.is_none() => {
                    daemon = Some((
                        port(&command.1, "--coordinator-port")?,
                        port(&command.1, "--local-listen-port")?,
                    ))
                }
                _ => return Err("Dora apps must launch one coordinator and one daemon".into()),
            }
        }
        let (coordinator, coordinator_port) = coordinator.ok_or("Dora coordinator app missing")?;
        let (target, daemon_port) = daemon.ok_or("Dora daemon app missing")?;
        if target != coordinator_port || daemon_port == coordinator_port {
            return Err("Dora app endpoints disagree".into());
        }
        // systemd resolves bare ExecStart names in the app's native execution context.
        let pid = coordinator.main_pid().await.map_err(error)?;
        if pid == 0 {
            return Err("Dora coordinator is not running".into());
        }
        let executable = tokio::fs::read_link(format!("/proc/{pid}/exe"))
            .await
            .map_err(error)?
            .into_os_string()
            .into_string()
            .map_err(|_| "Dora executable path is not UTF-8".to_string())?;
        Ok(Self {
            executable,
            coordinator,
            dataflow_file: run.dataflow_file.clone(),
            run: run.id.clone(),
            commands: 0,
            registered: None,
        })
    }

    /// Resolve the coordinator's working directory through its actual bind mounts.
    async fn directory(&self) -> Result<(PathBuf, PathBuf)> {
        let directory = PathBuf::from(self.coordinator.working_directory().await.map_err(error)?);
        if !directory.is_absolute() {
            return Err("Dora coordinator needs an absolute working directory".into());
        }
        let mut binds = self.coordinator.bind_paths().await.map_err(error)?;
        binds.extend(
            self.coordinator
                .bind_read_only_paths()
                .await
                .map_err(error)?,
        );
        let host = if let Some((source, target, _, _)) = binds
            .iter()
            .filter(|(_, target, _, _)| directory.starts_with(target))
            .max_by_key(|(_, target, _, _)| target.len())
        {
            PathBuf::from(source).join(directory.strip_prefix(target).map_err(error)?)
        } else {
            let root = self.coordinator.root_directory().await.map_err(error)?;
            if root.is_empty() {
                directory.clone()
            } else {
                PathBuf::from(root).join(directory.strip_prefix("/").map_err(error)?)
            }
        };
        if !host.is_dir() {
            return Err("Dora coordinator working directory is unavailable".into());
        }
        Ok((directory, host))
    }

    /// Run a workflow-owned CLI command with its explicit user and artifact bind.
    async fn cli(&mut self, manager: &Manager, scope: &Scope, args: &[&str]) -> Result<String> {
        self.commands += 1;
        let name = scope.unit(&format!("{CLI_PREFIX}{}", self.commands));
        let (target, directory) = self.directory().await?;
        let output = directory.join(format!("cli-{}.out", self.commands));
        let commands = self.coordinator.exec_start().await.map_err(error)?;
        let [command] = commands.as_slice() else {
            return Err("Dora coordinator command changed".into());
        };
        let endpoint = port(&command.1, "--port")?.to_string();
        let argv: Vec<String> = std::iter::once(self.executable.as_str())
            .chain(args.iter().copied())
            .chain(["--coordinator-port", endpoint.as_str()])
            .map(str::to_owned)
            .collect();
        let user = self.coordinator.user().await.map_err(error)?;
        let runtime = crate::Bind {
            source: directory.to_string_lossy().into_owned(),
            target: target.to_string_lossy().into_owned(),
            read_only: false,
        };
        manager
            .run_oneshot(scope, &name, &argv, &user, &output, &[runtime])
            .await?;
        tokio::fs::read_to_string(output).await.map_err(error)
    }

    /// Wait until the daemon reports itself connected to the coordinator.
    pub async fn await_daemon(&mut self, manager: &Manager, scope: &Scope) -> Result<()> {
        loop {
            if let Ok(status) = self
                .cli(manager, scope, &["status", "--format", "json"])
                .await
                && serde_json::from_str::<serde_json::Value>(&status)
                    .is_ok_and(|status| status["daemon"]["status"] == "running")
            {
                return Ok(());
            }
            tokio::time::sleep(STARTUP_CHECK_INTERVAL).await;
        }
    }

    /// Register the graph as the run's dataflow and record its identity.
    pub async fn register(&mut self, manager: &Manager, scope: &Scope) -> Result<()> {
        if self.dataflow_file.is_empty() {
            return Ok(());
        }
        let (directory, host) = self.directory().await?;
        let graph = directory.join(GRAPH_FILE);
        tokio::fs::copy(&self.dataflow_file, host.join(GRAPH_FILE))
            .await
            .map_err(error)?;
        let graph = graph.to_string_lossy().into_owned();
        let run = self.run.clone();
        self.cli(
            manager,
            scope,
            &["start", &graph, "--name", &run, "--detach"],
        )
        .await?;
        let listed = self
            .cli(
                manager,
                scope,
                &["list", "--format", "json", "--name", &run],
            )
            .await?;
        let entries: Vec<serde_json::Value> = listed
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()
            .map_err(error)?;
        match entries.as_slice() {
            [entry] if entry["name"] == run.as_str() && entry["status"] == "Running" => {
                self.registered = entry["uuid"].as_str().map(str::to_owned);
                self.registered
                    .as_ref()
                    .map(|_| ())
                    .ok_or_else(|| "Dora graph UUID missing".into())
            }
            _ => Err(format!("Dora graph identity is ambiguous: {listed}")),
        }
    }

    /// Ask the coordinator to stop every dataflow before its units stop.
    pub async fn down(&mut self, manager: &Manager, scope: &Scope) -> Result<()> {
        self.cli(manager, scope, &["down", "--force"])
            .await
            .map(|_| ())
    }
}
