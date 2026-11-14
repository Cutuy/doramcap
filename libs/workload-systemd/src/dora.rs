//! Dora preparation and control over generated programs.
use crate::{Bind, EnvironmentVariable, Result, RunPlan, SystemdPlan, error, manager::Unit};
use dora_control::DoraCoordinatorClient;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::time::Instant;
use workload_lifecycle::{
    dora::{option, port},
    launch::{DoraRuntime, ResolvedFileBinding},
};
use zbus_systemd::systemd1::ServiceProxy;

const GRAPH_FILE: &str = "dataflow.yml";

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
            let relative = path.strip_prefix(&bind.target).expect("matched bind");
            if relative.as_os_str().is_empty() {
                bind.source.clone()
            } else {
                Path::new(&bind.source)
                    .join(relative)
                    .to_string_lossy()
                    .into_owned()
            }
        })
}

/// Fill generated Dora runtime facts from captured inputs before startup.
pub(crate) fn prepare(run: &mut RunPlan, plan: &mut SystemdPlan) -> Result<()> {
    let indices: Vec<_> = plan
        .programs
        .iter()
        .enumerate()
        .filter(|(_, program)| run.dora_apps.contains(&program.id))
        .map(|(index, _)| index)
        .collect();
    if run.dora_apps.is_empty() {
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
    if phase.programs.is_empty() {
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
    Ok(())
}

/// Native coordinator context and graph control ownership; neutral runtime facts stay in RunPlan.
pub(crate) struct Dora {
    pub client: DoraCoordinatorClient,
    coordinator: ServiceProxy<'static>,
    directory: String,
}

impl Dora {
    /// Observe the generated Dora services using the prepared runtime facts.
    pub async fn from_units(units: &[Unit], run: &RunPlan, deadline: Instant) -> Result<Self> {
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
        let (daemon, _) = daemon.ok_or("Dora daemon app missing")?;
        let directory = coordinator.working_directory().await.map_err(error)?;
        if !Path::new(&directory).is_absolute() {
            return Err("Dora coordinator needs an absolute working directory".into());
        }
        if !Path::new(&runtime(run)?.runtime_directory).is_dir() {
            return Err("Dora coordinator working directory is unavailable".into());
        }
        use std::os::unix::fs::MetadataExt;
        let network = tokio::fs::metadata("/proc/self/ns/net")
            .await
            .map_err(error)?;
        for (role, service) in [("coordinator", &coordinator), ("daemon", &daemon)] {
            let pid = service.main_pid().await.map_err(error)?;
            if pid == 0 {
                return Err(format!("Dora {role} is not running"));
            }
            let actual = tokio::fs::metadata(format!("/proc/{pid}/ns/net"))
                .await
                .map_err(|e| format!("Dora {role} network namespace is unavailable: {e}"))?;
            if (actual.dev(), actual.ino()) != (network.dev(), network.ino()) {
                return Err(format!("Dora {role} must share the executor host network"));
            }
        }
        let environment = coordinator.environment().await.map_err(error)?;
        let native_value = |name: &str| {
            environment
                .iter()
                .find_map(|value| value.strip_prefix(&format!("{name}=")))
        };
        let endpoint = workload_lifecycle::dora::endpoint(
            &coordinator_args,
            native_value("DORA_COORDINATOR_INTERFACE"),
        )?;
        if endpoint.port() as u32 != runtime(run)?.coordinator_port {
            return Err("native Dora endpoint differs from the prepared runtime".into());
        }
        let authenticated = coordinator_args.iter().any(|arg| arg == "--auth");
        let token = workload_lifecycle::dora::token(
            Path::new(&runtime(run)?.runtime_directory),
            authenticated,
            deadline,
        )
        .await?;
        if authenticated
            && native_value("DORA_AUTH_TOKEN")
                .filter(|value| !value.is_empty())
                .is_some_and(|value| token.as_deref() != Some(value))
        {
            return Err(
                "native Dora token override disagrees with the run-local credential".into(),
            );
        }
        Ok(Self {
            client: DoraCoordinatorClient::new(endpoint, token),
            coordinator,
            directory,
        })
    }

    /// Shut down the attempted graph only while its native coordinator remains owned and running.
    /// With no Start attempt, close transport and let native unit teardown stop the runtime.
    pub async fn down(&mut self, deadline: Instant) -> Result<()> {
        let shutdown = async {
            if self.client.graph().is_none() {
                self.client.disconnect();
                return Ok(());
            }
            if self.coordinator.main_pid().await.map_err(error)? == 0 {
                self.client.disconnect();
                return Err("native Dora coordinator exited before control shutdown".into());
            }
            self.client.shutdown(deadline).await.map_err(error)
        };
        tokio::time::timeout_at(deadline, shutdown)
            .await
            .map_err(|_| "Dora control cleanup deadline expired".to_owned())?
    }

    /// Register the captured merged graph and retain the actual shared source/target and content identity.
    pub async fn register(
        &mut self,
        run: &mut RunPlan,
        expected_sha256: &str,
        deadline: Instant,
    ) -> Result<()> {
        if run.dataflow_file.is_empty() {
            return Ok(());
        }
        if self.client.graph().is_some() {
            return Err("Dora graph start already attempted".into());
        }
        if self.coordinator.main_pid().await.map_err(error)? == 0 {
            return Err("Dora coordinator exited before Start".into());
        }
        let source = run.dataflow_file.clone();
        let bytes = tokio::fs::read(&source).await.map_err(error)?;
        let hash = hex::encode(Sha256::digest(&bytes));
        if hash != expected_sha256 {
            return Err("frozen Dora graph changed".into());
        }
        let graph = workload_lifecycle::dora::graph(&source, &bytes)?;
        let destination = Path::new(&runtime(run)?.runtime_directory).join(GRAPH_FILE);
        if Path::new(&source) != destination {
            tokio::fs::copy(&source, &destination)
                .await
                .map_err(error)?;
        }
        if hex::encode(Sha256::digest(
            tokio::fs::read(&destination).await.map_err(error)?,
        )) != hash
        {
            return Err("frozen Dora graph changed".into());
        }
        let session = dora_control::protocol::SessionId::generate();
        let start = workload_lifecycle::dora::start(
            graph,
            run.id.clone(),
            self.directory.clone().into(),
            session,
            None,
        )?;
        let root = Path::new(&runtime(run)?.runtime_directory);
        tokio::fs::create_dir_all(root.join("out"))
            .await
            .map_err(error)?;
        tokio::fs::write(
            root.join(workload_lifecycle::dora::SESSION_FILE),
            serde_json::to_vec_pretty(&workload_lifecycle::dora::session_artifact(session))
                .map_err(error)?,
        )
        .await
        .map_err(error)?;
        let prepared = run.dora_runtime.as_mut().expect("prepared Dora runtime");
        prepared.dataflow = Some(ResolvedFileBinding {
            source,
            target: Path::new(&self.directory)
                .join(GRAPH_FILE)
                .to_string_lossy()
                .into_owned(),
        });
        prepared.dataflow_sha256 = hash;
        self.client
            .start_graph(start, deadline)
            .await
            .map_err(error)?;
        self.client.inspect_graph(deadline).await.map_err(error)
    }
}

fn runtime(run: &RunPlan) -> Result<&DoraRuntime> {
    run.dora_runtime
        .as_ref()
        .ok_or_else(|| "prepared Dora runtime missing".into())
}
