use crate::{
    Catalog, CatalogEntry, CatalogOutcome, DiscoverRequest, LanguageVariant, Mode, QueryDiagnostic,
    query_diagnostic::Termination,
};
use orchestrator::Step;
use serde::Deserialize;
use std::{collections::BTreeMap, os::unix::process::ExitStatusExt, path::Path, process::Stdio};
use tokio::{io::AsyncReadExt, sync::watch};

const JSON_SCHEMA_VERSION: u32 = 1;

/// Read list and per-language modes in one native project context.
/// Retains full query streams; memory grows with streams and catalog size.
/// Owner shutdown kills and reaps; dropping only kills the direct CLI child.
#[derive(Default)]
pub struct DiscoverApplications {
    shutdown: Option<watch::Receiver<bool>>,
}

impl DiscoverApplications {
    /// True or sender disconnect cancels discovery and reaps its active direct child.
    /// The owner keeps the invocation alive until cancellation returns.
    pub fn with_shutdown(shutdown: watch::Receiver<bool>) -> Self {
        Self {
            shutdown: Some(shutdown),
        }
    }
}

impl Step for DiscoverApplications {
    type Input = DiscoverRequest;
    type Output = Catalog;
    const ID: Option<&'static str> = Some("holoscan.discover-applications");
    const DISPLAY_NAME: Option<&'static str> = Some("Discover Holoscan applications");

    async fn run(&mut self, input: DiscoverRequest) -> orchestrator::Result<Catalog> {
        if !Path::new(&input.project_root).is_absolute() || !Path::new(&input.cli).is_absolute() {
            return Err(orchestrator::Error::Configuration(
                "Holoscan project_root and cli must be absolute".into(),
            ));
        }
        let mut catalog = Catalog {
            project_root: input.project_root.clone(),
            ..Default::default()
        };
        if !Path::new(&input.project_root).is_dir() {
            return Ok(failed(
                catalog,
                format!(
                    "Holoscan project_root is not a readable directory: {}",
                    input.project_root
                ),
            ));
        }
        let list = query(&input, &["list", "--json"], &mut self.shutdown).await?;
        catalog.diagnostics.push(list);
        if !check_query(&mut catalog) {
            return Ok(catalog);
        }
        let listed: Listed = match decode(&catalog.diagnostics[0]) {
            Ok(value) => value,
            Err(error) => return Ok(failed(catalog, error)),
        };
        if listed.schema_version != JSON_SCHEMA_VERSION {
            return Ok(failed(
                catalog,
                format!(
                    "unsupported Holoscan list schema_version: {}",
                    listed.schema_version
                ),
            ));
        }
        for entry in listed.projects {
            if entry.name.is_empty()
                || entry.project_type.is_empty()
                || !Path::new(&entry.source_folder).is_absolute()
            {
                return Ok(failed(
                    catalog,
                    "Holoscan list entry requires a name, project_type and absolute source_folder",
                ));
            }
            catalog.entries.push(CatalogEntry {
                name: entry.name,
                project_type: entry.project_type,
                source_folder: entry.source_folder,
                variants: entry
                    .language
                    .into_iter()
                    .map(|language| LanguageVariant {
                        language,
                        modes: Vec::new(),
                    })
                    .collect(),
                declared_modes: entry.modes,
            });
        }
        for index in 0..catalog.entries.len() {
            if catalog.entries[index].project_type != "application" {
                continue;
            }
            if catalog.entries[index].variants.is_empty() {
                catalog.entries[index]
                    .variants
                    .push(LanguageVariant::default());
            }
            for variant in 0..catalog.entries[index].variants.len() {
                let entry = &catalog.entries[index];
                let language = normalize_language(&entry.variants[variant].language);
                let mut args = vec!["modes", entry.name.as_str()];
                if !language.is_empty() {
                    args.extend(["--language", language.as_str()]);
                }
                args.push("--json");
                let result = query(&input, &args, &mut self.shutdown).await?;
                catalog.diagnostics.push(result);
                if !check_query(&mut catalog) {
                    return Ok(catalog);
                }
                let response: Modes = match decode(catalog.diagnostics.last().unwrap()) {
                    Ok(value) => value,
                    Err(error) => return Ok(failed(catalog, error)),
                };
                if response.schema_version != JSON_SCHEMA_VERSION
                    || response.project != catalog.entries[index].name
                    || (!language.is_empty()
                        && !response
                            .language
                            .iter()
                            .any(|name| normalize_language(name) == language))
                {
                    return Ok(failed(
                        catalog,
                        "Holoscan modes response does not match its schema, application or language",
                    ));
                }
                catalog.entries[index].variants[variant].modes = response
                    .modes
                    .into_iter()
                    .map(|(name, mode)| Mode {
                        name,
                        description: mode.description,
                    })
                    .collect();
            }
        }
        catalog.outcome = if catalog
            .entries
            .iter()
            .any(|entry| entry.project_type == "application")
        {
            CatalogOutcome::Available
        } else {
            CatalogOutcome::Empty
        } as i32;
        Ok(catalog)
    }
}

pub(crate) fn normalize_language(name: &str) -> String {
    match name.trim().to_lowercase().as_str() {
        "c++" | "cpp" => "cpp".into(),
        "py" | "python" => "python".into(),
        name => name.into(),
    }
}

pub(crate) async fn cancelled(shutdown: &mut Option<watch::Receiver<bool>>) {
    match shutdown {
        Some(receiver) => {
            let _ = receiver.wait_for(|closing| *closing).await;
        }
        None => std::future::pending().await,
    }
}

pub(crate) async fn query(
    input: &DiscoverRequest,
    args: &[&str],
    shutdown: &mut Option<watch::Receiver<bool>>,
) -> orchestrator::Result<QueryDiagnostic> {
    let argv: Vec<String> = [
        &[
            input.cli.as_str(),
            "--project-root",
            input.project_root.as_str(),
        ][..],
        args,
    ]
    .concat()
    .into_iter()
    .map(str::to_owned)
    .collect();
    if shutdown
        .as_ref()
        .is_some_and(|receiver| *receiver.borrow() || receiver.has_changed().is_err())
    {
        return Ok(QueryDiagnostic {
            argv,
            error: "Closing".into(),
            ..Default::default()
        });
    }
    // Both cwd and --project-root keep native relative configuration in this project.
    let child = tokio::process::Command::new(&input.cli)
        .args(&argv[1..])
        .current_dir(&input.project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            return Ok(QueryDiagnostic {
                argv,
                error: format!("start Holoscan CLI: {error}"),
                ..Default::default()
            });
        }
    };
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    let result = tokio::select! {
        result = async {
            tokio::try_join!(child.wait(), stdout.read_to_end(&mut output), stderr.read_to_end(&mut diagnostics))
        } => result.map(|(status, _, _)| status),
        _ = cancelled(shutdown) => {
            // Descendants may retain their write ends; shutdown never waits for pipe EOF.
            drop(stdout);
            drop(stderr);
            child.kill().await.map_err(|source| orchestrator::Error::Io { operation: "reap cancelled Holoscan CLI", source })?;
            let status = child.wait().await.map_err(|source| orchestrator::Error::Io { operation: "read cancelled Holoscan CLI status", source })?;
            return Ok(QueryDiagnostic {
                argv, stdout: output, stderr: diagnostics, termination: termination(status), error: "Closing".into(),
            });
        }
    };
    let (status, error) = match result {
        Ok(status) => (Some(status), String::new()),
        Err(error) => {
            child
                .kill()
                .await
                .map_err(|source| orchestrator::Error::Io {
                    operation: "reap failed Holoscan CLI",
                    source,
                })?;
            (None, error.to_string())
        }
    };
    Ok(QueryDiagnostic {
        argv,
        stdout: output,
        stderr: diagnostics,
        termination: status.and_then(termination),
        error,
    })
}

fn termination(status: std::process::ExitStatus) -> Option<Termination> {
    status
        .code()
        .map(Termination::ExitCode)
        .or_else(|| status.signal().map(Termination::Signal))
}

fn check_query(catalog: &mut Catalog) -> bool {
    let query = catalog.diagnostics.last().unwrap();
    if query.termination == Some(Termination::ExitCode(0)) && query.error.is_empty() {
        return true;
    }
    let stderr = String::from_utf8_lossy(&query.stderr);
    let unavailable = query.error.starts_with("start Holoscan CLI:")
        || (query.termination == Some(Termination::ExitCode(2))
            && ([
                "unrecognized arguments: --json",
                "unrecognized arguments: --project-root",
                "unrecognized arguments: --language",
                "invalid choice: 'list'",
                "invalid choice: 'modes'",
            ]
            .iter()
            .any(|cause| stderr.contains(cause))));
    catalog.outcome = if unavailable {
        CatalogOutcome::Unavailable
    } else {
        CatalogOutcome::Failed
    } as i32;
    catalog.error = if !query.error.is_empty() {
        query.error.clone()
    } else {
        format!(
            "Holoscan query failed ({:?}): {}",
            query.termination,
            stderr.trim()
        )
    };
    false
}
fn failed(mut catalog: Catalog, error: impl Into<String>) -> Catalog {
    catalog.outcome = CatalogOutcome::Failed as i32;
    catalog.error = error.into();
    catalog
}
fn decode<T: serde::de::DeserializeOwned>(query: &QueryDiagnostic) -> Result<T, String> {
    serde_json::from_slice(&query.stdout)
        .map_err(|error| format!("unreadable Holoscan JSON response: {error}"))
}
#[derive(Deserialize)]
struct Listed {
    schema_version: u32,
    projects: Vec<ListedEntry>,
}
#[derive(Deserialize)]
struct ListedEntry {
    name: String,
    project_type: String,
    source_folder: String,
    language: Vec<String>,
    modes: Vec<String>,
}
#[derive(Deserialize)]
struct Modes {
    schema_version: u32,
    project: String,
    language: Vec<String>,
    modes: BTreeMap<String, ModeDescription>,
}
#[derive(Deserialize)]
struct ModeDescription {
    #[serde(default)]
    description: String,
}
