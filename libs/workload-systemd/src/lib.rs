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
    if let Some(dora) = &plan.dora {
        let absolute = |path: &str| Path::new(path).is_absolute();
        if !absolute(&dora.executable)
            || !Path::new(&dora.runtime_directory).is_dir()
            || !absolute(&dora.runtime_directory)
            || (!dora.graph.is_empty() && !absolute(&dora.graph))
            || dora.coordinator_port == 0
            || dora.daemon_port == 0
        {
            return Err("Dora needs an absolute executable, an existing absolute runtime directory, an absolute graph and both ports".into());
        }
    }
    let mut ids = std::collections::BTreeSet::new();
    for group in &plan.groups {
        for program in &group.programs {
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
            if !program.cpus.is_empty() {
                resource_policy::CpuSet::parse(&program.cpus).map_err(error)?;
            }
        }
        for file in &group.unit_files {
            if !Path::new(file).is_absolute() || !ids.insert(unit_stem(file)?.to_owned()) {
                return Err(format!("invalid or duplicate unit file {file:?}"));
            }
        }
    }
    Ok(())
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
            groups: vec![UnitGroup {
                programs: vec![Program {
                    id: "monitor".into(),
                    argv: vec!["/bin/sleep".into(), "60".into()],
                    cpus: "0-1".into(),
                    ..Default::default()
                }],
                unit_files: vec!["/repo/deploy/perception.service".into()],
                await_recordings: false,
            }],
            dora: None,
        }
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
            |p| p.groups[0].programs[0].argv[0] = "sleep".into(),
            |p| p.groups[0].programs[0].id = "Monitor".into(),
            |p| p.groups[0].programs[0].cpus = "0-".into(),
            |p| p.groups[0].unit_files[0] = "/repo/deploy/monitor.service".into(),
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
        let text = r#"groups { programs { id: "a" argv: "/bin/true" } }"#;
        // Act
        let plan = parse_plan(text).unwrap();
        let run = parse_run(r#"id: "r1" stop_seconds: 5"#).unwrap();
        // Assert
        assert_eq!(plan.groups[0].programs[0].argv, ["/bin/true"]);
        assert_eq!((run.id.as_str(), run.stop_seconds), ("r1", 5));
        assert!(parse_plan("unknown_field: 1").is_err());
    }
}
