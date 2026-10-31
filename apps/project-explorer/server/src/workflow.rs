//! Gallery bindings for the shared Docker session workflow and result publication.
use crate::{
    lifecycle::Token,
    registry::{ArtifactCollection, NestedProject, RecordingCount, Registration, ResultFormat},
};
use std::{
    fs::{self, DirBuilder},
    os::unix::fs::DirBuilderExt,
    path::Path,
    rc::Rc,
};
use tokio::sync::oneshot;
use workload_docker::compose::DockerAdapter;
use workload_docker::evidence::write_private_once;
pub use workload_docker::workflows::compose_lifecycle::StopReport;
use workload_docker::workflows::{
    recorded_compose::{ComposeRun, ComposeRunner},
    recovery::{RecoveryGroup, reconcile_group, recover_project},
};

fn private_dir(path: &Path) -> std::io::Result<()> {
    DirBuilder::new().mode(0o700).create(path)
}

/// Run one app to completion. The caller owns the task, signals `stop` when its
/// sessions expire, and awaits cleanup even when its readiness receiver is gone.
/// Docker startup, health inspection, shutdown and evidence are owned by the
/// shared runner. Only successfully finalized declared results become public.
pub async fn run(
    adapter: Rc<DockerAdapter>,
    app: &Registration,
    generation: Token,
    root: &Path,
    (uid, gid): (u32, u32),
    ready: oneshot::Sender<Option<std::net::SocketAddr>>,
    stop: oneshot::Receiver<()>,
) -> StopReport {
    let mut runner = ComposeRunner::new(adapter.clone());
    let mut report = StopReport {
        quiesced: false,
        cleanup_verified: true,
        errors: vec![],
        changes: None,
    };
    let directory = root.join(format!("{}-{generation:032x}", app.id));
    let result = async {
        if !root.is_absolute() || fs::canonicalize(root).map_err(|e| e.to_string())? != root {
            return Err("generation root must be absolute and canonical".to_owned());
        }
        private_dir(&directory).map_err(|e| e.to_string())?;
        let metadata = directory.join("metadata");
        private_dir(&metadata).map_err(|e| e.to_string())?;
        write_private_once(&metadata.join("additional-declared.json"), &app.additional_files)?;
        write_private_once(&metadata.join("collections-declared.json"), &app.artifact_collections)?;
        write_private_once(&metadata.join("primary-policy.json"), &primary_policy(app))?;
        if app.nested_runs { write_private_once(&metadata.join("nested-policy.json"), &app.nested_projects)?; }
        let results = directory.join("results");
        private_dir(&results).map_err(|e| e.to_string())?;
        if !app.private_directories.is_empty() {
            let private = directory.join("private");
            private_dir(&private).map_err(|e| e.to_string())?;
            for name in &app.private_directories { private_dir(&private.join(name)).map_err(|e| e.to_string())?; }
        }
        let mut project = app.project(&results, uid, gid).map_err(str::to_owned)?;
        project.environment.insert(app.environment_key("RUN_ID"), format!("gal-{generation:032x}"));
        if app.docker_socket {
            use std::os::unix::fs::{FileTypeExt, MetadataExt};
            let path = adapter.socket_path();
            let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
            if !metadata.file_type().is_socket() { return Err("Docker endpoint is not a Unix socket".into()); }
            project.environment.insert(app.environment_key("DOCKER_SOCKET"), path.to_str().ok_or("Docker socket path is not UTF-8")?.into());
            project.environment.insert(app.environment_key("DOCKER_GID"), metadata.gid().to_string());
        }
        let mut system_apps = app.system_apps.clone().unwrap_or_default();
        for flow in &mut system_apps.dora_flows { flow.results_directory = results.to_str().ok_or("results path is not UTF-8")?.into(); }
        let spec = ComposeRun {
            id: format!("gal-{generation:032x}"), artifacts: metadata.join("docker"),
            filesystem: None, system_apps, recovery_anchor_dir: None,
            projects: vec![project], first_party: vec![],
            readiness_seconds: app.readiness_seconds, stop_seconds: app.stop_seconds,
        };
        let (started, receiver) = oneshot::channel();
        let (cancel, cancelled) = oneshot::channel::<()>();
        let completion = runner.run_with_ready(spec, async {
            tokio::select! { _ = stop => {}, _ = cancelled => {} }
        }, |_: &orchestrator::Event| {}, Some(started));
        tokio::pin!(completion);
        let endpoint_error = tokio::select! {
            result = &mut completion => return result.map_err(|e| e.to_string()),
            plans = receiver => {
                match plans {
                    Ok(plans) => {
                        let endpoint = match (app.port, plans.iter().find(|plan| plan.key == app.id)) {
                            (Some(port), Some(plan)) => adapter.project(plan).endpoint(&app.service, &app.network, port as u16).await.map(Some).map_err(|e| e.to_string()),
                            (None, _) => Ok(None),
                            _ => Err("ready workflow has no app project".into()),
                        };
                        match endpoint {
                            Ok(endpoint) => ready.send(endpoint).err().map(|_| "readiness receiver dropped".to_owned()),
                            Err(error) => Some(error),
                        }
                    }
                    Err(_) => Some("workflow stopped before becoming ready".to_owned()),
                }
            }
        };
        if endpoint_error.is_some() { let _ = cancel.send(()); }
        // Keep the cancellation sender alive until normal completion.
        let mut outcome = completion.await.map_err(|e| e.to_string())?;
        if let Some(error) = endpoint_error { outcome.execution_error = Some(error); }
        Ok(outcome)
    }.await;
    match result {
        Ok(outcome) => {
            report.quiesced = outcome.writers_stopped;
            report.cleanup_verified = outcome.boundary_clean;
            report.changes = outcome.filesystem_changes;
            report.errors.extend(outcome.execution_error);
            report.errors.extend(
                outcome
                    .cleanup
                    .iter()
                    .flat_map(|cleanup| &cleanup.diagnostics)
                    .map(|d| format!("{}: {}", d.operation, d.message)),
            );
            report.errors.extend(
                outcome
                    .diagnostics
                    .iter()
                    .map(|d| format!("{}: {}", d.operation, d.message)),
            );
        }
        Err(error) => {
            report.cleanup_verified = !runner.blocked();
            report.errors.push(error);
        }
    }
    finish(adapter, app, generation, &directory, &mut report).await;
    report
}

/// Recover a generation under the manager's exclusive lease. Saved app policies
/// are checked here; Docker ownership, evidence and teardown use shared recovery.
pub async fn recover(
    adapter: Rc<DockerAdapter>,
    app: &Registration,
    generation: Token,
    root: &Path,
) -> Result<StopReport, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let directory = root.join(format!("{}-{generation:032x}", app.id));
    let metadata = directory.join("metadata");
    let descriptor = workload_docker::evidence::root(&metadata).map_err(|e| e.to_string())?;
    let read = |name: &str| -> Result<serde_json::Value, String> {
        use std::io::Read;
        let mut bytes = Vec::new();
        workload_docker::evidence::open(&descriptor, name, false)
            .map_err(|e| e.to_string())?
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("saved app policy is too large".into());
        }
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    };
    for (name, expected) in [
        (
            "primary-policy.json",
            serde_json::json!(primary_policy(app)),
        ),
        (
            "additional-declared.json",
            serde_json::json!(app.additional_files),
        ),
        (
            "collections-declared.json",
            serde_json::json!(app.artifact_collections),
        ),
    ] {
        if read(name)? != expected {
            return Err(format!("saved app policy changed: {name}"));
        }
    }
    if app.nested_runs && read("nested-policy.json")? != serde_json::json!(app.nested_projects) {
        return Err("nested project policy changed".into());
    }
    // Older generations stored Docker metadata directly beside app policy.
    let docker = if metadata
        .join("docker")
        .try_exists()
        .map_err(|e| e.to_string())?
    {
        metadata.join("docker")
    } else {
        metadata
    };
    let mut report = recover_project(
        adapter.clone(),
        &docker,
        &format!("gal-{generation:032x}"),
        &app.id,
        false,
        app.stop_seconds,
    )
    .await?;
    finish(adapter, app, generation, &directory, &mut report).await;
    Ok(report)
}

async fn finish(
    adapter: Rc<DockerAdapter>,
    app: &Registration,
    generation: Token,
    directory: &Path,
    report: &mut StopReport,
) {
    if app.nested_runs {
        stop_and_verify_nested(
            &adapter,
            directory,
            &format!("gal-{generation:032x}"),
            &app.nested_projects,
            app.stop_seconds,
            report,
        )
        .await;
    }
    if !report.cleanup_verified {
        report
            .errors
            .push("result publication requires verified cleanup".into());
    } else if report.errors.is_empty()
        && let Err(error) = publish_results(app, directory).await
    {
        report.errors.push(format!("result finalization: {error}"));
    }
}

/// Validate the declared outputs, freeze their indexes, then publish one marker.
/// Normal completion and crash recovery use this same publication operation.
async fn publish_results(app: &Registration, directory: &Path) -> Result<(), String> {
    let metadata = directory.join("metadata");
    let results = directory.join("results");
    let recording = results.join(&app.result_file);
    verify_outputs(
        &recording,
        app.result_format(),
        app.result_optional,
        app.capture_directory
            .as_ref()
            .map(|name| results.join(name))
            .as_deref(),
        app.shutdown_marker
            .as_ref()
            .map(|name| results.join(name))
            .as_deref(),
        app.recording_count.as_ref(),
        &app.additional_files,
    )
    .await?;
    finalize_additional_files(&metadata, &results, &app.additional_files)?;
    finalize_collections(&metadata, &results, &app.artifact_collections).await?;
    finalize_primary(&metadata, &recording, app.result_optional)?;
    mark_recording_complete(&metadata).await
}

async fn mark_recording_complete(metadata: &Path) -> Result<(), String> {
    use std::{
        io::{Read, Write},
        os::unix::fs::OpenOptionsExt,
    };
    let marker = metadata.join("recording.complete");
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&marker)
    {
        Ok(mut file) => {
            file.write_all(b"verified\n").map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let mut file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(marker)
                .map_err(|e| e.to_string())?;
            let mut content = String::new();
            file.read_to_string(&mut content)
                .map_err(|e| e.to_string())?;
            if content == "verified\n" {
                Ok(())
            } else {
                Err("invalid completion marker".into())
            }
        }
        Err(error) => Err(error.to_string()),
    }
}

fn finalize_additional_files(
    metadata: &Path,
    results: &Path,
    declared: &[String],
) -> Result<(), String> {
    let mut present = Vec::new();
    for name in declared {
        match std::fs::symlink_metadata(results.join(name)) {
            Ok(info) if info.file_type().is_file() => present.push(name.clone()),
            Ok(_) => return Err(format!("additional file {name} is not regular")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.to_string()),
        }
    }
    write_private_once(&metadata.join("additional-index.json"), &present)
}

/// Frozen list of verified paths for one declared collection.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishedCollection {
    pub id: String,
    pub files: Vec<String>,
}

async fn finalize_collections(
    metadata: &Path,
    results: &Path,
    declared: &[ArtifactCollection],
) -> Result<(), String> {
    let mut published = Vec::new();
    if declared.is_empty() {
        return write_private_once(&metadata.join("collections-index.json"), &published)
            .map_err(|e| e.to_string());
    }
    let root = crate::artifact_fs::root(results).map_err(|e| e.to_string())?;
    for collection in declared {
        let base = collection.directory.as_deref().unwrap_or("");
        let base_dir = if base.is_empty() {
            root.try_clone().map_err(|e| e.to_string())?
        } else {
            match crate::artifact_fs::open(&root, base, true) {
                Ok(dir) => dir,
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound && !collection.required =>
                {
                    published.push(PublishedCollection {
                        id: collection.id.clone(),
                        files: Vec::new(),
                    });
                    continue;
                }
                Err(error) => {
                    return Err(format!("collection {} directory: {error}", collection.id));
                }
            }
        };
        let directories = if collection.run_directories {
            crate::artifact_fs::names(&base_dir)
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|name| collection.matches_run(name))
                .map(|name| {
                    if base.is_empty() {
                        name
                    } else {
                        format!("{base}/{name}")
                    }
                })
                .collect::<Vec<_>>()
        } else {
            vec![base.to_owned()]
        };
        if directories.len() > 512 {
            return Err(format!(
                "collection {} exceeds its run directory limit",
                collection.id
            ));
        }
        let mut files = Vec::new();
        for directory in directories {
            let dir = if directory.is_empty() {
                root.try_clone().map_err(|e| e.to_string())?
            } else {
                crate::artifact_fs::open(&root, &directory, true).map_err(|e| {
                    format!("collection {} directory {directory}: {e}", collection.id)
                })?
            };
            for name in crate::artifact_fs::names(&dir).map_err(|e| e.to_string())? {
                if !collection.matches(&name) {
                    continue;
                }
                if files.len() == collection.max_files as usize {
                    return Err(format!(
                        "collection {} exceeds its file limit",
                        collection.id
                    ));
                }
                let path = if directory.is_empty() {
                    name
                } else {
                    format!("{directory}/{name}")
                };
                let file = crate::artifact_fs::open(&root, &path, false)
                    .map_err(|e| format!("collection {} file {path}: {e}", collection.id))?;
                if collection.format() == ResultFormat::Mcap {
                    verify_recording_file(file, None)
                        .await
                        .map_err(|e| format!("collection {} file {path}: {e}", collection.id))?;
                }
                files.push(path);
            }
        }
        if collection.required && files.is_empty() {
            return Err(format!("collection {} is empty", collection.id));
        }
        files.sort();
        published.push(PublishedCollection {
            id: collection.id.clone(),
            files,
        });
    }
    write_private_once(&metadata.join("collections-index.json"), &published)
}

fn primary_policy(app: &Registration) -> Vec<String> {
    vec![
        app.result_file.clone(),
        match app.result_format() {
            ResultFormat::Mcap => "mcap",
            ResultFormat::File => "file",
        }
        .into(),
        if app.result_optional {
            "optional"
        } else {
            "required"
        }
        .into(),
        app.capture_directory.clone().unwrap_or_default(),
        app.shutdown_marker.clone().unwrap_or_default(),
        app.recording_count
            .as_ref()
            .map(|check| check.count_file.clone())
            .unwrap_or_default(),
        app.recording_count
            .as_ref()
            .map(|check| check.topic.clone())
            .unwrap_or_default(),
    ]
}

fn finalize_primary(metadata: &Path, result: &Path, optional: bool) -> Result<(), String> {
    let state = match std::fs::symlink_metadata(result) {
        Ok(info) if info.file_type().is_file() => "present",
        Ok(_) => return Err("primary result is not regular".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && optional => "absent",
        Err(error) => return Err(error.to_string()),
    };
    let name = result
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid primary result name")?;
    write_private_once(&metadata.join("primary-state.json"), &[state, name])
}

/// Registered capture-file names are numeric and cannot select arbitrary paths.
pub fn capture_name(name: &str) -> bool {
    name.strip_prefix("capture-")
        .and_then(|s| s.strip_suffix(".mcap"))
        .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

fn verify_shutdown_marker(path: &Path) -> Result<(), String> {
    use std::{io::Read, os::unix::fs::OpenOptionsExt};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| format!("shutdown marker: {e}"))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("shutdown marker is not a regular file".into());
    }
    let mut contents = Vec::new();
    file.take(10)
        .read_to_end(&mut contents)
        .map_err(|e| e.to_string())?;
    if contents != b"complete\n" {
        return Err("ordered shutdown was not verified".into());
    }
    Ok(())
}

async fn verify_outputs(
    recording: &Path,
    result_format: ResultFormat,
    result_optional: bool,
    captures: Option<&Path>,
    shutdown_marker: Option<&Path>,
    recording_count: Option<&RecordingCount>,
    additional_files: &[String],
) -> Result<(), String> {
    if let Some(marker) = shutdown_marker {
        verify_shutdown_marker(marker)?;
    }
    let absent = match std::fs::symlink_metadata(recording) {
        Ok(_) => false,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && result_optional => true,
        Err(error) => return Err(error.to_string()),
    };
    if !absent {
        match result_format {
            ResultFormat::Mcap => {
                let expected = if let Some(check) = recording_count {
                    let directory = recording.parent().ok_or("result has no directory")?;
                    Some((
                        check.topic.as_str(),
                        read_count(&directory.join(&check.count_file))?,
                    ))
                } else {
                    None
                };
                verify_recording(recording, expected).await?;
            }
            ResultFormat::File => {
                use std::os::unix::fs::OpenOptionsExt;
                let file = std::fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                    .open(recording)
                    .map_err(|e| e.to_string())?;
                if !file.metadata().map_err(|e| e.to_string())?.is_file() {
                    return Err("result is not a regular file".into());
                }
            }
        }
    }
    let directory = recording.parent().ok_or("result has no directory")?;
    for name in additional_files {
        use std::os::unix::fs::OpenOptionsExt;
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(directory.join(name))
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("additional file {name}: {error}")),
        };
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err(format!("additional file {name} is not regular"));
        }
    }
    if let Some(directory) = captures {
        let metadata = tokio::fs::symlink_metadata(directory)
            .await
            .map_err(|e| e.to_string())?;
        if !metadata.file_type().is_dir() {
            return Err("capture directory is not a directory".into());
        }
        let mut entries = tokio::fs::read_dir(directory)
            .await
            .map_err(|e| e.to_string())?;
        let mut count = 0;
        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let name = entry.file_name();
            if name.to_str().is_some_and(capture_name) {
                verify_recording(&entry.path(), None).await?;
                count += 1;
            }
        }
        if count == 0 {
            return Err("no finalized capture recordings".into());
        }
    }
    Ok(())
}

/// Read the producer's bounded decimal delivery count from a regular file.
fn read_count(path: &Path) -> Result<u64, String> {
    use std::{io::Read, os::unix::fs::OpenOptionsExt};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| format!("published count: {e}"))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("published count is not a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(32)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let digits = bytes.strip_suffix(b"\n").ok_or("invalid published count")?;
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return Err("invalid published count".into());
    }
    std::str::from_utf8(digits)
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())
}

/// Verify the MCAP record stream incrementally, including its final footer and
/// trailing magic. Memory grows with the largest record, not the whole recording.
async fn verify_recording(path: &Path, expected: Option<(&str, u64)>) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("recording is not a regular file".into());
    }
    verify_recording_file(file, expected).await
}

async fn verify_recording_file(
    file: std::fs::File,
    expected: Option<(&str, u64)>,
) -> Result<(), String> {
    let options = mcap::tokio::linear_reader::LinearReaderOptions::default()
        .with_check_finishes_after_end_magic(true)
        .with_validate_chunk_crcs(true)
        .with_validate_data_section_crc(true)
        .with_validate_summary_section_crc(true);
    let mut reader =
        mcap::tokio::LinearReader::new_with_options(tokio::fs::File::from_std(file), &options);
    let mut buffer = Vec::new();
    let mut footer = false;
    let mut channels = std::collections::HashMap::new();
    let mut count = 0u64;
    while let Some(record) = reader.next_record(&mut buffer).await {
        let opcode = record.map_err(|e| e.to_string())?;
        match mcap::parse_record(opcode, &buffer).map_err(|e| e.to_string())? {
            mcap::records::Record::Channel(channel) => {
                if let Some(previous) = channels.insert(channel.id, channel.topic.clone())
                    && previous != channel.topic
                {
                    return Err("MCAP channel changed topic".into());
                }
            }
            mcap::records::Record::Message { header, .. } if expected.is_some() => {
                let topic = channels
                    .get(&header.channel_id)
                    .ok_or("MCAP message has no channel")?;
                if expected.is_some_and(|(expected_topic, _)| topic == expected_topic) {
                    count = count.checked_add(1).ok_or("MCAP message count overflow")?;
                }
            }
            _ => {}
        }
        footer |= opcode == mcap::records::op::FOOTER;
    }
    if !footer {
        return Err("MCAP footer missing".into());
    }
    if let Some((topic, published)) = expected
        && count != published
    {
        return Err(format!(
            "MCAP topic {topic}: published {published}, recorded {count}"
        ));
    }
    Ok(())
}

/// Stop writers, collect category-preserving filesystem metadata, and remove the
/// owned project. The result directory remains on disk. Callers must preserve the
/// report and block replacement if `cleanup_verified` is false.
async fn stop_and_verify_nested(
    adapter: &Rc<DockerAdapter>,
    generation: &Path,
    run: &str,
    policy: &[NestedProject],
    stop_seconds: u64,
    report: &mut StopReport,
) {
    if report.quiesced
        && let Err(error) =
            reconcile_nested(adapter.clone(), generation, run, policy, stop_seconds, true).await
    {
        eprintln!("nested shutdown failed: {error}");
        report
            .errors
            .push("nested shutdown is incomplete; evidence remains private".into());
    }
    verify_no_nested_resources(adapter, run, report).await;
}

async fn verify_no_nested_resources(adapter: &DockerAdapter, run: &str, report: &mut StopReport) {
    match adapter.resources().discover_runs(&format!("{run}-")).await {
        Ok(resources) if resources.is_empty() => {}
        Ok(_) => {
            report.cleanup_verified = false;
            report
                .errors
                .push("nested Docker resources require recovery".into());
        }
        Err(error) => {
            report.cleanup_verified = false;
            eprintln!("nested Docker discovery failed: {error}");
            report
                .errors
                .push("nested Docker cleanup could not be verified".into());
        }
    }
}

async fn reconcile_nested(
    adapter: Rc<DockerAdapter>,
    generation: &Path,
    parent_run: &str,
    policy: &[NestedProject],
    stop_seconds: u64,
    preserve_only: bool,
) -> Result<(), String> {
    reconcile_group(
        adapter,
        RecoveryGroup {
            artifacts_root: &generation.join("results/runs"),
            anchors: &generation.join("private/recovery"),
            parent_run,
            policy,
            stop_seconds,
            preserve_only,
            attempt: rand::random(),
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn file_result_requires_a_regular_file() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("gallery-result-{:032x}", rand::random::<u128>()));
        std::fs::create_dir(&root).unwrap();
        let result = root.join("samples.pb");
        let link = root.join("link");
        // Act / Assert
        assert!(
            verify_outputs(&result, ResultFormat::File, false, None, None, None, &[])
                .await
                .is_err()
        );
        // Act / Assert: a declared optional primary result may be absent.
        assert!(
            verify_outputs(&result, ResultFormat::File, true, None, None, None, &[])
                .await
                .is_ok()
        );
        std::fs::write(&result, b"synthetic telemetry").unwrap();
        assert!(
            verify_outputs(&result, ResultFormat::File, false, None, None, None, &[])
                .await
                .is_ok()
        );
        assert!(
            verify_outputs(&result, ResultFormat::Mcap, false, None, None, None, &[])
                .await
                .is_err()
        );
        std::os::unix::fs::symlink(&result, &link).unwrap();
        assert!(
            verify_outputs(&link, ResultFormat::File, false, None, None, None, &[])
                .await
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn optional_files_allow_absence_and_reject_symlinks() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("gallery-extra-{:032x}", rand::random::<u128>()));
        std::fs::create_dir(&root).unwrap();
        let result = root.join("result.json");
        std::fs::write(&result, b"{}").unwrap();
        let optional = vec!["diagnostics.log".to_string()];
        // Act / Assert: an absent optional file is valid.
        assert!(
            verify_outputs(
                &result,
                ResultFormat::File,
                false,
                None,
                None,
                None,
                &optional
            )
            .await
            .is_ok()
        );
        // Act / Assert: a normal retained file is valid.
        std::fs::write(root.join(&optional[0]), b"log").unwrap();
        assert!(
            verify_outputs(
                &result,
                ResultFormat::File,
                false,
                None,
                None,
                None,
                &optional
            )
            .await
            .is_ok()
        );
        // Act / Assert: a symlink cannot be published as an artifact.
        std::fs::remove_file(root.join(&optional[0])).unwrap();
        std::os::unix::fs::symlink(&result, root.join(&optional[0])).unwrap();
        assert!(
            verify_outputs(
                &result,
                ResultFormat::File,
                false,
                None,
                None,
                None,
                &optional
            )
            .await
            .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn final_artifact_index_is_idempotent_and_cannot_expand() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("gallery-index-{:032x}", rand::random::<u128>()));
        std::fs::create_dir(&root).unwrap();
        let metadata = root.join("metadata");
        let results = root.join("results");
        std::fs::create_dir(&metadata).unwrap();
        std::fs::create_dir(&results).unwrap();
        let declared = vec!["first.txt".into(), "later.txt".into()];
        std::fs::write(results.join("first.txt"), b"one").unwrap();
        // Act / Assert: the same verified set can be finalized again.
        finalize_additional_files(&metadata, &results, &declared).unwrap();
        finalize_additional_files(&metadata, &results, &declared).unwrap();
        // Act / Assert: a file created later cannot expand the published set.
        std::fs::write(results.join("later.txt"), b"two").unwrap();
        assert!(finalize_additional_files(&metadata, &results, &declared).is_err());
        let index: Vec<String> =
            serde_json::from_slice(&std::fs::read(metadata.join("additional-index.json")).unwrap())
                .unwrap();
        assert_eq!(index, ["first.txt"]);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn primary_policy_freezes_validation_requirements() {
        // Arrange
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .unwrap();
        let registry = crate::registry::Registry::load(root, &["cpu-viewer".into()]).unwrap();
        let app = registry.get("cpu-viewer").unwrap();
        let original = primary_policy(app);
        // Act / Assert: removing the delivery check changes the recovery policy.
        let mut changed = app.clone();
        changed.recording_count = None;
        assert_ne!(primary_policy(&changed), original);
        // Act / Assert: changing required output semantics also changes it.
        changed = app.clone();
        changed.result_optional = true;
        assert_ne!(primary_policy(&changed), original);
    }
    #[tokio::test]
    async fn special_output_files_do_not_block_validation() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("gallery-fifo-{:032x}", rand::random::<u128>()));
        std::fs::create_dir(&root).unwrap();
        let fifo = root.join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let result = root.join("result");
        std::fs::write(&result, b"{}").unwrap();
        // Act / Assert: a FIFO is rejected without waiting for a writer.
        assert!(verify_shutdown_marker(&fifo).is_err());
        assert!(read_count(&fifo).is_err());
        assert!(verify_recording(&fifo, None).await.is_err());
        assert!(
            verify_outputs(
                &result,
                ResultFormat::File,
                false,
                None,
                None,
                None,
                &["fifo".into()]
            )
            .await
            .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn shutdown_marker_must_be_complete_and_regular() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("gallery-shutdown-{:032x}", rand::random::<u128>()));
        std::fs::create_dir(&root).unwrap();
        let marker = root.join("shutdown.complete");
        let link = root.join("link");
        // Act / Assert
        assert!(verify_shutdown_marker(&marker).is_err());
        std::fs::write(&marker, b"partial\n").unwrap();
        assert!(verify_shutdown_marker(&marker).is_err());
        std::fs::write(&marker, b"complete\n").unwrap();
        assert!(verify_shutdown_marker(&marker).is_ok());
        std::os::unix::fs::symlink(&marker, &link).unwrap();
        assert!(verify_shutdown_marker(&link).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn published_count_requires_a_regular_decimal_file() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("gallery-published-{:032x}", rand::random::<u128>()));
        std::fs::create_dir(&root).unwrap();
        let count = root.join("published.count");
        let link = root.join("link");
        // Act / Assert
        assert!(read_count(&count).is_err());
        std::fs::write(&count, b"42\n").unwrap();
        assert_eq!(read_count(&count).unwrap(), 42);
        std::os::unix::fs::symlink(&count, &link).unwrap();
        assert!(read_count(&link).is_err());
        std::fs::write(&count, b"42\nmore").unwrap();
        assert!(read_count(&count).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn recording_validation_detects_truncated_finalization() {
        // Arrange
        let path =
            std::env::temp_dir().join(format!("gallery-mcap-{:032x}", rand::random::<u128>()));
        let mut writer = mcap::Writer::new(std::io::Cursor::new(Vec::new())).unwrap();
        writer.finish().unwrap();
        let bytes = writer.into_inner().into_inner();
        std::fs::write(&path, &bytes).unwrap();

        // Act / Assert
        assert!(verify_recording(&path, None).await.is_ok());
        std::fs::write(&path, &bytes[..bytes.len() - 8]).unwrap();
        assert!(verify_recording(&path, None).await.is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[tokio::test]
    async fn recording_validation_checks_crc_and_trailing_bytes() {
        // Arrange
        let path =
            std::env::temp_dir().join(format!("gallery-crc-{:032x}", rand::random::<u128>()));
        let mut writer = mcap::Writer::new(std::io::Cursor::new(Vec::new())).unwrap();
        let schema = writer.add_schema("sample", "protobuf", b"schema").unwrap();
        let channel = writer
            .add_channel(schema, "samples", "protobuf", &Default::default())
            .unwrap();
        let payload = b"unique-payload-for-crc";
        writer
            .write_to_known_channel(
                &mcap::records::MessageHeader {
                    channel_id: channel,
                    sequence: 0,
                    log_time: 1,
                    publish_time: 1,
                },
                payload,
            )
            .unwrap();
        writer.finish().unwrap();
        let bytes = writer.into_inner().into_inner();
        std::fs::write(&path, &bytes).unwrap();
        assert!(verify_recording(&path, None).await.is_ok());
        // Act / Assert: payload corruption with an intact structure is detected.
        let mut corrupted = bytes.clone();
        let offset = corrupted
            .windows(payload.len())
            .position(|window| window == payload)
            .unwrap();
        corrupted[offset] ^= 1;
        std::fs::write(&path, corrupted).unwrap();
        assert!(verify_recording(&path, None).await.is_err());
        // Act / Assert: trailing bytes after the closing magic are rejected.
        let mut trailing = bytes;
        trailing.extend_from_slice(b"extra");
        std::fs::write(&path, trailing).unwrap();
        assert!(verify_recording(&path, None).await.is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[tokio::test]
    async fn nested_collections_freeze_runs_and_reject_symlink_escapes() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("gallery-nested-{:032x}", rand::random::<u128>()));
        let results = root.join("results");
        let metadata = root.join("metadata");
        let run = "0123456789abcdef0123456789abcdef";
        std::fs::create_dir_all(results.join("runs").join(run)).unwrap();
        std::fs::create_dir(&metadata).unwrap();
        let collection = ArtifactCollection {
            id: "activity".into(),
            base_file: Some("producer.activity.pb".into()),
            prefix: "consumer-".into(),
            suffix: ".activity.pb".into(),
            digits: 10,
            variable_digits: true,
            max_files: 3,
            required: true,
            directory: Some("runs".into()),
            run_directories: true,
            run_prefix: None,
            generation_runs: false,
            format: ResultFormat::File.into(),
        };
        let file = results.join("runs").join(run).join("producer.activity.pb");
        std::fs::write(&file, b"activity").unwrap();
        std::fs::write(file.with_file_name("consumer-4.activity.pb"), b"activity").unwrap();
        std::fs::write(
            file.with_file_name("consumer-12345.activity.pb"),
            b"activity",
        )
        .unwrap();
        std::fs::write(file.with_file_name("run.log"), b"private").unwrap();

        // Act: finalize a run directory and its declared files.
        finalize_collections(&metadata, &results, std::slice::from_ref(&collection))
            .await
            .unwrap();
        // Assert: the index excludes undeclared logs and is immutable.
        let index: Vec<PublishedCollection> = serde_json::from_slice(
            &std::fs::read(metadata.join("collections-index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            index[0].files,
            vec![
                format!("runs/{run}/consumer-12345.activity.pb"),
                format!("runs/{run}/consumer-4.activity.pb"),
                format!("runs/{run}/producer.activity.pb"),
            ]
        );
        std::fs::write(file.with_file_name("consumer-0.activity.pb"), b"late").unwrap();
        assert!(
            finalize_collections(&metadata, &results, std::slice::from_ref(&collection))
                .await
                .is_err()
        );

        // Act / Assert: a trusted run prefix selects numbered experiments, not input directories.
        let prefixed_metadata = root.join("prefixed-metadata");
        std::fs::create_dir(&prefixed_metadata).unwrap();
        let prefixed = results.join("runs/doramcap-live-123-456-1");
        std::fs::create_dir(&prefixed).unwrap();
        std::fs::write(prefixed.join("producer.activity.pb"), b"experiment").unwrap();
        std::fs::create_dir(results.join("runs/doramcap-live-123-456-1-inputs")).unwrap();
        let mut prefixed_collection = collection.clone();
        prefixed_collection.run_prefix = Some("doramcap-live-".into());
        finalize_collections(&prefixed_metadata, &results, &[prefixed_collection])
            .await
            .unwrap();
        let index: Vec<PublishedCollection> = serde_json::from_slice(
            &std::fs::read(prefixed_metadata.join("collections-index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            index[0].files,
            vec!["runs/doramcap-live-123-456-1/producer.activity.pb"]
        );

        // Act / Assert: nested MCAP files are verified before they enter the index.
        let mcap_metadata = root.join("mcap-metadata");
        std::fs::create_dir(&mcap_metadata).unwrap();
        let mut mcap_collection = collection.clone();
        mcap_collection.id = "recordings".into();
        mcap_collection.base_file = Some("producer.mcap".into());
        mcap_collection.suffix = ".mcap".into();
        mcap_collection.format = ResultFormat::Mcap.into();
        let mut writer = mcap::Writer::new(std::io::Cursor::new(Vec::new())).unwrap();
        writer.finish().unwrap();
        let bytes = writer.into_inner().into_inner();
        let mcap_file = file.with_file_name("producer.mcap");
        std::fs::write(&mcap_file, &bytes).unwrap();
        finalize_collections(
            &mcap_metadata,
            &results,
            std::slice::from_ref(&mcap_collection),
        )
        .await
        .unwrap();
        let corrupt_metadata = root.join("corrupt-metadata");
        std::fs::create_dir(&corrupt_metadata).unwrap();
        std::fs::write(&mcap_file, &bytes[..bytes.len() - 8]).unwrap();
        assert!(
            finalize_collections(&corrupt_metadata, &results, &[mcap_collection])
                .await
                .is_err()
        );

        // Act / Assert: a run-directory symlink cannot escape the result root.
        let other_metadata = root.join("other-metadata");
        std::fs::create_dir(&other_metadata).unwrap();
        let outside = root.join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("producer.activity.pb"), b"outside").unwrap();
        std::os::unix::fs::symlink(
            &outside,
            results
                .join("runs")
                .join("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        )
        .unwrap();
        assert!(
            finalize_collections(&other_metadata, &results, &[collection])
                .await
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn oversized_artifact_index_is_rejected_before_creation() {
        // Arrange
        let path = std::env::temp_dir().join(format!(
            "gallery-large-index-{:032x}",
            rand::random::<u128>()
        ));
        let value = "x".repeat(2 * 1024 * 1024);
        // Act / Assert
        assert!(write_private_once(&path, &value).is_err());
        assert!(!path.exists());
    }
    #[tokio::test]
    async fn rotated_collection_verifies_and_freezes_bounded_files() {
        // Arrange
        let root = std::env::temp_dir().join(format!(
            "gallery-collection-{:032x}",
            rand::random::<u128>()
        ));
        std::fs::create_dir(&root).unwrap();
        let metadata = root.join("metadata");
        let results = root.join("results");
        std::fs::create_dir(&metadata).unwrap();
        std::fs::create_dir(&results).unwrap();
        let collection = ArtifactCollection {
            id: "recordings".into(),
            base_file: Some("faults.mcap".into()),
            prefix: "faults.mcap.".into(),
            suffix: "".into(),
            digits: 6,
            variable_digits: false,
            max_files: 2,
            required: true,
            directory: None,
            run_directories: false,
            run_prefix: None,
            generation_runs: false,
            format: ResultFormat::Mcap.into(),
        };
        let mut writer = mcap::Writer::new(std::io::Cursor::new(Vec::new())).unwrap();
        writer.finish().unwrap();
        let bytes = writer.into_inner().into_inner();
        // Act / Assert: required empty, corrupted and symlinked parts fail.
        assert!(
            finalize_collections(&metadata, &results, std::slice::from_ref(&collection))
                .await
                .is_err()
        );
        // Act / Assert: an optional collection may contain no recordings.
        let empty_metadata = root.join("empty-metadata");
        std::fs::create_dir(&empty_metadata).unwrap();
        let mut optional = collection.clone();
        optional.required = false;
        finalize_collections(&empty_metadata, &results, &[optional])
            .await
            .unwrap();
        std::fs::write(results.join("faults.mcap"), &bytes).unwrap();
        std::fs::write(
            results.join("faults.mcap.000001"),
            &bytes[..bytes.len() - 8],
        )
        .unwrap();
        assert!(
            finalize_collections(&metadata, &results, std::slice::from_ref(&collection))
                .await
                .is_err()
        );
        std::fs::remove_file(results.join("faults.mcap.000001")).unwrap();
        std::os::unix::fs::symlink(
            results.join("faults.mcap"),
            results.join("faults.mcap.000001"),
        )
        .unwrap();
        assert!(
            finalize_collections(&metadata, &results, std::slice::from_ref(&collection))
                .await
                .is_err()
        );
        // Act / Assert: matching regular files are verified and their names frozen.
        std::fs::remove_file(results.join("faults.mcap.000001")).unwrap();
        std::fs::write(results.join("faults.mcap.000001"), &bytes).unwrap();
        std::fs::write(results.join("faults.mcap.not-a-part"), b"ignored").unwrap();
        finalize_collections(&metadata, &results, std::slice::from_ref(&collection))
            .await
            .unwrap();
        let index: Vec<PublishedCollection> = serde_json::from_slice(
            &std::fs::read(metadata.join("collections-index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(index[0].files, ["faults.mcap", "faults.mcap.000001"]);
        // Act / Assert: an extra matching part exceeds the declared bound.
        std::fs::write(results.join("faults.mcap.000002"), &bytes).unwrap();
        assert!(
            finalize_collections(&metadata, &results, &[collection])
                .await
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn recording_count_checks_topic_delivery() {
        // Arrange
        let path =
            std::env::temp_dir().join(format!("gallery-count-{:032x}", rand::random::<u128>()));
        let mut writer = mcap::Writer::new(std::io::Cursor::new(Vec::new())).unwrap();
        let schema = writer.add_schema("sample", "protobuf", b"schema").unwrap();
        let channel = writer
            .add_channel(schema, "samples", "protobuf", &Default::default())
            .unwrap();
        writer
            .write_to_known_channel(
                &mcap::records::MessageHeader {
                    channel_id: channel,
                    sequence: 0,
                    log_time: 1,
                    publish_time: 1,
                },
                b"sample",
            )
            .unwrap();
        writer.finish().unwrap();
        std::fs::write(&path, writer.into_inner().into_inner()).unwrap();
        // Act / Assert
        assert!(verify_recording(&path, Some(("samples", 1))).await.is_ok());
        assert!(verify_recording(&path, Some(("samples", 2))).await.is_err());
        assert!(verify_recording(&path, Some(("other", 1))).await.is_err());
        std::fs::remove_file(path).unwrap();
    }
}
