//! One run's apps as owned systemd units in a run slice, controlled over D-Bus.
//!
//! Requires the system service manager and root privileges. The repository root
//! scopes admission, unit names, the slice and reset.
pub mod cgroup;
mod dora;
mod journal;
mod manager;
mod preparation;
mod runner;
mod running;
mod runtime;

pub use preparation::{DiscardRun, InspectRun, PrepareRun, inspect};
pub use runner::{SystemdRunReport, SystemdRunner, UnitExit};
pub use workload_lifecycle::launch::RunPlan;

include!(concat!(env!("OUT_DIR"), "/doramcap.systemd_plan.v1.rs"));

pub type Result<T> = std::result::Result<T, String>;
pub(crate) fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Longest program ID and adopted unit stem.
const MAX_ID_CHARS: usize = 40;
/// Hex characters of the repository hash naming its slice and units.
const SCOPE_CHARS: usize = 12;
/// File naming the active run in the runtime directory holding adopted unit copies.
pub(crate) const ACTIVE_RUN: &str = "active-run";
/// Runtime root.
const RUNTIME_ROOT: &str = "/run/doramcap";
/// Directory where the service manager reads runtime units and drop-ins.
const SYSTEMD_RUNTIME_UNITS: &str = "/run/systemd/system";

/// Names owned by one repository: its slice, unit prefix and runtime directory.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Scope {
    pub prefix: String,
    pub slice: String,
    pub directory: std::path::PathBuf,
}
impl Scope {
    /// Derive names from the canonical repository root.
    pub fn of(repository: &std::path::Path) -> Result<Self> {
        let hash = repository_id(repository)?;
        let prefix = format!("doramcap-{hash}");
        Ok(Self {
            slice: format!("{prefix}.slice"),
            directory: std::path::Path::new(RUNTIME_ROOT).join(hash),
            prefix,
        })
    }
    /// Service name for a program ID or adopted unit stem.
    pub fn unit(&self, id: &str) -> String {
        format!("{}-{id}.service", self.prefix)
    }
}

fn repository_id(repository: &std::path::Path) -> Result<String> {
    use sha2::Digest;
    let canonical = std::fs::canonicalize(repository).map_err(error)?;
    let digest = sha2::Sha256::digest(canonical.as_os_str().as_encoded_bytes());
    Ok(hex::encode(digest)[..SCOPE_CHARS].to_owned())
}

fn valid_id(id: &str) -> bool {
    (1..=MAX_ID_CHARS).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Stem of an adopted unit file: its basename without `.service`.
pub(crate) fn unit_stem(path: &str) -> Result<&str> {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".service"))
        .filter(|stem| valid_id(stem))
        .ok_or_else(|| format!("unit file must be a [a-z0-9-] name ending in .service: {path}"))
}

/// Reject runs that would start nothing safely: missing or relative paths, invalid or
/// duplicate IDs, invalid cpulists or zero budgets.
pub fn validate(run: &RunPlan, plan: &SystemdPlan) -> Result<()> {
    use std::path::Path;
    if !Path::new(&run.repository_root).is_absolute() || !Path::new(&run.artifacts).is_absolute() {
        return Err("repository root and artifacts must be absolute".into());
    }
    if run.readiness_seconds == 0 || run.stop_seconds == 0 {
        return Err("readiness and stop budgets must be positive".into());
    }
    workload_lifecycle::plan::validate_apps(run, app_ids(plan)?.iter().map(String::as_str))?;
    let mut ids = std::collections::BTreeSet::new();
    for program in &plan.programs {
        if !valid_id(&program.id) || !ids.insert(program.id.clone()) {
            return Err(format!("invalid or duplicate program id {:?}", program.id));
        }
        if !program
            .argv
            .first()
            .is_some_and(|exe| Path::new(exe).is_absolute())
        {
            return Err(format!(
                "{}: argv must start with an absolute executable",
                program.id
            ));
        }
        if !program.working_directory.is_empty()
            && !Path::new(&program.working_directory).is_absolute()
        {
            return Err(format!(
                "{}: working directory must be absolute",
                program.id
            ));
        }
        if program.binds.iter().any(|bind| {
            !Path::new(&bind.source).is_absolute() || !Path::new(&bind.target).is_absolute()
        }) {
            return Err(format!(
                "{}: bind sources and targets must be absolute",
                program.id
            ));
        }
        if !program.cpus.is_empty() {
            resource_policy::CpuSet::parse(&program.cpus).map_err(error)?;
        }
    }
    for file in &plan.unit_files {
        if !Path::new(file).is_absolute() || !ids.insert(unit_stem(file)?.to_owned()) {
            return Err(format!("invalid or duplicate unit file {file:?}"));
        }
    }
    runtime::validate_binds(run, plan)
}

/// IDs of programs and adopted units, in launch input order.
fn app_ids(plan: &SystemdPlan) -> Result<Vec<String>> {
    let mut ids: Vec<_> = plan
        .programs
        .iter()
        .map(|program| program.id.clone())
        .collect();
    for file in &plan.unit_files {
        ids.push(unit_stem(file)?.to_owned());
    }
    Ok(ids)
}

/// Select the fixed system or user phase after the complete plan has been validated.
fn phase(run: &RunPlan, plan: &SystemdPlan, role: workload_lifecycle::plan::Phase) -> SystemdPlan {
    let selected = |id: &str| workload_lifecycle::plan::phase(run, id) == role;
    SystemdPlan {
        programs: plan
            .programs
            .iter()
            .filter(|app| selected(&app.id))
            .cloned()
            .collect(),
        unit_files: plan
            .unit_files
            .iter()
            .filter(|file| selected(unit_stem(file).expect("validated unit")))
            .cloned()
            .collect(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> RunPlan {
        RunPlan {
            id: "r1".into(),
            repository_root: "/repo".into(),
            artifacts: "/data/r1".into(),
            readiness_seconds: 10,
            stop_seconds: 5,
            host_shm_access: workload_lifecycle::launch::HostShmAccess::Passthrough as i32,
            ..Default::default()
        }
    }

    fn plan() -> SystemdPlan {
        SystemdPlan {
            programs: vec![Program {
                id: "monitor".into(),
                argv: vec!["/bin/sleep".into(), "60".into()],
                cpus: "0-1".into(),
                ..Default::default()
            }],
            unit_files: vec!["/repo/deploy/perception.service".into()],
            ..Default::default()
        }
    }

    #[test]
    fn dora_apps_are_ordinary_launch_inputs() {
        // Arrange
        let mut plan = plan();
        plan.programs[0].id = "dora-coordinator".into();
        let mut run = run();
        run.dora_apps = vec!["dora-coordinator".into()];
        // Act / Assert
        assert!(validate(&run, &plan).is_ok());
        assert_eq!(
            app_ids(&phase(&run, &plan, workload_lifecycle::plan::Phase::Dora)).unwrap(),
            ["dora-coordinator"]
        );
        // Act / Assert: the former control-unit prefix is available to ordinary apps.
        plan.programs[0].id = "dora-cli-1".into();
        run.dora_apps = vec!["dora-cli-1".into()];
        assert!(validate(&run, &plan).is_ok());
    }

    #[test]
    fn fixed_phases_use_shared_roles_for_every_launch_kind() {
        // Arrange
        let run = RunPlan {
            system_apps: vec!["monitor".into()],
            ..run()
        };
        let plan = plan();
        // Act
        validate(&run, &plan).unwrap();
        let systems = phase(&run, &plan, workload_lifecycle::plan::Phase::System);
        let users = phase(&run, &plan, workload_lifecycle::plan::Phase::User);
        // Assert
        assert_eq!(app_ids(&systems).unwrap(), ["monitor"]);
        assert_eq!(app_ids(&users).unwrap(), ["perception"]);
    }

    #[test]
    fn valid_plan_passes() {
        // Arrange
        let (run, plan) = (run(), plan());
        // Act
        let result = validate(&run, &plan);
        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn unsafe_or_ambiguous_plans_are_rejected() {
        let cases: [fn(&mut SystemdPlan); 4] = [
            |p| p.programs[0].argv[0] = "sleep".into(),
            |p| p.programs[0].id = "Monitor".into(),
            |p| p.programs[0].cpus = "0-".into(),
            |p| p.unit_files[0] = "/repo/deploy/monitor.service".into(),
        ];
        for case in cases {
            // Arrange
            let mut plan = plan();
            case(&mut plan);
            // Act
            let result = validate(&run(), &plan);
            // Assert
            assert!(result.is_err());
        }
        // Arrange / Act / Assert: a zero stop budget is rejected.
        let mut run = run();
        run.stop_seconds = 0;
        assert!(validate(&run, &plan()).is_err());
    }
}
