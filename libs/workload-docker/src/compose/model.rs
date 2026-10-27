use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

/// Launch configuration for one Docker Compose project.
///
/// A project groups services defined by an ordered set of Compose files. It is a
/// configuration value; resolution produces a separate runtime identity.
/// Files and env-files are interpreted relative to `directory`, in supplied order.
/// Environment is explicit; the manager does not inherit interpolation variables.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposeProject {
    /// Caller-selected key, unique across this run; used in artifact and Docker project names.
    pub key: String,
    /// Absolute Compose project directory; relative file paths are resolved here.
    pub directory: PathBuf,
    /// Compose files in merge order; at least one file is required.
    pub files: Vec<PathBuf>,
    /// Interpolation environment files in precedence order, relative to `directory`.
    #[serde(default)]
    pub env_files: Vec<PathBuf>,
    /// Explicit interpolation variables; overrides env-file values. Docker/Compose control variables are rejected.
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    /// Compose profiles to activate; empty uses the default profile selection.
    #[serde(default)]
    pub profiles: Vec<String>,
    /// Explicit Compose service targets; empty selects all enabled services.
    #[serde(default)]
    pub services: Vec<String>,
    /// Services whose successful exit is expected. Compose completion dependencies
    /// are also recognized. Other services must remain running until explicit stop.
    #[serde(default)]
    pub completed_services: Vec<String>,
    /// Delete experiment-owned named volumes at teardown. Default retains them
    /// as isolated artifacts; external volumes are always retained.
    #[serde(default)]
    pub remove_volumes: bool,
}

/// Immutable execution identity returned by resolution. The manifest is private
/// (may contain resolved environment secrets); its digest is checked before use.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedComposeProject {
    /// Run ownership label assigned by the caller.
    pub run: String,
    /// Prepared project key.
    pub key: String,
    /// Generated Docker Compose project name: `dc-{run}-{key}`.
    pub name: String,
    /// Private resolved execution manifest path; can contain secrets.
    pub manifest: PathBuf,
    /// Digest used to reject changes to the execution manifest.
    pub sha256: String,
    /// Selected service names mapped to expected image and lifecycle state.
    pub services: BTreeMap<String, ComposeService>,
    /// Resolution warnings and applied policy changes.
    pub diagnostics: Vec<String>,
    /// Whether teardown removes owned named volumes.
    pub remove_volumes: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComposeService {
    /// Pinned local Docker image ID.
    pub image_id: String,
    /// Expected positive number of service containers.
    pub replicas: usize,
    /// Whether exit code zero is an accepted completion state.
    pub completed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolveInput {
    /// Run identity used to namespace and label Docker resources.
    pub run: String,
    /// Existing directory receiving the manifest and resolution evidence.
    pub artifacts: PathBuf,
    /// Prepared launch settings for one Compose project.
    pub project: ComposeProject,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartInput {
    /// Resolved project whose manifest digest is verified before launch.
    pub plan: ResolvedComposeProject,
    /// Timeout budget for startup/readiness operations, in seconds.
    pub readiness_seconds: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StopInput {
    /// Owned project to stop and inspect.
    pub plan: ResolvedComposeProject,
    /// Grace period passed to shutdown operations, in seconds.
    pub stop_seconds: u64,
}

/// Serializable error evidence, including Engine status and complete bounded CLI
/// diagnostics. Limits and timeout errors explicitly report incomplete capture.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Operation that failed.
    pub operation: String,
    /// Machine-readable error category.
    pub kind: String,
    /// Human-readable error details.
    pub message: String,
    /// CLI exit code or Engine HTTP status when available.
    pub status: Option<i64>,
    /// Captured CLI stdout when available; subject to the runner limit.
    pub stdout: Option<String>,
    /// Captured CLI stderr when available; subject to the runner limit.
    pub stderr: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Container {
    /// Exact Docker container ID.
    pub id: String,
    /// Compose service name.
    pub service: String,
    /// Actual image ID reported by Engine.
    pub image_id: String,
    /// Host PID reported by Engine; zero when stopped.
    pub pid: i64,
    /// Engine container lifecycle status.
    pub status: String,
    /// Configured health-check status, if present.
    pub health: Option<String>,
    /// Exit status when the container has terminated.
    pub exit_code: Option<i64>,
    /// Engine-reported restart count.
    pub restart_count: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComposeProjectState {
    /// Resolved project identity used for this inspection.
    pub plan: ResolvedComposeProject,
    /// Containers verified as belonging to the project and run.
    pub containers: Vec<Container>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cleanup {
    /// Generated Compose project name.
    pub project: String,
    /// Shutdown/removal errors retained even when later cleanup succeeds.
    pub diagnostics: Vec<Diagnostic>,
    /// Resource identities or unresolved checks preventing clean teardown.
    pub remaining: Vec<String>,
    /// Owned named volumes intentionally retained as artifacts.
    pub retained_volumes: Vec<String>,
    /// Whether the final resource checks establish a clean boundary.
    pub verified: bool,
}
