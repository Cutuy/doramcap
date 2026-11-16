//! Native Linux Docker routing shared by Holoscan preparation and execution.
//! Reads DOCKER_HOST, DOCKER_CONTEXT, DOCKER_CONFIG, HOME, DOCKER_TLS and DOCKER_TLS_VERIFY.
//! The supported CLI/executable must preserve this native routing; wrappers are not inspected.
use docker_engine::{Engine, Error, Result};
use serde::{Deserialize, de::DeserializeOwned};
use std::{ffi::OsString, path::{Path, PathBuf}};

const DEFAULT_SOCKET: &str = "/var/run/docker.sock";
const DEFAULT_CONTEXT: &str = "default";

#[derive(Default)]
struct NativeEnvironment {
    host: Option<String>,
    context: Option<String>,
    config: Option<PathBuf>,
    home: Option<PathBuf>,
    tls: bool,
    tls_verify: bool,
}
impl NativeEnvironment {
    fn capture() -> Result<Self> {
        fn text(name: &str) -> Result<Option<String>> {
            nonempty(std::env::var_os(name))
                .map(|value| value.into_string().map_err(|_| Error::Invalid(format!("{name} must be valid UTF-8"))))
                .transpose()
        }
        Ok(Self {
            host: text("DOCKER_HOST")?,
            context: text("DOCKER_CONTEXT")?,
            config: nonempty(std::env::var_os("DOCKER_CONFIG")).map(PathBuf::from),
            home: std::env::home_dir(),
            tls: nonempty(std::env::var_os("DOCKER_TLS")).is_some(),
            tls_verify: nonempty(std::env::var_os("DOCKER_TLS_VERIFY")).is_some(),
        })
    }
    fn config_directory(&self, cwd: &Path) -> Result<PathBuf> {
        let directory = self.config.clone().or_else(|| self.home.as_ref().map(|home| home.join(".docker")))
            .ok_or_else(|| Error::Invalid("cannot determine the native Docker configuration directory: DOCKER_CONFIG and user home are unavailable".into()))?;
        Ok(if directory.is_absolute() { directory } else { cwd.join(directory) })
    }
}
fn nonempty(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
}

#[derive(Default, Deserialize)]
struct DockerConfig {
    #[serde(rename = "currentContext")]
    current_context: Option<String>,
}
#[derive(Deserialize)]
struct ContextMetadata {
    #[serde(rename = "Endpoints")]
    endpoints: ContextEndpoints,
}
#[derive(Deserialize)]
struct ContextEndpoints {
    docker: DockerEndpoint,
}
#[derive(Deserialize)]
struct DockerEndpoint {
    #[serde(rename = "Host")]
    host: String,
}

async fn read_json<T: DeserializeOwned>(path: &Path, missing_default: bool) -> Result<Option<T>> {
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(error) if missing_default && error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Error::Invalid(format!("cannot read Docker routing input {}: {error}", path.display()))),
    };
    serde_json::from_slice(&bytes).map(Some).map_err(|error| Error::Invalid(format!(
        "invalid Docker routing fields in {} ({:?} at line {}, column {})",
        path.display(), error.classify(), error.line(), error.column()
    )))
}
fn unix_socket(host: &str, input: &str) -> Result<PathBuf> {
    let Some(socket) = host.strip_prefix("unix://") else {
        return Err(Error::Invalid(format!("{input} must select an absolute Unix Docker socket; other transports cannot use local overlays")));
    };
    let socket = PathBuf::from(socket);
    if !socket.is_absolute() || socket.as_os_str().as_encoded_bytes().contains(&0) || host["unix://".len()..].contains("://") {
        return Err(Error::Invalid(format!("{input} must select an absolute Unix Docker socket")));
    }
    Ok(socket)
}
async fn resolve(environment: &NativeEnvironment, child_cwd: &Path) -> Result<PathBuf> {
    if !child_cwd.is_absolute() {
        return Err(Error::Invalid("Docker routing requires the exact absolute CLI child working directory".into()));
    }
    // Verified Docker 28.0.3/29.9.1 source and 29.9.1 CLI prefer DOCKER_HOST over DOCKER_CONTEXT.
    if let Some(host) = &environment.host {
        let host = host.trim();
        if !host.is_empty() && host != "unix://" {
            return unix_socket(host, "DOCKER_HOST");
        }
        if host == "unix://" { return Ok(DEFAULT_SOCKET.into()); }
        return default_socket(environment);
    }
    let context = if let Some(context) = &environment.context {
        context.clone()
    } else {
        let path = environment.config_directory(child_cwd)?.join("config.json");
        read_json::<DockerConfig>(&path, true).await?.unwrap_or_default().current_context
            .filter(|context| !context.is_empty()).unwrap_or_else(|| DEFAULT_CONTEXT.into())
    };
    if context == DEFAULT_CONTEXT { return default_socket(environment); }
    let path = environment.config_directory(child_cwd)?.join("contexts/meta")
        .join(host_filesystem::files::digest(context.as_bytes())).join("meta.json");
    let metadata = read_json::<ContextMetadata>(&path, false).await?.expect("required context input");
    unix_socket(&metadata.endpoints.docker.host, "selected Docker context Endpoints.docker.Host")
}
fn default_socket(environment: &NativeEnvironment) -> Result<PathBuf> {
    if environment.tls || environment.tls_verify {
        return Err(Error::Invalid("native Docker TLS settings select a TCP default; local overlays require an explicit absolute Unix socket".into()));
    }
    Ok(DEFAULT_SOCKET.into())
}

/// Resolve the native CLI's absolute Unix socket using its unchanged inherited environment and exact cwd.
/// Selected unreadable/malformed routing fields and non-Unix/relative endpoints fail; registry/TLS credentials are not read.
/// Compatibility follows Docker 28.0.3/29.9.1 host-first precedence; no subprocess or environment mutation occurs.
pub async fn native_docker_socket(child_cwd: &Path) -> Result<PathBuf> {
    resolve(&NativeEnvironment::capture()?, child_cwd).await
}

/// Require native CLI and Console sockets to expose the same nonempty daemon ID and manager mount namespace.
/// Performs only Unix-socket/API reads; failures block launch. Native Docker routing must remain unchanged by wrappers.
/// Returns the observed daemon ID; callers recheck before side effects when launch inputs may have changed.
pub async fn verify_native_daemon(child_cwd: &Path, console: &Engine) -> Result<String> {
    verify_socket(&native_docker_socket(child_cwd).await?, console).await
}
async fn verify_socket(socket: &Path, console: &Engine) -> Result<String> {
    docker_engine::namespace::verify_namespace(socket).await?;
    docker_engine::namespace::verify_namespace(console.socket_path()).await?;
    let socket = socket.to_str().ok_or_else(|| Error::Invalid("native Docker socket must be valid UTF-8".into()))?;
    let native = Engine::connect(socket).await?;
    let native_id = daemon_id(&native, "native CLI-selected Docker").await?;
    let console_id = daemon_id(console, "Console Docker").await?;
    if native_id != console_id {
        return Err(Error::Invalid("native Docker routing and Console select different daemon IDs".into()));
    }
    Ok(native_id)
}
async fn daemon_id(engine: &Engine, owner: &str) -> Result<String> {
    let info = engine.client().info().await.map_err(|source| Error::Engine {
        operation: format!("read {owner} daemon identity"), source,
    })?;
    info.id.filter(|id| !id.trim().is_empty())
        .ok_or_else(|| Error::Invalid(format!("{owner} daemon ID is missing or empty")))
}

#[cfg(test)]
mod tests;
