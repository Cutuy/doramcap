//! One run's apps as owned systemd units in a run slice, controlled over D-Bus.
//!
//! Requires the system service manager and root privileges. The repository root
//! scopes admission, unit names, the slice and reset.
mod dora;
mod manager;
mod runner;
mod running;

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
    if run.dora.is_some() != plan.dora.is_some() {
        return Err("Dora needs both shared settings and launch settings".into());
    }
    if let (Some(dora), Some(launch)) = (&run.dora, &plan.dora) {
        if dora.participant_uid.is_some() || dora.participant_gid.is_some() {
            return Err(
                "systemd Dora uses its launch user; numeric participant IDs are unsupported".into(),
            );
        }
        let absolute = |path: &str| Path::new(path).is_absolute();
        if !absolute(&launch.executable)
            || !Path::new(&dora.runtime_directory).is_dir()
            || !absolute(&dora.runtime_directory)
            || (!dora.graph.is_empty() && !absolute(&dora.graph))
        {
            return Err("Dora needs an absolute executable, an existing absolute runtime directory, an absolute graph and both ports".into());
        }
        let valid_port = |port| u16::try_from(port).is_ok_and(|port| port != 0);
        if !valid_port(dora.coordinator_port)
            || !valid_port(dora.daemon_port)
            || dora.coordinator_port == dora.daemon_port
        {
            return Err("Dora ports must be distinct and in 1..=65535".into());
        }
    }
    let mut ids = std::collections::BTreeSet::new();
    if plan.dora.is_some() {
        ids.extend([dora::COORDINATOR.to_owned(), dora::DAEMON.to_owned()]);
    }
    let reserved = |id: &str| plan.dora.is_some() && id.starts_with(dora::CLI_PREFIX);
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
    let apps = plan.app_units.iter().map(|app| &app.file);
    for file in plan.unit_files.iter().chain(apps) {
        if !Path::new(file).is_absolute()
            || reserved(unit_stem(file)?)
            || !ids.insert(unit_stem(file)?.to_owned())
        {
            return Err(format!("invalid or duplicate unit file {file:?}"));
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
    for file in plan
        .unit_files
        .iter()
        .chain(plan.app_units.iter().map(|app| &app.file))
    {
        ids.push(unit_stem(file)?.to_owned());
    }
    Ok(ids)
}

/// Select the fixed system or user phase after the complete plan has been validated.
fn phase(run: &RunPlan, plan: &SystemdPlan, system: bool) -> SystemdPlan {
    let selected = |id: &str| run.system_apps.iter().any(|app| app == id) == system;
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
            .filter(|app| selected(unit_stem(&app.file).expect("validated unit")))
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
            dora: None,
            ..Default::default()
        }
    }

    fn dora_run() -> RunPlan {
        RunPlan {
            dora: Some(workload_lifecycle::launch::DoraSettings {
                runtime_directory: std::env::temp_dir().to_string_lossy().into_owned(),
                coordinator_port: 6013,
                daemon_port: 6014,
                ..Default::default()
            }),
            ..run()
        }
    }

    fn dora_plan() -> SystemdPlan {
        SystemdPlan {
            dora: Some(DoraUnits {
                executable: "/bin/true".into(),
                ..Default::default()
            }),
            ..plan()
        }
    }

    #[test]
    fn dora_ports_require_distinct_valid_tcp_ports() {
        for (coordinator, daemon, accepted) in [
            (1, 65535, true),
            (0, 6014, false),
            (6013, 0, false),
            (65536, 6014, false),
            (6013, 65536, false),
            (6013, 6013, false),
        ] {
            // Arrange
            let plan = dora_plan();
            let mut run = dora_run();
            let dora = run.dora.as_mut().unwrap();
            dora.coordinator_port = coordinator;
            dora.daemon_port = daemon;
            // Act
            let result = validate(&run, &plan);
            // Assert
            assert_eq!(
                result.is_ok(),
                accepted,
                "{coordinator}, {daemon}: {result:?}"
            );
        }
    }

    #[test]
    fn dora_names_are_reserved_for_programs_and_adopted_units() {
        for id in ["dora-coordinator", "dora-daemon", "dora-cli-1"] {
            for adopted in [false, true] {
                // Arrange
                let mut plan = dora_plan();
                if adopted {
                    plan.unit_files = vec![format!("/repo/{id}.service")];
                } else {
                    plan.programs[0].id = id.into();
                }
                // Act / Assert
                assert!(
                    validate(&dora_run(), &plan).is_err(),
                    "{id}, adopted={adopted}"
                );
                // Act: without Dora these are ordinary user-owned names.
                plan.dora = None;
                // Assert
                assert!(validate(&run(), &plan).is_ok());
            }
        }
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
                file: "/apps/capture.service".into(),
                binds: vec![],
            }],
            binaries: "/bin".into(),
            ..plan()
        };
        // Act
        validate(&run, &plan).unwrap();
        let systems = phase(&run, &plan, true);
        let users = phase(&run, &plan, false);
        // Assert
        assert_eq!(app_ids(&systems).unwrap(), ["monitor", "capture"]);
        assert_eq!(app_ids(&users).unwrap(), ["perception"]);
        // Act / Assert: old configurable phases are rejected by the text parser.
        assert!(parse_plan("groups { await_recordings: true }").is_err());
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
