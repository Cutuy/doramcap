//! Service manager calls over the system bus, typed for this crate's units.
use crate::{Program, Result, Scope, error};
use futures_util::StreamExt;
use std::time::Duration;
use zbus::{
    OwnedVariant, Variant,
    proxy::{CacheProperties, PropertyStream},
};
use zbus_systemd::systemd1::{ManagerProxy, ServiceProxy, UnitProxy};

/// States in which a unit runs no processes.
const ENDED_STATES: [&str; 2] = ["inactive", "failed"];
/// Microseconds per second, for systemd's `*USec` properties.
const MICROS_PER_SECOND: u64 = 1_000_000;
/// `MS_REC`: bind mounts include submounts, as systemd's own `BindPaths=` does.
const MS_REC: u64 = 16384;
/// Host directories every generated unit sees read-only inside its private root.
const SYSTEM_PATHS: [&str; 6] = ["/usr", "/etc", "/bin", "/sbin", "/lib", "/lib64"];
/// systemd sets PATH to ExecSearchPath, so app units keep the usual system directories after
/// the binaries directory.
const DEFAULT_SEARCH_PATH: [&str; 6] = [
    "/usr/local/sbin",
    "/usr/local/bin",
    "/usr/sbin",
    "/usr/bin",
    "/sbin",
    "/bin",
];
/// Scope subdirectory holding each generated unit's private root.
const ROOTS: &str = "roots";
/// Plain runtime source names beside an app root; normal root cleanup removes these links.
const BIND_SOURCES_SUFFIX: &str = ".bind-sources";
/// Drop-in placing an adopted unit in the run slice.
const STOP_REPORT_MARGIN: Duration = Duration::from_secs(5);
/// Bits in each byte of a CPU mask.
const BITS_PER_BYTE: usize = 8;
/// Drop-in filename.
const SLICE_DROP_IN: &str = "10-doramcap-slice.conf";

/// One unit this run started, with its state subscription.
pub(crate) struct Unit {
    pub name: String,
    pub service: ServiceProxy<'static>,
    /// Wakes on state changes; needs the property cache of `_watcher`.
    states: PropertyStream<'static, String>,
    _watcher: UnitProxy<'static>,
    /// Uncached reads, so a value from before the start never looks current.
    unit: UnitProxy<'static>,
}
impl Unit {
    /// Wait until the unit runs no processes.
    pub async fn ended(&mut self) -> Result<()> {
        loop {
            let state = self.unit.active_state().await.map_err(error)?;
            if ENDED_STATES.contains(&state.as_str()) {
                return Ok(());
            }
            self.states
                .next()
                .await
                .ok_or_else(|| format!("{}: state notifications ended", self.name))?;
        }
    }
}

/// A submitted start job and its subscription, retained until the group waits.
pub(crate) struct StartJob {
    name: String,
    job: zbus::OwnedObjectPath,
    events: zbus_systemd::systemd1::JobRemovedStream,
}

/// The system service manager, subscribed to unit signals.
pub(crate) struct Manager {
    connection: zbus::Connection,
    manager: ManagerProxy<'static>,
}
impl Manager {
    pub async fn connect() -> Result<Self> {
        let connection = zbus::Connection::system().await.map_err(error)?;
        let manager = ManagerProxy::new(&connection).await.map_err(error)?;
        // Subscribe before any start so no state change is missed.
        manager.subscribe().await.map_err(error)?;
        Ok(Self {
            connection,
            manager,
        })
    }

    /// Load `name` and subscribe to its state before it starts.
    pub(crate) async fn watch(&self, name: &str) -> Result<Unit> {
        let path = self.manager.load_unit(name.into()).await.map_err(error)?;
        let unit = UnitProxy::builder(&self.connection)
            .path(path.clone())
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .map_err(error)?;
        let service = ServiceProxy::builder(&self.connection)
            .path(path)
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .map_err(error)?;
        // Subscribe before reading state; zbus streams property changes only from a caching proxy.
        let watcher = UnitProxy::builder(&self.connection)
            .path(unit.inner().path().to_owned())
            .build()
            .await
            .map_err(error)?;
        let states = watcher.receive_active_state_changed().await;
        Ok(Unit {
            name: name.into(),
            service,
            states,
            _watcher: watcher,
            unit,
        })
    }

    /// Await the end of start job `job`; a result other than `done` failed the start.
    async fn started(
        &self,
        name: &str,
        jobs: &mut zbus_systemd::systemd1::JobRemovedStream,
        job: zbus::OwnedObjectPath,
    ) -> Result<()> {
        while let Some(signal) = jobs.next().await {
            let removed = signal.args().map_err(error)?;
            if *removed.job == *job {
                return match removed.result.as_str() {
                    "done" => Ok(()),
                    result => Err(format!("{name} failed to start: {result}")),
                };
            }
        }
        Err(format!("{name}: job notifications ended"))
    }

    /// Start `program` as a referenced transient service in the scope's slice and
    /// return its submitted start job.
    pub async fn start_program(
        &self,
        scope: &Scope,
        run: &str,
        program: &Program,
        stop_seconds: u64,
    ) -> Result<StartJob> {
        let name = scope.unit(&program.id);
        let mut properties = transient_properties(scope, run, program, stop_seconds)?;
        properties.extend(
            private_root(
                scope,
                &name,
                &program.argv[0],
                &program.working_directory,
                &program.binds,
            )
            .await?,
        );
        let jobs = self.manager.receive_job_removed().await.map_err(error)?;
        let job = self
            .manager
            .start_transient_unit(name.clone(), "fail".into(), properties, vec![])
            .await
            .map_err(error)?;
        Ok(StartJob {
            name,
            job,
            events: jobs,
        })
    }

    /// Run `argv` to completion as a oneshot unit in the scope's slice, writing its
    /// standard output to `output`. Errors: a start or exit other than success.
    pub async fn run_oneshot(
        &self,
        scope: &Scope,
        name: &str,
        argv: &[String],
        user: &str,
        output: &std::path::Path,
        binds: &[crate::Bind],
    ) -> Result<()> {
        let mut properties = vec![
            (
                "Description".into(),
                value(format!("Doramcap command {name}"))?,
            ),
            ("Slice".into(), value(scope.slice.clone())?),
            ("Type".into(), value("oneshot")?),
            (
                "ExecStart".into(),
                value(vec![(argv[0].clone(), argv.to_vec(), false)])?,
            ),
            (
                "StandardOutputFileToTruncate".into(),
                value(output.to_string_lossy().into_owned())?,
            ),
        ];
        if !user.is_empty() {
            properties.push(("User".into(), value(user.to_owned())?));
        }
        properties.extend(private_root(scope, name, &argv[0], "", binds).await?);
        let mut jobs = self.manager.receive_job_removed().await.map_err(error)?;
        let job = self
            .manager
            .start_transient_unit(name.into(), "fail".into(), properties, vec![])
            .await
            .map_err(error)?;
        // A oneshot start job ends when its process exits.
        let result = self.started(name, &mut jobs, job).await;
        if result.is_err() {
            self.release(name).await;
        }
        result
    }

    /// Link copies of repository unit files into the runtime unit directory and
    /// place each in the scope's slice. Call [`Self::start_adopted`] after this.
    pub async fn adopt(
        &self,
        scope: &Scope,
        files: &[String],
        apps: &[crate::AppUnit],
        binaries: &str,
        stop_seconds: u64,
        runtime: Option<&workload_lifecycle::launch::DoraRuntime>,
    ) -> Result<Vec<String>> {
        let units = scope.directory.join("units");
        tokio::fs::create_dir_all(&units).await.map_err(error)?;
        let mut names = Vec::new();
        let mut copies = Vec::new();
        let user = files.iter().map(|file| (file, None));
        let apps = apps.iter().map(|app| (&app.file, Some(app)));
        for (file, app) in user.chain(apps) {
            let id = match app {
                Some(app) => app.id.as_str(),
                None => crate::unit_stem(file)?,
            };
            let name = scope.unit(id);
            let copy = units.join(&name);
            // Earlier defaults yield to any stop grace declared by the source unit.
            let environment = runtime
                .map(|runtime| crate::dora::environment(runtime, id, app.is_none()))
                .transpose()?
                .unwrap_or_default();
            let mut contents = format!("[Service]\nTimeoutStopSec={stop_seconds}s\n").into_bytes();
            // The unit's own node identity takes precedence over the default app ID.
            for variable in environment
                .iter()
                .filter(|variable| variable.name == dora_app::environment::NODE_ID)
            {
                contents.extend(environment_line(variable)?.as_bytes());
            }
            contents.extend(tokio::fs::read(file).await.map_err(error)?);
            tokio::fs::write(&copy, contents).await.map_err(error)?;
            let drop_in =
                std::path::Path::new(crate::SYSTEMD_RUNTIME_UNITS).join(format!("{name}.d"));
            tokio::fs::create_dir_all(&drop_in).await.map_err(error)?;
            let mut run = format!("[Service]\nSlice={}\n", scope.slice);
            for variable in environment
                .iter()
                .filter(|variable| variable.name != dora_app::environment::NODE_ID)
            {
                run.push_str(&environment_line(variable)?);
            }
            // Native project units expose the host configuration path through their own filesystem.
            if let Some(app) = app {
                run.push_str(&app_root(scope, &name, &app.binds, binaries).await?);
            }
            tokio::fs::write(drop_in.join(SLICE_DROP_IN), run)
                .await
                .map_err(error)?;
            copies.push(copy.to_string_lossy().into_owned());
            names.push(name);
        }
        if !copies.is_empty() {
            self.manager
                .link_unit_files(copies, true, true)
                .await
                .map_err(error)?;
            self.manager.reload().await.map_err(error)?;
        }
        Ok(names)
    }

    /// Reference and submit an adopted unit start; the caller waits after submission.
    pub async fn start_adopted(&self, name: &str) -> Result<StartJob> {
        self.manager.ref_unit(name.into()).await.map_err(error)?;
        let jobs = self.manager.receive_job_removed().await.map_err(error)?;
        let job = self
            .manager
            .start_unit(name.into(), "replace".into())
            .await
            .map_err(error)?;
        Ok(StartJob {
            name: name.into(),
            job,
            events: jobs,
        })
    }

    /// Await one already-submitted job after the whole group has been submitted.
    pub async fn wait_started(&self, mut job: StartJob) -> Result<()> {
        self.started(&job.name, &mut job.events, job.job).await
    }

    pub async fn stop(&self, name: &str) -> Result<()> {
        self.manager
            .stop_unit(name.into(), "replace".into())
            .await
            .map(|_| ())
            .map_err(error)
    }

    /// Release this run's reference so systemd may collect the unit.
    pub async fn release(&self, name: &str) {
        let _ = self.manager.unref_unit(name.into()).await;
        // A unit that ended cleanly may already be collected.
        let _ = self.manager.reset_failed_unit(name.into()).await;
    }

    /// Stop one service and observe termination using its effective systemd timeout.
    pub async fn stop_and_wait(&self, unit: &mut Unit) -> Result<()> {
        let state = unit.unit.active_state().await.map_err(error)?;
        // An inactive unit can still have a queued start that must be cancelled.
        if ENDED_STATES.contains(&state.as_str()) && unit.unit.job().await.map_err(error)?.0 == 0 {
            return Ok(());
        }
        let budget = unit
            .service
            .timeout_stop_u_sec()
            .await
            .map_err(error)
            .and_then(stop_budget);
        let started = tokio::time::Instant::now();
        self.stop(&unit.name).await?;
        let budget = budget.map_err(|e| format!("{}: {e}", unit.name))?;
        let deadline = started
            .checked_add(budget)
            .ok_or_else(|| format!("{}: stop deadline overflow", unit.name))?;
        tokio::time::timeout_at(deadline, unit.ended())
            .await
            .map_err(|_| format!("{}: shutdown incomplete after {budget:?}", unit.name))?
    }

    /// Inspect owned units and runtime files without stopping or deleting anything.
    pub async fn remaining(&self, scope: &Scope) -> Result<Vec<String>> {
        let mut remaining: Vec<_> = self
            .units(&format!("{}*", scope.prefix))
            .await?
            .into_iter()
            .filter(|(_, load, active)| load != "not-found" || active != "inactive")
            .map(|(name, ..)| name)
            .collect();
        for path in entries(&scope.directory).await? {
            if path.file_name().is_some_and(|name| name != "lock") {
                remaining.push(path.display().to_string());
            }
        }
        remaining.extend(
            runtime_files(scope)
                .await?
                .iter()
                .map(|p| p.display().to_string()),
        );
        Ok(remaining)
    }

    /// Explicit cleanup: stop owned services, then remove their runtime files.
    pub async fn reset(&self, scope: &Scope) -> Result<Vec<String>> {
        let loaded = self.units(&format!("{}*", scope.prefix)).await?;
        let mut units = Vec::new();
        for (name, load, _) in &loaded {
            if name.ends_with(".service") && load != "not-found" {
                units.push(self.watch(name).await?);
            }
        }
        let results =
            futures_util::future::join_all(units.iter_mut().map(|unit| self.stop_and_wait(unit)))
                .await;
        for result in results {
            result?;
        }
        // Slice teardown has no service work remaining; wait for its stop job.
        if loaded
            .iter()
            .any(|(name, load, _)| name == &scope.slice && load != "not-found")
        {
            let mut jobs = self.manager.receive_job_removed().await.map_err(error)?;
            let job = self
                .manager
                .stop_unit(scope.slice.clone(), "replace".into())
                .await
                .map_err(error)?;
            tokio::time::timeout(
                STOP_REPORT_MARGIN,
                self.started(&scope.slice, &mut jobs, job),
            )
            .await
            .map_err(|_| format!("{}: slice stop not confirmed", scope.slice))??;
        }
        for (name, ..) in loaded {
            self.release(&name).await;
        }
        self.unlink(scope).await?;
        remove_path(&scope.directory.join(crate::ACTIVE_RUN)).await?;
        self.remaining(scope).await
    }

    /// Remove owned runtime links, drop-ins and copies; propagate filesystem errors.
    async fn unlink(&self, scope: &Scope) -> Result<()> {
        for path in runtime_files(scope).await? {
            remove_path(&path).await?;
        }
        remove_path(&scope.directory.join("units")).await?;
        remove_path(&scope.directory.join(ROOTS)).await?;
        self.manager.reload().await.map_err(error)
    }

    /// Loaded units matching `pattern`: name, load state and active state.
    async fn units(&self, pattern: &str) -> Result<Vec<(String, String, String)>> {
        Ok(self
            .manager
            .list_units_by_patterns(vec![], vec![pattern.into()])
            .await
            .map_err(error)?
            .into_iter()
            .map(|unit| (unit.0, unit.2, unit.3))
            .collect())
    }
}

/// Absence is clean; all other traversal failures are errors.
async fn entries(directory: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
    let mut reader = match tokio::fs::read_dir(directory).await {
        Ok(reader) => reader,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", directory.display())),
    };
    let mut paths = Vec::new();
    while let Some(entry) = reader.next_entry().await.map_err(error)? {
        paths.push(entry.path());
    }
    Ok(paths)
}

async fn runtime_files(scope: &Scope) -> Result<Vec<std::path::PathBuf>> {
    let prefix = format!("{}-", scope.prefix);
    Ok(entries(std::path::Path::new(crate::SYSTEMD_RUNTIME_UNITS))
        .await?
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .collect())
}

async fn remove_path(path: &std::path::Path) -> Result<()> {
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let result = if metadata.is_dir() {
        tokio::fs::remove_dir_all(path).await
    } else {
        tokio::fs::remove_file(path).await
    };
    result.map_err(|e| format!("{}: {e}", path.display()))
}

fn stop_budget(micros: u64) -> Result<Duration> {
    if micros == u64::MAX {
        return Err("infinite stop timeout; shutdown observation incomplete".into());
    }
    Duration::from_micros(micros)
        .checked_add(STOP_REPORT_MARGIN)
        .ok_or_else(|| "stop observation budget overflow".into())
}

fn value<'a>(value: impl Into<Variant<'a>>) -> Result<OwnedVariant> {
    value.into().try_into().map_err(error)
}

/// Quote one approved environment assignment using systemd.syntax C escapes and literal specifiers.
fn environment_line(variable: &crate::EnvironmentVariable) -> Result<String> {
    use std::fmt::Write as _;
    let mut line = String::from("Environment=\"");
    for character in format!("{}={}", variable.name, variable.value).chars() {
        match character {
            '\0' => return Err("Dora environment values cannot contain NUL".into()),
            '%' => line.push_str("%%"),
            '\\' => line.push_str("\\\\"),
            '"' => line.push_str("\\\""),
            character if character.is_ascii_control() => {
                write!(&mut line, "\\x{:02x}", character as u32).expect("write to string")
            }
            character => line.push(character),
        }
    }
    line.push_str("\"\n");
    Ok(line)
}

/// Transient service properties for one program.
fn transient_properties(
    scope: &Scope,
    run: &str,
    program: &Program,
    stop_seconds: u64,
) -> Result<Vec<(String, OwnedVariant)>> {
    let executable = program.argv[0].clone();
    let stop_seconds = if program.stop_seconds == 0 {
        stop_seconds
    } else {
        program.stop_seconds
    };
    let mut properties = vec![
        (
            "Description".into(),
            value(format!("Doramcap run {run}: {}", program.id))?,
        ),
        ("Slice".into(), value(scope.slice.clone())?),
        ("Type".into(), value("exec")?),
        // Keep the unit and its accounting readable until this run releases it.
        ("AddRef".into(), value(true)?),
        (
            "ExecStart".into(),
            value(vec![(executable, program.argv.clone(), false)])?,
        ),
        (
            "TimeoutStopUSec".into(),
            value(
                stop_seconds
                    .checked_mul(MICROS_PER_SECOND)
                    .ok_or("stop timeout overflow")?,
            )?,
        ),
    ];
    if !program.user.is_empty() {
        properties.push(("User".into(), value(program.user.clone())?));
    }
    if !program.working_directory.is_empty() {
        properties.push((
            "WorkingDirectory".into(),
            value(program.working_directory.clone())?,
        ));
    }
    if !program.environment.is_empty() {
        let environment: Vec<String> = program
            .environment
            .iter()
            .map(|variable| format!("{}={}", variable.name, variable.value))
            .collect();
        properties.push(("Environment".into(), value(environment)?));
    }
    if program.memlock_bytes != 0 {
        properties.push(("LimitMEMLOCK".into(), value(program.memlock_bytes)?));
        properties.push(("LimitMEMLOCKSoft".into(), value(program.memlock_bytes)?));
    }
    if !program.cpus.is_empty() {
        let cpus = resource_policy::CpuSet::parse(&program.cpus).map_err(error)?;
        properties.push(("AllowedCPUs".into(), value(cpu_mask(cpus.ids()))?));
    }
    if program.kill_signal != 0 {
        properties.push(("KillSignal".into(), value(program.kill_signal)?));
    }
    Ok(properties)
}

/// Run a generated unit in a private root under the scope's runtime directory, so
/// bind targets never touch the host tree. The unit sees host system directories,
/// `/proc`, `/sys` and `/dev` (including the shared `/dev/shm`), its executable and
/// working directory at their own paths, and `binds`.
async fn private_root(
    scope: &Scope,
    name: &str,
    executable: &str,
    working_directory: &str,
    binds: &[crate::Bind],
) -> Result<Vec<(String, OwnedVariant)>> {
    let root = scope.directory.join(ROOTS).join(name);
    tokio::fs::create_dir_all(&root).await.map_err(error)?;
    let mount = |source: &str, target: &str, optional: bool| {
        (source.to_owned(), target.to_owned(), optional, MS_REC)
    };
    let mut read_only: Vec<_> = SYSTEM_PATHS
        .iter()
        .map(|path| mount(path, path, true))
        .collect();
    read_only.push(mount(executable, executable, false));
    let mut writable = Vec::new();
    if !working_directory.is_empty()
        && !binds
            .iter()
            .any(|bind| std::path::Path::new(working_directory).starts_with(&bind.target))
    {
        writable.push(mount(working_directory, working_directory, false));
    }
    for bind in binds {
        let entry = mount(&bind.source, &bind.target, false);
        if bind.read_only {
            read_only.push(entry);
        } else {
            writable.push(entry);
        }
    }
    let mut properties = vec![
        (
            "RootDirectory".into(),
            value(root.to_string_lossy().into_owned())?,
        ),
        ("MountAPIVFS".into(), value(true)?),
        ("BindReadOnlyPaths".into(), value(read_only)?),
    ];
    if !writable.is_empty() {
        properties.push(("BindPaths".into(), value(writable)?));
    }
    Ok(properties)
}

/// Drop-in lines running an app unit in its private root: host system directories
/// read-only, `binaries` on the executable search path, and the app's `binds`.
async fn app_root(
    scope: &Scope,
    name: &str,
    binds: &[crate::Bind],
    binaries: &str,
) -> Result<String> {
    let root = scope.directory.join(ROOTS).join(name);
    tokio::fs::create_dir_all(&root).await.map_err(error)?;
    // A leading `-` lets a system directory be absent, as on hosts without /lib64.
    let system: Vec<String> = SYSTEM_PATHS.iter().map(|path| format!("-{path}")).collect();
    let mut lines = format!(
        "RootDirectory={}\nMountAPIVFS=yes\nBindReadOnlyPaths={}\nBindReadOnlyPaths={binaries}\nExecSearchPath={binaries}:{}\n",
        root.display(),
        system.join(" "),
        DEFAULT_SEARCH_PATH.join(":"),
    );
    for (index, bind) in binds.iter().enumerate() {
        let (kind, source) = if bind.read_only {
            // Captured paths may contain any host filename character; unit syntax sees plain aliases.
            let sources = root.with_file_name(format!("{name}{BIND_SOURCES_SUFFIX}"));
            tokio::fs::create_dir_all(&sources).await.map_err(error)?;
            let source = sources.join(index.to_string());
            tokio::fs::symlink(&bind.source, &source)
                .await
                .map_err(error)?;
            ("BindReadOnlyPaths", source.to_string_lossy().into_owned())
        } else {
            ("BindPaths", bind.source.clone())
        };
        lines.push_str(&format!("{kind}={source}:{}\n", bind.target));
    }
    Ok(lines)
}

/// Little-endian CPU bitmask, as systemd's `AllowedCPUs` expects.
fn cpu_mask(ids: &[u32]) -> Vec<u8> {
    let mut mask = vec![
        0u8;
        ids.iter()
            .max()
            .map_or(0, |max| *max as usize / BITS_PER_BYTE + 1)
    ];
    for id in ids {
        mask[*id as usize / BITS_PER_BYTE] |= 1 << (*id as usize % BITS_PER_BYTE);
    }
    mask
}

#[cfg(test)]
mod tests {
    #[test]
    fn cpu_masks_set_one_bit_per_cpu() {
        // Arrange
        let ids = [0, 2, 9];
        // Act
        let mask = super::cpu_mask(&ids);
        // Assert
        assert_eq!(mask, [0b0000_0101, 0b0000_0010]);
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn cleanup_propagates_discovery_and_deletion_failures() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("systemd-cleanup-errors-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("owned");
        std::fs::write(&file, "owned").unwrap();
        // Act / Assert: a non-directory cannot establish an empty traversal.
        assert!(entries(&file).await.is_err());
        // Arrange: the unprivileged test cannot delete within this directory.
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o500)).unwrap();
        // Act
        let result = remove_path(&file).await;
        let preserved = file.exists();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        // Assert
        assert!(result.is_err());
        assert!(preserved);
    }
}
