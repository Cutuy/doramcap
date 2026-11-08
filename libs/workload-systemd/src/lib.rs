//! One run's apps as owned systemd units in a run slice, controlled over D-Bus.
//!
//! Requires the system service manager and root privileges. The repository root
//! scopes admission, unit names, the slice and reset.
mod dora;
mod manager;
mod preparation;
mod runner;
mod running;

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
/// Runtime directory holding adopted unit copies and the admission lock.
pub(crate) const ACTIVE_RUN: &str = "active-run";
/// Runtime root.
const RUNTIME_ROOT: &str = "/run/doramcap";
use workload_lifecycle::plan::HOST_SHM_ROOT as SHM_ROOT;
/// Directory where the service manager reads runtime units and drop-ins.
const SYSTEMD_RUNTIME_UNITS: &str = "/run/systemd/system";

/// Decode strict protobuf text into message `name` of this crate's descriptor pool.
fn parse_text<M: prost::Message + Default>(name: &str, text: &str) -> Result<M> {
    use prost::Message as _;
    let pool = prost_reflect::DescriptorPool::decode(
        include_bytes!(concat!(env!("OUT_DIR"), "/systemd_plan.bin")).as_slice(),
    )
    .map_err(error)?;
    let descriptor = pool
        .get_message_by_name(name)
        .ok_or_else(|| format!("{name} descriptor absent"))?;
    let message =
        prost_reflect::DynamicMessage::parse_text_format(descriptor, text).map_err(error)?;
    M::decode(message.encode_to_vec().as_slice()).map_err(error)
}

/// Decode a strict protobuf text-format `SystemdPlan`.
pub fn parse_plan(text: &str) -> Result<SystemdPlan> {
    parse_text("doramcap.systemd_plan.v1.SystemdPlan", text)
}

/// Decode a strict protobuf text-format `RunPlan`.
pub fn parse_run(text: &str) -> Result<RunPlan> {
    parse_text("doramcap.launch.v1.RunPlan", text)
}

/// Names owned by one repository: its slice, unit prefix and runtime directory.
#[derive(Clone, Debug)]
pub(crate) struct Scope {
    pub prefix: String,
    pub slice: String,
    pub directory: std::path::PathBuf,
    /// Shared-memory directory every unit sees as /dev/shm.
    pub shm: std::path::PathBuf,
}
impl Scope {
    /// Derive names from the canonical repository root.
    pub fn of(repository: &std::path::Path) -> Result<Self> {
        use sha2::Digest;
        let canonical = std::fs::canonicalize(repository).map_err(error)?;
        let digest = sha2::Sha256::digest(canonical.as_os_str().as_encoded_bytes());
        let hash = &hex::encode(digest)[..SCOPE_CHARS];
        let prefix = format!("doramcap-{hash}");
        Ok(Self {
            slice: format!("{prefix}.slice"),
            directory: std::path::Path::new(RUNTIME_ROOT).join(hash),
            shm: std::path::Path::new(SHM_ROOT).join(&prefix),
            prefix,
        })
    }
    /// Service name for a program ID or adopted unit stem.
    pub fn unit(&self, id: &str) -> String {
        format!("{}-{id}.service", self.prefix)
    }
}

/// An absolute path that unit-file syntax carries without quoting.
fn plain_path(path: &str) -> bool {
    std::path::Path::new(path).is_absolute()
        && !path
            .chars()
            .any(|c| c.is_whitespace() || c == ':' || c == '\\')
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
    let reserved = |id: &str| !run.dora_apps.is_empty() && id.starts_with(dora::CLI_PREFIX);
    for program in &plan.programs {
        if !valid_id(&program.id) || reserved(&program.id) || !ids.insert(program.id.clone()) {
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
        if !Path::new(file).is_absolute()
            || reserved(unit_stem(file)?)
            || !ids.insert(unit_stem(file)?.to_owned())
        {
            return Err(format!("invalid or duplicate unit file {file:?}"));
        }
    }
    for app in &plan.app_units {
        if !valid_id(&app.id) || reserved(&app.id) || !ids.insert(app.id.clone()) {
            return Err(format!("invalid or duplicate app id {:?}", app.id));
        }
        if !Path::new(&app.file).is_absolute()
            || Path::new(&app.file)
                .extension()
                .is_none_or(|extension| extension != "service")
        {
            return Err(format!(
                "app unit file must be an absolute .service path: {:?}",
                app.file
            ));
        }
    }
    if !plan.app_units.is_empty() && !plain_path(&plan.binaries) {
        return Err("app units need an absolute binaries directory without spaces".into());
    }
    for bind in plan.app_units.iter().flat_map(|app| &app.binds) {
        if !plain_path(&bind.source) || !plain_path(&bind.target) {
            return Err(format!(
                "app unit binds need absolute paths without spaces: {bind:?}"
            ));
        }
    }
    Ok(())
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
    ids.extend(plan.app_units.iter().map(|app| app.id.clone()));
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
        app_units: plan
            .app_units
            .iter()
            .filter(|app| selected(&app.id))
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
        // Act / Assert: control units remain owned by the workflow.
        plan.programs[0].id = "dora-cli-1".into();
        run.dora_apps = vec!["dora-cli-1".into()];
        assert!(validate(&run, &plan).is_err());
    }

    #[test]
    fn fixed_phases_use_shared_roles_for_every_launch_kind() {
        // Arrange
        let run = RunPlan {
            system_apps: vec!["monitor".into(), "capture".into()],
            ..run()
        };
        let plan = SystemdPlan {
            app_units: vec![AppUnit {
                id: "capture".into(),
                file: "/apps/worker.service".into(),
                binds: vec![],
            }],
            binaries: "/bin".into(),
            ..plan()
        };
        // Act
        validate(&run, &plan).unwrap();
        let systems = phase(&run, &plan, workload_lifecycle::plan::Phase::System);
        let users = phase(&run, &plan, workload_lifecycle::plan::Phase::User);
        // Assert
        assert_eq!(app_ids(&systems).unwrap(), ["monitor", "capture"]);
        assert_eq!(app_ids(&users).unwrap(), ["perception"]);
        // Act / Assert: old configurable phases are rejected by the text parser.
        assert!(parse_plan("groups { await_recordings: true }").is_err());
    }

    #[test]
    fn app_ids_are_unique_across_launch_kinds_and_keep_native_filenames_independent() {
        // Arrange
        let mut run = run();
        run.system_apps = vec!["capture".into(), "observer".into()];
        let mut plan = SystemdPlan {
            binaries: "/bin".into(),
            app_units: ["capture", "observer"]
                .into_iter()
                .map(|id| AppUnit {
                    id: id.into(),
                    file: format!("/apps/{id}/worker.service"),
                    binds: vec![],
                })
                .collect(),
            ..plan()
        };
        // Act / Assert: identical native basenames have independent app identities.
        assert!(validate(&run, &plan).is_ok());
        for id in ["", "Uppercase", "monitor", "perception", "observer"] {
            // Act / Assert: empty, invalid and colliding app IDs fail before launch.
            plan.app_units[0].id = id.into();
            assert!(
                validate(
                    &RunPlan {
                        system_apps: vec![],
                        ..run.clone()
                    },
                    &plan
                )
                .is_err(),
                "{id:?}"
            );
        }
        // Act / Assert: workflow-owned control names remain reserved.
        plan.app_units[0].id = "dora-cli-1".into();
        run.system_apps.clear();
        run.dora_apps = vec!["observer".into()];
        assert!(validate(&run, &plan).is_err());
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

    #[test]
    fn text_plans_decode_strictly() {
        // Arrange
        let text = r#"programs { id: "a" argv: "/bin/true" }"#;
        // Act
        let plan = parse_plan(text).unwrap();
        let run = parse_run(r#"id: "r1" stop_seconds: 5"#).unwrap();
        // Assert
        assert_eq!(plan.programs[0].argv, ["/bin/true"]);
        assert_eq!((run.id.as_str(), run.stop_seconds), ("r1", 5));
        assert!(parse_plan("unknown_field: 1").is_err());
    }
}
