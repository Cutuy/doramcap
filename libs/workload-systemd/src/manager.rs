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
/// Drop-in placing an adopted unit in the run slice.
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
    async fn watch(&self, name: &str) -> Result<Unit> {
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
    /// wait until its executable runs.
    pub async fn start_program(
        &self,
        scope: &Scope,
        run: &str,
        program: &Program,
        stop_seconds: u64,
    ) -> Result<Unit> {
        let name = scope.unit(&program.id);
        let unit = self.watch(&name).await?;
        let properties = transient_properties(scope, run, program, stop_seconds)?;
        let mut jobs = self.manager.receive_job_removed().await.map_err(error)?;
        let job = self
            .manager
            .start_transient_unit(name.clone(), "fail".into(), properties, vec![])
            .await
            .map_err(error)?;
        self.started(&name, &mut jobs, job).await?;
        Ok(unit)
    }

    /// Link copies of repository unit files into the runtime unit directory and
    /// place each in the scope's slice. Call [`Self::start_adopted`] after this.
    pub async fn adopt(&self, scope: &Scope, files: &[String]) -> Result<Vec<String>> {
        let units = scope.directory.join("units");
        tokio::fs::create_dir_all(&units).await.map_err(error)?;
        let mut names = Vec::new();
        let mut copies = Vec::new();
        for file in files {
            let name = scope.unit(crate::unit_stem(file)?);
            let copy = units.join(&name);
            tokio::fs::copy(file, &copy).await.map_err(error)?;
            let drop_in =
                std::path::Path::new(crate::SYSTEMD_RUNTIME_UNITS).join(format!("{name}.d"));
            tokio::fs::create_dir_all(&drop_in).await.map_err(error)?;
            tokio::fs::write(
                drop_in.join(SLICE_DROP_IN),
                format!("[Service]\nSlice={}\n", scope.slice),
            )
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

    /// Reference and start one adopted unit; wait for its start job.
    pub async fn start_adopted(&self, name: &str) -> Result<Unit> {
        let unit = self.watch(name).await?;
        self.manager.ref_unit(name.into()).await.map_err(error)?;
        let mut jobs = self.manager.receive_job_removed().await.map_err(error)?;
        let job = self
            .manager
            .start_unit(name.into(), "replace".into())
            .await
            .map_err(error)?;
        self.started(name, &mut jobs, job).await?;
        Ok(unit)
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

    /// Stop everything the scope owns and remove its runtime files.
    /// Returns the units still loaded afterwards; empty means clean.
    pub async fn reset(&self, scope: &Scope, deadline: Duration) -> Result<Vec<String>> {
        let pattern = format!("{}*", scope.prefix);
        let mut slice = self.watch(&scope.slice).await?;
        // Stopping the slice stops every unit placed in it.
        let _ = self.stop(&scope.slice).await;
        tokio::time::timeout(deadline, slice.ended())
            .await
            .map_err(|_| format!("{} did not stop within {deadline:?}", scope.slice))??;
        for (name, ..) in self.units(&pattern).await? {
            self.release(&name).await;
        }
        self.unlink(scope).await?;
        let _ = self.manager.reset_failed_unit(scope.slice.clone()).await;
        let remaining = self
            .units(&pattern)
            .await?
            .into_iter()
            .filter(|(_, load, active)| load != "not-found" || active != "inactive")
            .map(|(name, ..)| name)
            .collect();
        Ok(remaining)
    }

    /// Remove adopted unit links, copies and slice drop-ins, then reload.
    async fn unlink(&self, scope: &Scope) -> Result<()> {
        let units = scope.directory.join("units");
        let mut names = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(&units).await {
            while let Some(entry) = entries.next_entry().await.map_err(error)? {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        if !names.is_empty() {
            let _ = self.manager.disable_unit_files(names.clone(), true).await;
        }
        for name in &names {
            let drop_in =
                std::path::Path::new(crate::SYSTEMD_RUNTIME_UNITS).join(format!("{name}.d"));
            let _ = tokio::fs::remove_dir_all(drop_in).await;
        }
        let _ = tokio::fs::remove_dir_all(&units).await;
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

fn value<'a>(value: impl Into<Variant<'a>>) -> Result<OwnedVariant> {
    value.into().try_into().map_err(error)
}

/// Transient service properties for one program.
fn transient_properties(
    scope: &Scope,
    run: &str,
    program: &Program,
    stop_seconds: u64,
) -> Result<Vec<(String, OwnedVariant)>> {
    let executable = program.argv[0].clone();
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
            value(stop_seconds * MICROS_PER_SECOND)?,
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

/// Little-endian CPU bitmask, as systemd's `AllowedCPUs` expects.
fn cpu_mask(ids: &[u32]) -> Vec<u8> {
    let mut mask = vec![0u8; ids.iter().max().map_or(0, |max| *max as usize / 8 + 1)];
    for id in ids {
        mask[*id as usize / 8] |= 1 << (id % 8);
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
