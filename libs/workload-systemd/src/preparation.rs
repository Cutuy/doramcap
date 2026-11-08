//! Inspect explicit configuration inputs and launch the captured bytes after preflight.
use crate::{ConfigurationSnapshot, Result, RunPlan, SystemdPlan, error};
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

/// Synchronously validate structure and fingerprint unit, graph and read-only regular-file inputs.
/// Performs no writes or systemd calls; live directories, writable binds and binaries stay native.
/// Callers rediscover project units before inspecting or executing; runtime readiness is checked at launch.
pub fn inspect(run: &RunPlan, plan: &SystemdPlan) -> Result<ConfigurationSnapshot> {
    Ok(capture(run.clone(), plan.clone(), None)?.snapshot)
}

struct InputWriter {
    hash: Sha256,
    file: Option<File>,
}
impl Write for InputWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Some(file) = &mut self.file {
            file.write_all(bytes)?;
        }
        self.hash.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.as_mut().map_or(Ok(()), Write::flush)
    }
}

fn input(source: &str, name: &str, destination: Option<&Path>) -> Result<String> {
    let read = || -> Result<String> {
        // Follow native symlinks once; validate the opened handle without blocking on FIFOs.
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
            file: copy,
        };
        writer.hash.update(metadata.mode().to_le_bytes());
        writer.hash.update(metadata.uid().to_le_bytes());
        writer.hash.update(metadata.gid().to_le_bytes());
        let access = acl.as_deref().unwrap_or_default();
        writer.hash.update((access.len() as u64).to_le_bytes());
        writer.hash.update(access);
        // Stream the observed size so growing inputs cannot cause unbounded capture.
        let copied =
            std::io::copy(&mut (&mut file).take(metadata.len()), &mut writer).map_err(error)?;
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
        let key = format!("{}/{name}", hex::encode(writer.hash.finalize()));
        if let (Some(directory), Some(copy)) = (destination, writer.file) {
            restore_access(&copy, &metadata, acl.as_deref())?;
            let target = directory.join(&key);
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(PRIVATE_DIRECTORY_MODE)
                .create(target.parent().expect("input directory"))
                .map_err(error)?;
            // The key includes bytes and access metadata, so equivalent copies can share a path.
            std::fs::rename(directory.join(CAPTURE_FILE), target).map_err(error)?;
        }
        Ok(format!("{ARTIFACTS}/{INPUTS}/{key}"))
    };
    read().map_err(|e| format!("{source}: {e}"))
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

/// The protobuf launch inputs now reference private captured files owned by this artifact tree.
pub(crate) struct Prepared {
    pub run: RunPlan,
    pub plan: SystemdPlan,
    pub snapshot: ConfigurationSnapshot,
}

/// Stream inputs into one new private tree; failures remove only that newly owned tree.
pub(crate) fn prepare(run: RunPlan, plan: SystemdPlan) -> Result<Prepared> {
    crate::validate(&run, &plan)?;
    let artifacts = PathBuf::from(&run.artifacts);
    std::fs::DirBuilder::new()
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(&artifacts)
        .map_err(error)?;
    let prepare = || -> Result<Prepared> {
        let directory = artifacts.join(INPUTS);
        std::fs::DirBuilder::new()
            .mode(PRIVATE_DIRECTORY_MODE)
            .create(&directory)
            .map_err(error)?;
        let mut prepared = capture(run, plan, Some(&directory))?;
        let root = prepared.run.artifacts.clone();
        // Rewrite only captured file fields; arbitrary program text is never a capture token.
        replace(&mut prepared.run.dataflow_file, ARTIFACTS, &root);
        for file in &mut prepared.plan.unit_files {
            replace(file, ARTIFACTS, &root);
        }
        for app in &mut prepared.plan.app_units {
            replace(&mut app.file, ARTIFACTS, &root);
        }
        for bind in prepared
            .plan
            .programs
            .iter_mut()
            .flat_map(|program| &mut program.binds)
            .chain(
                prepared
                    .plan
                    .app_units
                    .iter_mut()
                    .flat_map(|app| &mut app.binds),
            )
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
) -> Result<Prepared> {
    crate::validate(&run, &plan)?;
    for file in &mut plan.unit_files {
        let name = format!("{}.service", crate::unit_stem(file)?);
        *file = input(file, &name, destination)?;
    }
    for app in &mut plan.app_units {
        app.file = input(&app.file, &format!("{}.service", app.id), destination)?;
    }
    for bind in plan
        .programs
        .iter_mut()
        .flat_map(|program| &mut program.binds)
        .chain(plan.app_units.iter_mut().flat_map(|app| &mut app.binds))
    {
        if !bind.read_only {
            continue;
        }
        let source = Path::new(&bind.source);
        match std::fs::metadata(source) {
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
    if !run.dataflow_file.is_empty() {
        run.dataflow_file = input(&run.dataflow_file, "dataflow", destination)?;
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
    Ok(Prepared {
        run,
        plan,
        snapshot,
    })
}

/// Remove only the new private artifacts owned by rejected preparation; retain cleanup errors.
pub(crate) fn discard(artifacts: &Path, cause: String) -> String {
    match std::fs::remove_dir_all(artifacts) {
        Ok(()) => cause,
        Err(e) => format!(
            "{cause}; could not remove rejected preparation {}: {e}",
            artifacts.display()
        ),
    }
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

    #[test]
    fn independent_artifacts_and_generated_config_paths_compare_by_contents() {
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
        let expected = inspect(&review, &plan).unwrap();
        let actual = inspect(&execution, &fresh).unwrap();
        // Assert
        assert_eq!(expected, actual);
        assert!(!Path::new(&review.artifacts).exists());
        assert!(!Path::new(&execution.artifacts).exists());
        // Act / Assert: generated names do not hide changed graph/config contents.
        std::fs::write(&execution.dataflow_file, "{\"nodes\":[{\"id\":\"new\"}]}").unwrap();
        assert_ne!(expected, inspect(&execution, &fresh).unwrap());
        std::fs::write(&execution.dataflow_file, "{\"nodes\":[]}").unwrap();
        std::fs::write(
            &fresh.programs[0].binds[0].source,
            "sample_interval_ms: 2000\n",
        )
        .unwrap();
        assert_ne!(expected, inspect(&execution, &fresh).unwrap());
    }

    #[test]
    fn launch_settings_selection_and_unit_contents_make_a_review_stale() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("review");
        let plan = fixture.plan();
        let expected = inspect(&run, &plan).unwrap();
        // Act / Assert
        let mut changed = run.clone();
        changed.stop_seconds += 1;
        assert_ne!(expected, inspect(&changed, &plan).unwrap());
        let mut changed = plan.clone();
        changed.programs[0].argv.push("changed".into());
        assert_ne!(expected, inspect(&run, &changed).unwrap());
        changed = plan.clone();
        changed.unit_files.clear();
        assert_ne!(expected, inspect(&run, &changed).unwrap());
        changed = plan.clone();
        changed
            .unit_files
            .push(fixture.file("added.service", "[Service]\nExecStart=/bin/true\n"));
        assert_ne!(expected, inspect(&run, &changed).unwrap());
        std::fs::write(&plan.unit_files[0], "[Service]\nExecStart=/bin/false\n").unwrap();
        assert_ne!(expected, inspect(&run, &plan).unwrap());
        std::fs::remove_file(&plan.unit_files[0]).unwrap();
        assert!(inspect(&run, &plan).unwrap_err().contains("worker.service"));
    }

    #[test]
    fn captured_files_survive_source_edits_and_symlink_changes_with_mode_preserved() {
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
        let Prepared {
            plan: frozen,
            snapshot,
            ..
        } = captured;
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
        assert_ne!(snapshot, inspect(&run, &plan).unwrap());
    }

    #[test]
    fn live_bind_paths_remain_native_and_missing_explicit_readonly_inputs_fail() {
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
        let Prepared {
            plan: frozen,
            snapshot: expected,
            ..
        } = captured;
        // Assert
        assert_eq!(frozen.programs[0].binds[1..], plan.programs[0].binds[1..]);
        std::fs::write(directory.join("live-data"), "changed runtime data").unwrap();
        assert_eq!(expected, inspect(&run, &plan).unwrap());
        // Act / Assert: missing ordinary config files are never guessed to be generated.
        std::fs::remove_file(&plan.programs[0].binds[0].source).unwrap();
        assert!(inspect(&run, &plan).unwrap_err().contains("monitor.pbtxt"));
    }

    #[test]
    fn caller_text_and_unrecognized_missing_runtime_files_are_not_reinterpreted() {
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
        let frozen = captured.plan;
        // Assert
        assert_eq!(frozen.programs[0].argv, plan.programs[0].argv);
        assert_eq!(frozen.programs[0].environment, plan.programs[0].environment);
        // Act / Assert: a missing file beneath the artifacts root is still an explicit error.
        plan.programs[0].binds[0].source = format!("{}/missing.pbtxt", run.artifacts);
        assert!(inspect(&run, &plan).unwrap_err().contains("missing.pbtxt"));
    }

    #[test]
    fn literal_text_cannot_impersonate_normalized_runtime_paths() {
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
        let expected = inspect(&run, &plan).unwrap();
        // Act / Assert
        for literal in ["@artifacts/file", "relative:file", "literal:relative:file"] {
            let mut changed = plan.clone();
            *changed.programs[0].argv.last_mut().unwrap() = literal.into();
            assert_ne!(expected, inspect(&run, &changed).unwrap());
            changed = plan.clone();
            changed.programs[0].environment[0].value = literal.into();
            assert_ne!(expected, inspect(&run, &changed).unwrap());
        }
    }

    #[test]
    fn nonregular_unit_inputs_fail_without_waiting_for_a_writer() {
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
        let result = inspect(&run, &plan);
        // Assert
        assert!(result.unwrap_err().contains("regular file"));
        assert!(!Path::new(&run.artifacts).exists());
    }

    #[test]
    fn file_access_changes_make_a_review_stale_and_existing_artifacts_are_preserved() {
        // Arrange
        let fixture = Fixture::new();
        let run = fixture.run("execution");
        let plan = fixture.plan();
        std::fs::set_permissions(
            &plan.programs[0].binds[0].source,
            std::fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        let expected = inspect(&run, &plan).unwrap();
        // Act / Assert
        std::fs::set_permissions(
            &plan.programs[0].binds[0].source,
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        assert_ne!(expected, inspect(&run, &plan).unwrap());
        std::fs::create_dir(&run.artifacts).unwrap();
        let retained = Path::new(&run.artifacts).join("existing");
        std::fs::write(&retained, "keep").unwrap();
        assert!(prepare(run, plan).is_err());
        assert_eq!(std::fs::read_to_string(retained).unwrap(), "keep");
    }
}
