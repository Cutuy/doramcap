//! Inspect explicit configuration inputs and launch the captured bytes after preflight.
use crate::{
    ConfigurationSnapshot, DiscardedRun, PreparationInput, PreparedRun, Result, RunPlan,
    SystemdPlan, error,
};
use orchestrator::{LocalExecutor, Step};
use prost::Message;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, Metadata, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

/// Private runtime copies beneath a run's artifact directory.
const INPUTS: &str = "inputs";
/// Capture token used only in file-source fields before the private path is known.
const ARTIFACTS: &str = "@artifacts";
/// Linux POSIX access ACL; named-user access is independent of the basic mode bits.
const ACCESS_ACL: &str = "system.posix_acl_access";
/// Only the owner may traverse private run/input directories.
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
/// Input copies are private until their original access metadata is restored.
const PRIVATE_FILE_MODE: u32 = 0o600;
/// One sequential capture's temporary file within the new private input directory.
const CAPTURE_FILE: &str = ".capture";

/// Fingerprint explicit configuration through `InspectRun`, with blocking I/O off the caller's task.
/// Performs no writes or systemd calls; callers rediscover project units before inspection and Start.
/// Register `InspectRun` on an existing executor to supply an observer and retain invocation lineage.
pub async fn inspect(run: &RunPlan, plan: &SystemdPlan) -> Result<ConfigurationSnapshot> {
    let mut executor = LocalExecutor::default();
    executor.register::<InspectRun>(|| InspectRun);
    executor
        .run::<InspectRun>(PreparationInput {
            run: Some(run.clone()),
            plan: Some(plan.clone()),
        })
        .await
        .map(|output| output.into_value())
        .map_err(error)
}

/// Validate structure and fingerprint unit, graph and read-only regular-file inputs on this host.
/// Host shm, live directories, writable binds and binaries retain their native behavior.
pub struct InspectRun;
impl Step for InspectRun {
    const ID: Option<&'static str> = Some("systemd.inspect");
    const DISPLAY_NAME: Option<&'static str> = Some("Inspect systemd inputs");
    type Input = PreparationInput;
    type Output = ConfigurationSnapshot;

    async fn run(
        &mut self,
        input: PreparationInput,
    ) -> orchestrator::Result<ConfigurationSnapshot> {
        tokio::task::spawn_blocking(move || -> Result<ConfigurationSnapshot> {
            let (run, plan) = inputs(input)?;
            Ok(capture(run, plan, None)?
                .snapshot
                .expect("captured snapshot"))
        })
        .await
        .map_err(|e| orchestrator::Error::message(error(e)))?
        .map_err(orchestrator::Error::message)
    }
}

/// Capture inputs into a new private artifact tree; await this step to retain cleanup ownership.
/// Partial preparation failures remove their owned tree before returning an error.
/// Dropping the call does not stop its blocking worker or undo its effects.
pub struct PrepareRun;
impl Step for PrepareRun {
    const ID: Option<&'static str> = Some("systemd.prepare");
    const DISPLAY_NAME: Option<&'static str> = Some("Prepare systemd inputs");
    type Input = PreparationInput;
    type Output = PreparedRun;

    async fn run(&mut self, input: PreparationInput) -> orchestrator::Result<PreparedRun> {
        tokio::task::spawn_blocking(move || {
            let (run, plan) = inputs(input)?;
            prepare(run, plan)
        })
        .await
        .map_err(|e| orchestrator::Error::message(error(e)))?
        .map_err(orchestrator::Error::message)
    }
}

/// Remove artifacts owned by a successful preparation after its preflight is rejected.
/// Pass a successful `PrepareRun` output while its tree is still owned and no units have launched.
/// Await completion; failures retain the affected path in their diagnostic.
pub struct DiscardRun;
impl Step for DiscardRun {
    const ID: Option<&'static str> = Some("systemd.discard");
    const DISPLAY_NAME: Option<&'static str> = Some("Discard systemd preparation");
    type Input = PreparedRun;
    type Output = DiscardedRun;

    async fn run(&mut self, prepared: PreparedRun) -> orchestrator::Result<DiscardedRun> {
        tokio::task::spawn_blocking(move || -> Result<DiscardedRun> {
            let run = prepared.run.ok_or("prepared run missing")?;
            remove(Path::new(&run.artifacts))?;
            Ok(DiscardedRun {})
        })
        .await
        .map_err(|e| orchestrator::Error::message(error(e)))?
        .map_err(orchestrator::Error::message)
    }
}

fn inputs(input: PreparationInput) -> Result<(RunPlan, SystemdPlan)> {
    Ok((
        input.run.ok_or("run inputs missing")?,
        input.plan.ok_or("systemd plan missing")?,
    ))
}

struct InputWriter {
    hash: Sha256,
    content_hash: Option<Sha256>,
    file: Option<File>,
}
impl Write for InputWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Some(file) = &mut self.file {
            file.write_all(bytes)?;
        }
        self.hash.update(bytes);
        if let Some(hash) = &mut self.content_hash {
            hash.update(bytes);
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.as_mut().map_or(Ok(()), Write::flush)
    }
}

fn input(source: &str, name: &str, destination: Option<&Path>) -> Result<String> {
    capture_files(&[PathBuf::from(source)], name, destination).map(|(path, _)| path)
}

/// Capture the unit and its adjacent native drop-ins into one ordered unit file.
fn unit_input(source: &str, name: &str, destination: Option<&Path>) -> Result<String> {
    let directory = PathBuf::from(format!("{source}.d"));
    let mut drop_ins = Vec::new();
    match std::fs::read_dir(&directory) {
        Ok(entries) => {
            for entry in entries {
                let path = entry.map_err(error)?.path();
                if path
                    .extension()
                    .is_some_and(|extension| extension == "conf")
                {
                    drop_ins.push(path);
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("{}: {e}", directory.display())),
    }
    drop_ins.sort();
    let mut sources = vec![PathBuf::from(source)];
    for path in drop_ins {
        // A /dev/null symlink is systemd's native way to mask a drop-in.
        if std::fs::canonicalize(&path).map_err(error)? != Path::new("/dev/null") {
            sources.push(path);
        }
    }
    capture_files(&sources, name, destination).map(|(path, _)| path)
}

fn capture_files(
    sources: &[PathBuf],
    name: &str,
    destination: Option<&Path>,
) -> Result<(String, Option<String>)> {
    let copy = destination
        .map(|directory| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(PRIVATE_FILE_MODE)
                .open(directory.join(CAPTURE_FILE))
                .map_err(error)
        })
        .transpose()?;
    let mut writer = InputWriter {
        hash: Sha256::new(),
        content_hash: (name == "dataflow").then(Sha256::new),
        file: copy,
    };
    let mut access = None;
    for (index, source) in sources.iter().enumerate() {
        let mut read = || -> Result<(Metadata, Option<Vec<u8>>)> {
            // Follow native symlinks once; never block on FIFOs.
            let mut file = OpenOptions::new()
                .read(true)
                .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32)
                .open(source)
                .map_err(error)?;
            let metadata = file.metadata().map_err(error)?;
            if !metadata.is_file() {
                return Err("configuration input must be a regular file".into());
            }
            let acl = acl(&file)?;
            if index != 0 {
                writer.write_all(b"\n").map_err(error)?;
            }
            writer.hash.update(metadata.mode().to_le_bytes());
            writer.hash.update(metadata.uid().to_le_bytes());
            writer.hash.update(metadata.gid().to_le_bytes());
            let access = acl.as_deref().unwrap_or_default();
            writer.hash.update((access.len() as u64).to_le_bytes());
            writer.hash.update(access);
            // Stream the observed size so growing inputs cannot cause unbounded capture.
            let copied = if name == "dataflow" {
                let mut bytes = Vec::new();
                (&mut file)
                    .take(metadata.len())
                    .read_to_end(&mut bytes)
                    .map_err(error)?;
                let graph = workload_lifecycle::dora::graph(
                    source.to_str().ok_or("graph path is not UTF-8")?,
                    &bytes,
                )?;
                writer
                    .write_all(&serde_json::to_vec(&graph).map_err(error)?)
                    .map_err(error)?;
                bytes.len() as u64
            } else {
                std::io::copy(&mut (&mut file).take(metadata.len()), &mut writer).map_err(error)?
            };
            let after = file.metadata().map_err(error)?;
            if copied != metadata.len()
                || metadata.len() != after.len()
                || metadata.mtime() != after.mtime()
                || metadata.mtime_nsec() != after.mtime_nsec()
                || metadata.ctime() != after.ctime()
                || metadata.ctime_nsec() != after.ctime_nsec()
            {
                return Err("configuration input changed while being captured".into());
            }
            Ok((metadata, acl))
        };
        let observed = read().map_err(|e| format!("{}: {e}", source.display()))?;
        if access.is_none() {
            access = Some(observed);
        }
    }
    let content_hash = writer.content_hash.map(|hash| hex::encode(hash.finalize()));
    let key = format!("{}/{name}", hex::encode(writer.hash.finalize()));
    if let (Some(directory), Some(copy), Some((metadata, acl))) = (destination, writer.file, access)
    {
        restore_access(&copy, &metadata, acl.as_deref())?;
        let target = directory.join(&key);
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(PRIVATE_DIRECTORY_MODE)
            .create(target.parent().expect("input directory"))
            .map_err(error)?;
        std::fs::rename(directory.join(CAPTURE_FILE), target).map_err(error)?;
    }
    Ok((format!("{ARTIFACTS}/{INPUTS}/{key}"), content_hash))
}

fn restore_access(file: &File, metadata: &Metadata, acl: Option<&[u8]>) -> Result<()> {
    // Ownership precedes mode/ACL restoration because chown may clear special mode bits.
    let current = file.metadata().map_err(error)?;
    if (current.uid(), current.gid()) != (metadata.uid(), metadata.gid()) {
        rustix::fs::fchown(
            file,
            Some(rustix::fs::Uid::from_raw(metadata.uid())),
            Some(rustix::fs::Gid::from_raw(metadata.gid())),
        )
        .map_err(error)?;
    }
    file.set_permissions(std::fs::Permissions::from_mode(metadata.mode()))
        .map_err(error)?;
    if let Some(acl) = acl {
        rustix::fs::fsetxattr(file, ACCESS_ACL, acl, rustix::fs::XattrFlags::empty())
            .map_err(error)?;
    } else {
        // A parent default ACL must not grant new readers to an input with no source ACL.
        match rustix::fs::fremovexattr(file, ACCESS_ACL) {
            Ok(()) | Err(rustix::io::Errno::NODATA | rustix::io::Errno::NOTSUP) => {}
            Err(e) => return Err(error(e)),
        }
    }
    Ok(())
}

fn acl(file: &File) -> Result<Option<Vec<u8>>> {
    let size = match rustix::fs::fgetxattr(file, ACCESS_ACL, &mut [0u8; 0]) {
        Ok(size) => size,
        Err(rustix::io::Errno::NODATA | rustix::io::Errno::NOTSUP) => return Ok(None),
        Err(e) => return Err(error(e)),
    };
    let mut bytes = vec![0; size];
    let read = rustix::fs::fgetxattr(file, ACCESS_ACL, bytes.as_mut_slice()).map_err(error)?;
    bytes.truncate(read);
    Ok(Some(bytes))
}

/// Stream inputs into one new private tree; failures remove only that newly owned tree.
fn prepare(run: RunPlan, plan: SystemdPlan) -> Result<PreparedRun> {
    crate::validate(&run, &plan)?;
    let artifacts = PathBuf::from(&run.artifacts);
    std::fs::DirBuilder::new()
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(&artifacts)
        .map_err(error)?;
    let prepare = || -> Result<PreparedRun> {
        let directory = artifacts.join(INPUTS);
        std::fs::DirBuilder::new()
            .mode(PRIVATE_DIRECTORY_MODE)
            .create(&directory)
            .map_err(error)?;
        let mut prepared = capture(run, plan, Some(&directory))?;
        let run = prepared.run.as_mut().expect("captured run");
        let plan = prepared.plan.as_mut().expect("captured plan");
        let root = run.artifacts.clone();
        // Rewrite only captured file fields; arbitrary program text is never a capture token.
        replace(&mut run.dataflow_file, ARTIFACTS, &root);
        for file in &mut plan.unit_files {
            replace(file, ARTIFACTS, &root);
        }
        for app in &mut plan.app_units {
            replace(&mut app.file, ARTIFACTS, &root);
        }
        for bind in plan
            .programs
            .iter_mut()
            .flat_map(|program| &mut program.binds)
            .chain(plan.app_units.iter_mut().flat_map(|app| &mut app.binds))
        {
            replace(&mut bind.source, ARTIFACTS, &root);
        }
        Ok(prepared)
    };
    prepare().map_err(|e| discard(&artifacts, e))
}

fn capture(
    mut run: RunPlan,
    mut plan: SystemdPlan,
    destination: Option<&Path>,
) -> Result<PreparedRun> {
    crate::validate(&run, &plan)?;
    for file in &mut plan.unit_files {
        let name = format!("{}.service", crate::unit_stem(file)?);
        *file = unit_input(file, &name, destination)?;
    }
    for app in &mut plan.app_units {
        app.file = unit_input(&app.file, &format!("{}.service", app.id), destination)?;
    }
    for bind in plan
        .programs
        .iter_mut()
        .flat_map(|program| &mut program.binds)
        .chain(plan.app_units.iter_mut().flat_map(|app| &mut app.binds))
    {
        let source = Path::new(&bind.source);
        let metadata = std::fs::metadata(source);
        if metadata.is_ok() && host_filesystem::is_host_shm_source(source).map_err(error)? {
            bind.source = std::fs::canonicalize(source)
                .map_err(error)?
                .to_string_lossy()
                .into_owned();
            continue;
        }
        if !bind.read_only {
            continue;
        }
        match metadata {
            Ok(metadata) if metadata.is_file() => {
                bind.source = input(&bind.source, "file", destination)?
            }
            Ok(_) => {}
            // These exact directories are created by the runner before any unit starts.
            Err(e)
                if e.kind() == std::io::ErrorKind::NotFound
                    && (source == Path::new(&run.artifacts)
                        || source == Path::new(&run.artifacts).join("dora/runtime")) => {}
            Err(e) => return Err(format!("{}: {e}", bind.source)),
        }
    }
    let mut dataflow_sha256 = String::new();
    if !run.dataflow_file.is_empty() {
        let (path, hash) = capture_files(
            &[PathBuf::from(&run.dataflow_file)],
            "dataflow",
            destination,
        )?;
        run.dataflow_file = path;
        dataflow_sha256 = hash.expect("dataflow capture checksum");
    }
    let mut normalized_run = run.clone();
    let mut normalized_plan = plan.clone();
    normalize(&mut normalized_run, &mut normalized_plan, &run.artifacts);
    normalized_run.id.clear();
    let mut bytes = normalized_run.encode_length_delimited_to_vec();
    normalized_plan
        .encode_length_delimited(&mut bytes)
        .map_err(error)?;
    let snapshot = ConfigurationSnapshot {
        sha256: hex::encode(Sha256::digest(bytes)),
    };
    Ok(PreparedRun {
        run: Some(run),
        plan: Some(plan),
        snapshot: Some(snapshot),
        dataflow_sha256,
    })
}

/// Remove only the new private artifacts owned by rejected preparation; retain cleanup errors.
fn discard(artifacts: &Path, cause: String) -> String {
    match remove(artifacts) {
        Ok(()) => cause,
        Err(e) => format!("{cause}; {e}"),
    }
}

fn remove(artifacts: &Path) -> Result<()> {
    std::fs::remove_dir_all(artifacts).map_err(|e| {
        format!(
            "could not remove rejected preparation {}: {e}",
            artifacts.display()
        )
    })
}

fn replace(path: &mut String, root: &str, replacement: &str) {
    if path == root {
        *path = replacement.into();
    } else if let Some(relative) = path
        .strip_prefix(root)
        .and_then(|rest| rest.strip_prefix('/'))
    {
        *path = format!("{replacement}/{relative}");
    }
}

// Tag every candidate so a caller's literal text cannot collide with a normalized runtime path.
fn normalize_value(value: &mut String, root: &str) {
    if value == root {
        *value = "relative:".into();
    } else if let Some(relative) = value
        .strip_prefix(root)
        .and_then(|rest| rest.strip_prefix('/'))
    {
        *value = format!("relative:{relative}");
    } else {
        *value = format!("literal:{value}");
    }
}

fn normalize(run: &mut RunPlan, plan: &mut SystemdPlan, root: &str) {
    normalize_value(&mut run.artifacts, root);
    normalize_value(&mut run.dataflow_file, root);
    for recording in &mut run.recordings {
        normalize_value(&mut recording.stream, root);
        normalize_value(&mut recording.recording, root);
    }
    for program in &mut plan.programs {
        for arg in &mut program.argv {
            normalize_value(arg, root);
        }
        normalize_value(&mut program.working_directory, root);
        for env in &mut program.environment {
            normalize_value(&mut env.value, root);
        }
        for bind in &mut program.binds {
            normalize_value(&mut bind.source, root);
            normalize_value(&mut bind.target, root);
        }
    }
    for file in &mut plan.unit_files {
        normalize_value(file, root);
    }
    for app in &mut plan.app_units {
        normalize_value(&mut app.file, root);
        for bind in &mut app.binds {
            normalize_value(&mut bind.source, root);
            normalize_value(&mut bind.target, root);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bind, Program};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "systemd-review-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn file(&self, name: &str, contents: &str) -> String {
            let path = self.0.join(name);
            std::fs::write(&path, contents).unwrap();
            path.to_string_lossy().into()
        }
        fn run(&self, name: &str) -> RunPlan {
            RunPlan {
                id: name.into(),
                repository_root: self.0.to_string_lossy().into(),
                artifacts: self.0.join(name).to_string_lossy().into(),
                readiness_seconds: 10,
                stop_seconds: 5,
                host_shm_access: workload_lifecycle::launch::HostShmAccess::Passthrough as i32,
                ..Default::default()
            }
        }
        fn plan(&self) -> SystemdPlan {
            SystemdPlan {
                unit_files: vec![self.file("worker.service", "[Service]\nExecStart=/bin/true\n")],
                programs: vec![Program {
                    id: "monitor".into(),
                    argv: vec!["/bin/true".into()],
                    binds: vec![Bind {
                        source: self.file("monitor.pbtxt", "sample_interval_ms: 1000\n"),
                        target: "/config/monitor.pbtxt".into(),
                        read_only: true,
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn preparation_steps_report_capture_and_discard_lineage() {
        use std::{cell::RefCell, rc::Rc};
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("execution");
        let plan = fixture.plan();
        let input = PreparationInput {
            run: Some(run.clone()),
            plan: Some(plan.clone()),
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let mut executor = LocalExecutor::new(move |event: &orchestrator::Event| {
            observed.borrow_mut().push(event.clone());
        });
        executor.register::<InspectRun>(|| InspectRun);
        executor.register::<PrepareRun>(|| PrepareRun);
        executor.register::<DiscardRun>(|| DiscardRun);
        // Act
        let review = executor.run::<InspectRun>(input.clone()).await.unwrap();
        let prepared = executor.run::<PrepareRun>(input).await.unwrap();
        // Assert: both serialized entry points preserve the same identity and captured bytes.
        assert_eq!(prepared.value().snapshot.as_ref(), Some(review.value()));
        std::fs::write(&plan.unit_files[0], "changed original").unwrap();
        let captured = &prepared.value().plan.as_ref().unwrap().unit_files[0];
        assert_eq!(
            std::fs::read_to_string(captured).unwrap(),
            "[Service]\nExecStart=/bin/true\n"
        );
        let producer = prepared.invocation().id;
        // Act
        executor.run::<DiscardRun>(prepared).await.unwrap();
        // Assert
        assert!(!Path::new(&run.artifacts).exists());
        assert_eq!(
            std::fs::read_to_string(&plan.unit_files[0]).unwrap(),
            "changed original"
        );
        let events = events.borrow();
        for (events, id) in events.as_chunks::<2>().0.iter().zip([
            "systemd.inspect",
            "systemd.prepare",
            "systemd.discard",
        ]) {
            assert_eq!(events[0].invocation.step, id);
            assert_eq!(events[0].state, orchestrator::State::Started);
            assert_eq!(events[1].state, orchestrator::State::Succeeded);
        }
        assert_eq!(events.len(), 6);
        assert_eq!(events.last().unwrap().invocation.dependencies, [producer]);
    }

    #[tokio::test]
    async fn failed_preparation_cleans_partial_copies_before_reporting_failure() {
        use std::{cell::RefCell, rc::Rc};
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("execution");
        let mut plan = fixture.plan();
        plan.programs[0].binds[0].source = fixture.0.join("missing.pbtxt").to_string_lossy().into();
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let artifacts = run.artifacts.clone();
        let mut executor = LocalExecutor::new(move |event: &orchestrator::Event| {
            if event.state == orchestrator::State::Failed {
                assert!(!Path::new(&artifacts).exists());
            }
            observed.borrow_mut().push(event.clone());
        });
        executor.register::<PrepareRun>(|| PrepareRun);
        // Act
        let error = executor
            .run::<PrepareRun>(PreparationInput {
                run: Some(run.clone()),
                plan: Some(plan.clone()),
            })
            .await
            .unwrap_err();
        // Assert
        assert!(error.to_string().contains("missing.pbtxt"));
        assert!(!Path::new(&run.artifacts).exists());
        assert!(Path::new(&plan.unit_files[0]).exists());
        assert_eq!(executor.observer_failures(), 0);
        let events = events.borrow();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].state, orchestrator::State::Failed);
        assert!(events[1].error.as_ref().unwrap().contains("missing.pbtxt"));
    }

    #[tokio::test]
    async fn discard_failure_reports_its_owned_path_and_keeps_replacement() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("execution");
        let mut executor = LocalExecutor::default();
        executor.register::<PrepareRun>(|| PrepareRun);
        executor.register::<DiscardRun>(|| DiscardRun);
        let prepared = executor
            .run::<PrepareRun>(PreparationInput {
                run: Some(run.clone()),
                plan: Some(fixture.plan()),
            })
            .await
            .unwrap();
        std::fs::remove_dir_all(&run.artifacts).unwrap();
        std::fs::write(&run.artifacts, "replacement").unwrap();
        // Act
        let error = executor.run::<DiscardRun>(prepared).await.unwrap_err();
        // Assert
        assert!(
            error
                .to_string()
                .contains("could not remove rejected preparation")
        );
        assert!(error.to_string().contains(&run.artifacts));
        assert_eq!(
            std::fs::read_to_string(&run.artifacts).unwrap(),
            "replacement"
        );
    }

    #[tokio::test]
    async fn independent_artifacts_and_generated_config_paths_compare_by_contents() {
        // Arrange
        let fixture = Fixture::new();
        let mut review = fixture.run("review");
        let mut plan = fixture.plan();
        for id in ["coordinator", "daemon"] {
            review.dora_apps.push(id.into());
            plan.programs.push(Program {
                id: id.into(),
                argv: vec!["/bin/true".into(), id.into(), "--port".into(), "0".into()],
                ..Default::default()
            });
        }
        review.dataflow_file = fixture.file("review.graph.json", "{\"nodes\":[]}");
        plan.programs[0].binds.push(Bind {
            source: review.artifacts.clone(),
            target: "/results".into(),
            read_only: false,
        });
        let mut execution = review.clone();
        execution.id = "execution".into();
        execution.artifacts = fixture.run("execution").artifacts;
        execution.dataflow_file = fixture.file("execution.graph.json", "{\"nodes\":[]}");
        let mut fresh = plan.clone();
        fresh.programs[0].binds[0].source =
            fixture.file("execution.bridge.pbtxt", "sample_interval_ms: 1000\n");
        fresh.programs[0].binds[1].source = execution.artifacts.clone();
        // Act
        let expected = inspect(&review, &plan).await.unwrap();
        let actual = inspect(&execution, &fresh).await.unwrap();
        // Assert
        assert_eq!(expected, actual);
        assert!(!Path::new(&review.artifacts).exists());
        assert!(!Path::new(&execution.artifacts).exists());
        // Act / Assert: generated names do not hide changed graph/config contents.
        std::fs::write(
            &execution.dataflow_file,
            "{\"nodes\":[{\"id\":\"new\",\"path\":\"dynamic\"}]}",
        )
        .unwrap();
        assert_ne!(expected, inspect(&execution, &fresh).await.unwrap());
        std::fs::write(&execution.dataflow_file, "{\"nodes\":[]}").unwrap();
        std::fs::write(
            &fresh.programs[0].binds[0].source,
            "sample_interval_ms: 2000\n",
        )
        .unwrap();
        assert_ne!(expected, inspect(&execution, &fresh).await.unwrap());
    }

    #[tokio::test]
    async fn launch_settings_selection_and_unit_contents_make_a_review_stale() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("review");
        let plan = fixture.plan();
        let expected = inspect(&run, &plan).await.unwrap();
        // Act / Assert
        let mut changed = run.clone();
        changed.stop_seconds += 1;
        assert_ne!(expected, inspect(&changed, &plan).await.unwrap());
        let mut changed = plan.clone();
        changed.programs[0].argv.push("changed".into());
        assert_ne!(expected, inspect(&run, &changed).await.unwrap());
        changed = plan.clone();
        changed.unit_files.clear();
        assert_ne!(expected, inspect(&run, &changed).await.unwrap());
        changed = plan.clone();
        changed
            .unit_files
            .push(fixture.file("added.service", "[Service]\nExecStart=/bin/true\n"));
        assert_ne!(expected, inspect(&run, &changed).await.unwrap());
        std::fs::write(&plan.unit_files[0], "[Service]\nExecStart=/bin/false\n").unwrap();
        assert_ne!(expected, inspect(&run, &plan).await.unwrap());
        std::fs::remove_file(&plan.unit_files[0]).unwrap();
        assert!(
            inspect(&run, &plan)
                .await
                .unwrap_err()
                .contains("worker.service")
        );
    }

    #[tokio::test]
    async fn captured_files_survive_source_edits_and_symlink_changes_with_mode_preserved() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("execution");
        let mut plan = fixture.plan();
        let source = plan.programs[0].binds[0].source.clone();
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o640)).unwrap();
        let link = fixture.0.join("config-link");
        std::os::unix::fs::symlink(&source, &link).unwrap();
        plan.programs[0].binds[0].source = link.to_string_lossy().into();
        let captured = prepare(run.clone(), plan.clone()).unwrap();
        // Act
        std::fs::write(&source, "changed").unwrap();
        std::fs::write(&plan.unit_files[0], "changed unit").unwrap();
        std::fs::remove_file(&link).unwrap();
        let replacement = fixture.file("replacement", "replacement");
        std::os::unix::fs::symlink(replacement, &link).unwrap();
        let frozen = captured.plan.unwrap();
        let snapshot = captured.snapshot.unwrap();
        // Assert
        assert_eq!(
            std::fs::read_to_string(&frozen.programs[0].binds[0].source).unwrap(),
            "sample_interval_ms: 1000\n"
        );
        assert_eq!(
            std::fs::read_to_string(&frozen.unit_files[0]).unwrap(),
            "[Service]\nExecStart=/bin/true\n"
        );
        assert_eq!(
            std::fs::metadata(&frozen.programs[0].binds[0].source)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
        assert_eq!(crate::unit_stem(&frozen.unit_files[0]).unwrap(), "worker");
        assert_ne!(snapshot, inspect(&run, &plan).await.unwrap());
    }

    #[tokio::test]
    async fn readonly_host_shm_stays_live_through_a_symlink_at_an_alternate_target() {
        // Arrange
        let fixture = Fixture::new();
        let shm = std::path::Path::new("/dev/shm").join(fixture.0.file_name().unwrap());
        std::fs::create_dir(&shm).unwrap();
        let live = Fixture(shm);
        let source = live.file("camera", "first frame");
        let link = fixture.0.join("camera-link");
        std::os::unix::fs::symlink(&source, &link).unwrap();
        let run = fixture.run("execution");
        let mut plan = fixture.plan();
        plan.programs[0].binds.push(Bind {
            source: link.to_string_lossy().into(),
            target: "/input/camera".into(),
            read_only: true,
        });
        let expected = inspect(&run, &plan).await.unwrap();

        // Act
        let prepared = prepare(run.clone(), plan.clone()).unwrap();
        std::fs::write(&source, "next frame").unwrap();
        let live_snapshot = inspect(&run, &plan).await.unwrap();

        // Assert
        let binding = &prepared.plan.as_ref().unwrap().programs[0].binds[1];
        assert_eq!(binding.source, source);
        assert_eq!(binding.target, "/input/camera");
        assert!(binding.read_only);
        assert_eq!(
            std::fs::read_to_string(&binding.source).unwrap(),
            "next frame"
        );
        assert_eq!(expected, prepared.snapshot.unwrap());
        assert_eq!(expected, live_snapshot);

        // Act: change the source object in the reviewed declaration.
        let replacement = live.file("replacement", "third frame");
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(replacement, &link).unwrap();
        let changed_snapshot = inspect(&run, &plan).await.unwrap();
        // Assert
        assert_ne!(expected, changed_snapshot);
    }

    #[tokio::test]
    async fn adjacent_unit_drop_ins_are_captured_in_native_order_and_make_review_stale() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("execution");
        let plan = fixture.plan();
        let directory = PathBuf::from(format!("{}.d", plan.unit_files[0]));
        std::fs::create_dir(&directory).unwrap();
        let first = directory.join("10-first.conf");
        let last = directory.join("90-last.conf");
        std::fs::write(&first, "[Service]\nPrivateIPC=yes\n").unwrap();
        std::fs::write(&last, "[Service]\nPrivateIPC=no\n").unwrap();
        std::os::unix::fs::symlink("/dev/null", directory.join("99-masked.conf")).unwrap();
        let expected = inspect(&run, &plan).await.unwrap();

        // Act
        let prepared = prepare(run.clone(), plan.clone()).unwrap();
        std::fs::write(&last, "[Service]\nPrivateIPC=yes\n").unwrap();
        let changed_snapshot = inspect(&run, &plan).await.unwrap();

        // Assert
        let unit = &prepared.plan.as_ref().unwrap().unit_files[0];
        assert_eq!(
            std::fs::read_to_string(unit).unwrap(),
            "[Service]\nExecStart=/bin/true\n\n[Service]\nPrivateIPC=yes\n\n[Service]\nPrivateIPC=no\n"
        );
        assert_eq!(prepared.snapshot.unwrap(), expected);
        assert_ne!(changed_snapshot, expected);
    }

    #[tokio::test]
    async fn live_bind_paths_remain_native_and_missing_explicit_readonly_inputs_fail() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("execution");
        let mut plan = fixture.plan();
        let directory = fixture.0.join("live");
        std::fs::create_dir(&directory).unwrap();
        plan.programs[0].binds.extend([
            Bind {
                source: directory.to_string_lossy().into(),
                target: "/live".into(),
                read_only: true,
            },
            Bind {
                source: fixture.0.join("future-writable").to_string_lossy().into(),
                target: "/output".into(),
                read_only: false,
            },
            Bind {
                source: format!("{}/dora/runtime", run.artifacts),
                target: "/runtime".into(),
                read_only: true,
            },
        ]);
        // Act
        let captured = prepare(run.clone(), plan.clone()).unwrap();
        let frozen = captured.plan.unwrap();
        let expected = captured.snapshot.unwrap();
        // Assert
        assert_eq!(frozen.programs[0].binds[1..], plan.programs[0].binds[1..]);
        std::fs::write(directory.join("live-data"), "changed runtime data").unwrap();
        assert_eq!(expected, inspect(&run, &plan).await.unwrap());
        // Act / Assert: missing ordinary config files are never guessed to be generated.
        std::fs::remove_file(&plan.programs[0].binds[0].source).unwrap();
        assert!(
            inspect(&run, &plan)
                .await
                .unwrap_err()
                .contains("monitor.pbtxt")
        );
    }

    #[tokio::test]
    async fn caller_text_and_unrecognized_missing_runtime_files_are_not_reinterpreted() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("execution");
        let mut plan = fixture.plan();
        plan.programs[0].argv.push("@artifacts/user-value".into());
        plan.programs[0]
            .environment
            .push(crate::EnvironmentVariable {
                name: "TEXT".into(),
                value: "@artifacts/user-value".into(),
            });
        let captured = prepare(run.clone(), plan.clone()).unwrap();
        // Act
        let frozen = captured.plan.unwrap();
        // Assert
        assert_eq!(frozen.programs[0].argv, plan.programs[0].argv);
        assert_eq!(frozen.programs[0].environment, plan.programs[0].environment);
        // Act / Assert: a missing file beneath the artifacts root is still an explicit error.
        plan.programs[0].binds[0].source = format!("{}/missing.pbtxt", run.artifacts);
        assert!(
            inspect(&run, &plan)
                .await
                .unwrap_err()
                .contains("missing.pbtxt")
        );
    }

    #[tokio::test]
    async fn literal_text_cannot_impersonate_normalized_runtime_paths() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("review");
        let mut plan = fixture.plan();
        plan.programs[0]
            .argv
            .push(format!("{}/file", run.artifacts));
        plan.programs[0]
            .environment
            .push(crate::EnvironmentVariable {
                name: "TEXT".into(),
                value: format!("{}/file", run.artifacts),
            });
        let expected = inspect(&run, &plan).await.unwrap();
        // Act / Assert
        for literal in ["@artifacts/file", "relative:file", "literal:relative:file"] {
            let mut changed = plan.clone();
            *changed.programs[0].argv.last_mut().unwrap() = literal.into();
            assert_ne!(expected, inspect(&run, &changed).await.unwrap());
            changed = plan.clone();
            changed.programs[0].environment[0].value = literal.into();
            assert_ne!(expected, inspect(&run, &changed).await.unwrap());
        }
    }

    #[tokio::test]
    async fn nonregular_unit_inputs_fail_without_waiting_for_a_writer() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("review");
        let mut plan = fixture.plan();
        let fifo = fixture.0.join("waiting.service");
        rustix::fs::mkfifoat(
            rustix::fs::CWD,
            &fifo,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .unwrap();
        plan.unit_files[0] = fifo.to_string_lossy().into();
        // Act
        let result = inspect(&run, &plan).await;
        // Assert
        assert!(result.unwrap_err().contains("regular file"));
        assert!(!Path::new(&run.artifacts).exists());
    }

    #[tokio::test]
    async fn file_access_changes_make_a_review_stale_and_existing_artifacts_are_preserved() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("execution");
        let plan = fixture.plan();
        std::fs::set_permissions(
            &plan.programs[0].binds[0].source,
            std::fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        let expected = inspect(&run, &plan).await.unwrap();
        // Act / Assert
        std::fs::set_permissions(
            &plan.programs[0].binds[0].source,
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        assert_ne!(expected, inspect(&run, &plan).await.unwrap());
        std::fs::create_dir(&run.artifacts).unwrap();
        let retained = Path::new(&run.artifacts).join("existing");
        std::fs::write(&retained, "keep").unwrap();
        assert!(prepare(run, plan).is_err());
        assert_eq!(std::fs::read_to_string(retained).unwrap(), "keep");
    }
}
