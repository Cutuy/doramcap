//! Dora preparation and control over generated programs or separately supplied native units.
use crate::{
    Bind, EnvironmentVariable, Result, RunPlan, Scope, SystemdPlan, error,
    manager::{Manager, Unit},
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::AsyncReadExt;
use workload_lifecycle::{
    dora::{option, port},
    launch::{DoraRuntime, ResolvedFileBinding},
};
use zbus_systemd::systemd1::ServiceProxy;

// Approved polling exception: Dora reports no daemon readiness event.
const STARTUP_CHECK_INTERVAL: Duration = Duration::from_millis(100);
const GRAPH_FILE: &str = "dataflow.yml";
const HASH_BUFFER_BYTES: usize = 64 * 1024;
pub(crate) const CLI_PREFIX: &str = "dora-cli-";

fn participants(run: &RunPlan, plan: &SystemdPlan) -> Result<bool> {
    Ok(crate::app_ids(plan)?
        .iter()
        .any(|id| !run.dora_apps.contains(id)))
}

/// Resolve one app-visible path through its most specific declared bind.
fn bound_path(binds: &[Bind], path: &str) -> Option<String> {
    let path = Path::new(path);
    binds
        .iter()
        .filter(|bind| path.starts_with(&bind.target))
        .max_by_key(|bind| bind.target.len())
        .map(|bind| {
            Path::new(&bind.source)
                .join(path.strip_prefix(&bind.target).expect("matched bind"))
                .to_string_lossy()
                .into_owned()
        })
}

/// Fill generated Dora runtime facts once before startup; native units retain their own settings.
fn prepare_programs(run: &mut RunPlan, plan: &mut SystemdPlan) -> Result<()> {
    let indices: Vec<_> = plan
        .programs
        .iter()
        .enumerate()
        .filter(|(_, program)| run.dora_apps.contains(&program.id))
        .map(|(index, _)| index)
        .collect();
    if indices.is_empty() {
        return Ok(());
    }
    if indices.len() != 2 || run.dora_apps.len() != 2 {
        return Err("generated Dora apps require one coordinator and one daemon".into());
    }
    let find = |role| {
        indices.iter().copied().find(|index| {
            plan.programs[*index]
                .argv
                .get(1)
                .is_some_and(|arg| arg == role)
        })
    };
    let coordinator = find("coordinator").ok_or("Dora coordinator app missing")?;
    let daemon = find("daemon").ok_or("Dora daemon app missing")?;
    let mut coordinator_args = plan.programs[coordinator].argv.clone();
    let mut daemon_args = plan.programs[daemon].argv.clone();
    let automatic =
        workload_lifecycle::dora::resolve_ports(&mut coordinator_args, &mut daemon_args)?;
    let directory = &plan.programs[coordinator].working_directory;
    if !Path::new(directory).is_absolute() {
        return Err("Dora coordinator needs an absolute working directory".into());
    }
    let zenoh = if participants(run, plan)? {
        let target = option(&daemon_args, "--zenoh-config-overlay")?;
        Some(ResolvedFileBinding {
            source: bound_path(&plan.programs[daemon].binds, target)
                .ok_or("Dora daemon needs its Zenoh config bound")?,
            target: target.into(),
        })
    } else {
        None
    };
    run.dora_runtime = Some(DoraRuntime {
        coordinator_port: port(&coordinator_args, "--port")?.into(),
        daemon_port: port(&daemon_args, "--local-listen-port")?.into(),
        automatic_coordinator_port: automatic[0],
        automatic_daemon_port: automatic[1],
        runtime_directory: bound_path(&plan.programs[coordinator].binds, directory)
            .unwrap_or_else(|| directory.clone()),
        zenoh,
        ..Default::default()
    });
    plan.programs[coordinator].argv = coordinator_args;
    plan.programs[daemon].argv = daemon_args;
    Ok(())
}

/// Prepare generated endpoints and freeze a graph before any units start.
/// Sources already captured beneath this run's private artifacts retain their access metadata.
pub(crate) async fn prepare(run: &mut RunPlan, plan: &mut SystemdPlan) -> Result<()> {
    prepare_programs(run, plan)?;
    if !run.dataflow_file.is_empty() {
        let artifacts = tokio::fs::canonicalize(&run.artifacts)
            .await
            .map_err(error)?;
        let source = tokio::fs::canonicalize(&run.dataflow_file)
            .await
            .map_err(error)?;
        if !tokio::fs::metadata(&source).await.map_err(error)?.is_file() {
            return Err("Dora dataflow input must be a regular file".into());
        }
        if !source.starts_with(&artifacts) {
            let frozen = artifacts.join(GRAPH_FILE);
            tokio::fs::copy(&source, &frozen).await.map_err(error)?;
            run.dataflow_file = frozen.to_string_lossy().into_owned();
        } else {
            run.dataflow_file = source.to_string_lossy().into_owned();
        }
    }
    Ok(())
}

/// Approved bridge values; native project units receive the captured host source.
/// Their configured user, root directory and mounts must make that source readable.
pub(crate) fn environment(
    runtime: &DoraRuntime,
    id: &str,
    native: bool,
) -> Result<Vec<EnvironmentVariable>> {
    let zenoh = runtime
        .zenoh
        .as_ref()
        .ok_or("Dora participant configuration binding missing")?;
    let overlay = if native { &zenoh.source } else { &zenoh.target };
    Ok(
        dora_app::environment::settings(id, runtime.daemon_port, overlay)
            .into_iter()
            .map(|(name, value)| EnvironmentVariable {
                name: name.into(),
                value,
            })
            .collect(),
    )
}

/// Attach one later phase to the prepared shared runtime; native project units keep their filesystem.
pub(crate) fn attach(run: &RunPlan, phase: &mut SystemdPlan) -> Result<()> {
    let Some(runtime) = &run.dora_runtime else {
        return Ok(());
    };
    if phase.programs.is_empty() && phase.app_units.is_empty() {
        return Ok(());
    }
    let zenoh = runtime
        .zenoh
        .as_ref()
        .ok_or("Dora participant configuration binding missing")?;
    let add = |id: &str, binds: &mut Vec<Bind>| -> Result<()> {
        if binds.iter().any(|bind| bind.target == zenoh.target) {
            return Err(format!("{id}: app mount overrides shared Dora config"));
        }
        binds.push(Bind {
            source: zenoh.source.clone(),
            target: zenoh.target.clone(),
            read_only: true,
        });
        Ok(())
    };
    for program in &mut phase.programs {
        for variable in environment(runtime, &program.id, false)? {
            if variable.name == dora_app::environment::NODE_ID
                && program
                    .environment
                    .iter()
                    .any(|existing| existing.name == variable.name)
            {
                continue;
            }
            program
                .environment
                .retain(|existing| existing.name != variable.name);
            program.environment.push(variable);
        }
        add(&program.id, &mut program.binds)?;
    }
    for app in &mut phase.app_units {
        if !crate::plain_path(&zenoh.target) {
            return Err(format!(
                "{}: native app units require a plain Dora config target: {:?}",
                app.id, zenoh.target
            ));
        }
        add(&app.id, &mut app.binds)?;
    }
    Ok(())
}

/// Native coordinator context and graph control ownership; neutral runtime facts stay in RunPlan.
pub(crate) struct Dora {
    coordinator: ServiceProxy<'static>,
    executable: String,
    directory: String,
    commands: u32,
    pub registered: Option<String>,
}
impl Dora {
    /// Observe native Dora units once and fill any runtime facts not supplied by generated programs.
    pub async fn from_units(units: &[Unit], run: &mut RunPlan, plan: &SystemdPlan) -> Result<Self> {
        let mut coordinator = None;
        let mut daemon = None;
        for unit in units {
            let commands = unit.service.exec_start().await.map_err(error)?;
            let [command] = commands.as_slice() else {
                return Err("Dora app needs one ExecStart".into());
            };
            match command.1.get(1).map(String::as_str) {
                Some("coordinator") if coordinator.is_none() => {
                    coordinator = Some((unit.service.clone(), command.1.clone()))
                }
                Some("daemon") if daemon.is_none() => {
                    daemon = Some((unit.service.clone(), command.1.clone()))
                }
                _ => return Err("Dora apps must launch one coordinator and one daemon".into()),
            }
        }
        let (coordinator, coordinator_args) = coordinator.ok_or("Dora coordinator app missing")?;
        let (daemon, daemon_args) = daemon.ok_or("Dora daemon app missing")?;
        let directory = coordinator.working_directory().await.map_err(error)?;
        if !Path::new(&directory).is_absolute() {
            return Err("Dora coordinator needs an absolute working directory".into());
        }
        if run.dora_runtime.is_none() {
            let coordinator_port = port(&coordinator_args, "--port")?;
            let daemon_port = port(&daemon_args, "--local-listen-port")?;
            if port(&daemon_args, "--coordinator-port")? != coordinator_port
                || daemon_port == coordinator_port
            {
                return Err("Dora app endpoints disagree".into());
            }
            let zenoh = if participants(run, plan)? {
                let target = option(&daemon_args, "--zenoh-config-overlay")?;
                Some(ResolvedFileBinding {
                    source: unit_path(&daemon, target).await?,
                    target: target.into(),
                })
            } else {
                None
            };
            run.dora_runtime = Some(DoraRuntime {
                coordinator_port: coordinator_port.into(),
                daemon_port: daemon_port.into(),
                runtime_directory: unit_path(&coordinator, &directory).await?,
                zenoh,
                ..Default::default()
            });
        }
        if !Path::new(&runtime(run)?.runtime_directory).is_dir() {
            return Err("Dora coordinator working directory is unavailable".into());
        }
        // Native units resolve bare executable names in their own execution context.
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
            directory,
            commands: 0,
            registered: None,
        })
    }

    /// Run a workflow-owned CLI command using the shared endpoint and explicit native execution context.
    async fn cli(
        &mut self,
        manager: &Manager,
        scope: &Scope,
        run: &RunPlan,
        args: &[&str],
    ) -> Result<String> {
        self.commands += 1;
        let name = scope.unit(&format!("{CLI_PREFIX}{}", self.commands));
        let runtime = runtime(run)?;
        let output =
            Path::new(&runtime.runtime_directory).join(format!("cli-{}.out", self.commands));
        let endpoint = runtime.coordinator_port.to_string();
        let argv: Vec<String> = std::iter::once(self.executable.as_str())
            .chain(args.iter().copied())
            .chain(["--coordinator-port", endpoint.as_str()])
            .map(str::to_owned)
            .collect();
        let user = self.coordinator.user().await.map_err(error)?;
        let mut binds = vec![Bind {
            source: runtime.runtime_directory.clone(),
            target: self.directory.clone(),
            read_only: false,
        }];
        if args.first() == Some(&"start")
            && let Some(graph) = &runtime.dataflow
        {
            binds.push(Bind {
                source: graph.source.clone(),
                target: graph.target.clone(),
                read_only: true,
            });
        }
        manager
            .run_oneshot(scope, &name, &argv, &user, &output, &binds)
            .await?;
        tokio::fs::read_to_string(output).await.map_err(error)
    }

    /// Wait until the daemon reports itself connected to the coordinator.
    pub async fn await_daemon(
        &mut self,
        manager: &Manager,
        scope: &Scope,
        run: &RunPlan,
    ) -> Result<()> {
        loop {
            if let Ok(status) = self
                .cli(manager, scope, run, &["status", "--format", "json"])
                .await
                && serde_json::from_str::<serde_json::Value>(&status)
                    .is_ok_and(|status| status["daemon"]["status"] == "running")
            {
                return Ok(());
            }
            tokio::time::sleep(STARTUP_CHECK_INTERVAL).await;
        }
    }

    /// Register the captured merged graph and retain the actual shared source/target and content identity.
    pub async fn register(
        &mut self,
        manager: &Manager,
        scope: &Scope,
        run: &mut RunPlan,
    ) -> Result<()> {
        if run.dataflow_file.is_empty() {
            return Ok(());
        }
        let source = run.dataflow_file.clone();
        let target = Path::new(&self.directory)
            .join(GRAPH_FILE)
            .to_string_lossy()
            .into_owned();
        let destination = Path::new(&runtime(run)?.runtime_directory).join(GRAPH_FILE);
        if Path::new(&source) != destination {
            tokio::fs::copy(&source, &destination)
                .await
                .map_err(error)?;
        }
        let hash = hash_file(Path::new(&source)).await?;
        let prepared = run.dora_runtime.as_mut().expect("prepared Dora runtime");
        prepared.dataflow = Some(ResolvedFileBinding {
            source,
            target: target.clone(),
        });
        prepared.dataflow_sha256 = hash;
        let id = run.id.clone();
        self.cli(
            manager,
            scope,
            run,
            &["start", &target, "--name", &id, "--detach"],
        )
        .await?;
        let listed = self
            .cli(
                manager,
                scope,
                run,
                &["list", "--format", "json", "--name", &id],
            )
            .await?;
        let entries: Vec<serde_json::Value> = listed
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()
            .map_err(error)?;
        match entries.as_slice() {
            [entry] if entry["name"] == id.as_str() && entry["status"] == "Running" => {
                self.registered = entry["uuid"].as_str().map(str::to_owned);
                self.registered
                    .as_ref()
                    .map(|_| ())
                    .ok_or_else(|| "Dora graph UUID missing".into())
            }
            _ => Err(format!("Dora graph identity is ambiguous: {listed}")),
        }
    }

    /// Stop every dataflow before its coordinator and daemon units stop.
    pub async fn down(&mut self, manager: &Manager, scope: &Scope, run: &RunPlan) -> Result<()> {
        self.cli(manager, scope, run, &["down", "--force"])
            .await
            .map(|_| ())
    }
}

fn runtime(run: &RunPlan) -> Result<&DoraRuntime> {
    run.dora_runtime
        .as_ref()
        .ok_or_else(|| "prepared Dora runtime missing".into())
}

/// Resolve a native service path once through its actual binds and root directory.
async fn unit_path(service: &ServiceProxy<'_>, path: &str) -> Result<String> {
    let mut binds = service.bind_paths().await.map_err(error)?;
    binds.extend(service.bind_read_only_paths().await.map_err(error)?);
    let binds: Vec<Bind> = binds
        .into_iter()
        .map(|(source, target, _, _)| Bind {
            source,
            target,
            read_only: false,
        })
        .collect();
    if let Some(host) = bound_path(&binds, path) {
        return Ok(host);
    }
    let root = service.root_directory().await.map_err(error)?;
    if root.is_empty() {
        return Ok(path.into());
    }
    Ok(PathBuf::from(root)
        .join(Path::new(path).strip_prefix("/").map_err(error)?)
        .to_string_lossy()
        .into_owned())
}

async fn hash_file(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await.map_err(error)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; HASH_BUFFER_BYTES];
    loop {
        let read = file.read(&mut buffer).await.map_err(error)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hex::encode(hash.finalize()))
}
