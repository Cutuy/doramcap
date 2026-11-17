//! Authoritative per-selection Build attempts; this module alone derives store paths.
use crate::{
    BuildAttempt, BuildAttemptRequest, BuildKey, BuildLookup, BuildRecord, BuildResult,
    BuildStoreRequest, build_result,
};
use orchestrator::{Error, Result};
use prost::Message;
use std::{
    io::ErrorKind,
    os::unix::fs::PermissionsExt,
    path::{Component, Path, PathBuf},
};
use tokio::{fs, io::AsyncWriteExt};
use workload_lifecycle::admission::Admission;

const VERSION: u32 = 1;
const LATEST: &str = "latest.pb";
const ATTEMPTS: &str = "attempts";
const STDOUT: &str = "stdout.log";
const STDERR: &str = "stderr.log";

/// One admitted Build attempt, held through process/output cleanup and completion publication.
/// Dropping it releases admission without changing the last published state.
pub struct Attempt {
    admission: Option<Admission>,
    directory: PathBuf,
    artifacts: PathBuf,
    latest: BuildAttempt,
}
impl Attempt {
    /// Persist a fresh incomplete latest attempt before returning permission to launch.
    /// Rejects admission contention, invalid/nonprivate paths, reused IDs and failed durable writes.
    pub async fn begin(request: BuildAttemptRequest) -> Result<Self> {
        let store = request
            .store
            .ok_or_else(|| invalid("Build store is required"))?;
        let key = store.key.ok_or_else(|| invalid("Build key is required"))?;
        validate_selection(&key)?;
        validate_id(&request.id)?;
        absolute(&store.root, "Build store")?;
        absolute(&key.repository_root, "Build repository")?;
        let admission = Admission::acquire(Path::new(&key.repository_root))
            .map_err(|error| invalid(&error.to_string()))?;
        let key = BuildKey {
            repository_root: admission
                .repository()
                .to_str()
                .ok_or_else(|| invalid("Build repository must be UTF-8"))?
                .into(),
            ..key
        };
        let root = Path::new(&store.root);
        private_directory(root).await?;
        let directory = root.join(slot(&key));
        private_directory(&directory).await?;
        let attempts = directory.join(ATTEMPTS);
        private_directory(&attempts).await?;
        let artifacts = attempts.join(&request.id);
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&artifacts)
            .await
            .map_err(|error| io(&artifacts, "create new Build attempt", error))?;
        sync_directory(&attempts).await?;
        let latest = BuildAttempt {
            version: VERSION,
            key: Some(key),
            id: request.id,
            source: request.source,
            result: None,
        };
        replace(&directory, &latest).await?;
        sync_directory(&directory).await?;
        Ok(Self {
            admission: Some(admission),
            directory,
            artifacts,
            latest,
        })
    }

    /// New private directory for this attempt's streaming logs and owned launch evidence.
    pub fn artifacts(&self) -> &Path {
        &self.artifacts
    }
    /// Fixed stdout log path beneath `artifacts`; callers create it once and stream to disk.
    pub fn stdout_path(&self) -> PathBuf {
        self.artifacts.join(STDOUT)
    }
    /// Fixed stderr log path beneath `artifacts`; callers create it once and stream to disk.
    pub fn stderr_path(&self) -> PathBuf {
        self.artifacts.join(STDERR)
    }
    /// Canonical repository protected by this attempt's retained admission.
    pub fn repository(&self) -> &Path {
        Path::new(
            &self
                .latest
                .key
                .as_ref()
                .expect("validated Build key")
                .repository_root,
        )
    }

    /// Publish completion after cleanup, then release admission.
    /// Failed facts never publish a usable record.
    ///
    /// # Retries
    /// An identical terminal retry returns this attempt's saved receipt. It does not read
    /// the current store, republish an older attempt or confirm that it is still latest.
    ///
    /// # Errors
    /// Replacement failure leaves the previous publication. A later directory-sync failure
    /// retains the visible terminal attempt and admission until retry acknowledges durability.
    /// Lookup acquires admission and acknowledges durability before returning eligible evidence.
    pub async fn finish(&mut self, mut result: BuildResult) -> Result<BuildAttempt> {
        if !successful(&result) {
            result.record = None;
        }
        if let Some(record) = &result.record {
            validate_record(&self.latest, record)?;
        }
        if let Some(previous) = &self.latest.result {
            if previous != &result {
                return Err(invalid("Build attempt is already complete"));
            }
            if self.admission.is_some() {
                sync_directory(&self.directory).await?;
                self.admission = None;
            }
            return Ok(self.latest.clone());
        }
        let complete = BuildAttempt {
            result: Some(result),
            ..self.latest.clone()
        };
        replace(&self.directory, &complete).await?;
        self.latest = complete;
        sync_directory(&self.directory).await?;
        self.admission = None;
        Ok(self.latest.clone())
    }
}

/// Read the latest attempt under nonblocking repository admission; active Build/Run ownership is an error.
/// Missing is not built; malformed/unreadable input is an error, with no older-success fallback.
/// Incomplete/failed attempts retain diagnostics and supply no record to Prepare.
///
/// # Scope
/// Console supplies an owner-private data-root-derived directory. One complete latest record
/// is materialized in memory; errors are explicit and no record-size ceiling is imposed.
pub async fn lookup(request: BuildStoreRequest) -> Result<BuildLookup> {
    let key = request
        .key
        .as_ref()
        .ok_or_else(|| invalid("Build key is required"))?;
    absolute(&key.repository_root, "Build repository")?;
    let admission = Admission::acquire(Path::new(&key.repository_root))
        .map_err(|error| invalid(&error.to_string()))?;
    lookup_admitted(request, &admission).await
}

/// Read while the caller already holds admission for the same canonical repository.
/// The guard covers reading and directory-sync acknowledgement; contention is not double-locked.
pub async fn lookup_admitted(
    request: BuildStoreRequest,
    admission: &Admission,
) -> Result<BuildLookup> {
    absolute(&request.root, "Build store")?;
    let mut key = request
        .key
        .ok_or_else(|| invalid("Build key is required"))?;
    validate_selection(&key)?;
    absolute(&key.repository_root, "Build repository")?;
    let repository = fs::canonicalize(&key.repository_root)
        .await
        .map_err(|error| {
            io(
                Path::new(&key.repository_root),
                "resolve Build repository",
                error,
            )
        })?;
    if !fs::metadata(&repository)
        .await
        .map_err(|error| io(&repository, "inspect Build repository", error))?
        .is_dir()
    {
        return Err(invalid("Build repository must be a directory"));
    }
    if repository != admission.repository() {
        return Err(invalid(
            "Build lookup admission belongs to another repository",
        ));
    }
    key.repository_root = repository
        .to_str()
        .ok_or_else(|| invalid("Build repository must be UTF-8"))?
        .into();
    let root = Path::new(&request.root);
    if !check_directory(root).await? {
        return Ok(missing());
    }
    let directory = root.join(slot(&key));
    if !check_directory(&directory).await? {
        return Ok(missing());
    }
    let path = directory.join(LATEST);
    let metadata = match fs::symlink_metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(missing()),
        Err(error) => return Err(io(&path, "inspect latest Build", error)),
    };
    if !metadata.is_file() || metadata.permissions().mode() & 0o077 != 0 {
        return Err(invalid(&format!(
            "{} must be a private regular file",
            path.display()
        )));
    }
    let bytes = fs::read(&path)
        .await
        .map_err(|error| io(&path, "read latest Build", error))?;
    let latest = BuildAttempt::decode(bytes.as_slice())
        .map_err(|_| invalid(&format!("{} is not a Build attempt", path.display())))?;
    if latest.version != VERSION || latest.key.as_ref() != Some(&key) {
        return Err(invalid(&format!(
            "{} has an unsupported version or different Build key",
            path.display()
        )));
    }
    validate_id(&latest.id)?;
    let mut record = None;
    let mut diagnostics = Vec::new();
    match &latest.result {
        None => diagnostics.push("Latest Build attempt is incomplete".into()),
        Some(result) => {
            if let Some(candidate) = &result.record {
                validate_record(&latest, candidate)?;
                if !successful(result) {
                    return Err(invalid("Failed Build attempt contains a usable record"));
                }
                sync_directory(&directory).await?;
                record = Some(candidate.clone());
            }
            if record.is_none() {
                diagnostics
                    .push("Latest Build attempt has no complete usable Build evidence".into());
            }
            for (name, error) in [
                ("process", &result.process_error),
                ("output", &result.output_error),
                ("image", &result.image_error),
                ("cleanup", &result.cleanup_error),
            ] {
                if !error.is_empty() {
                    diagnostics.push(format!("Build {name}: {error}"));
                }
            }
        }
    }
    Ok(BuildLookup {
        latest: Some(latest),
        record,
        diagnostics,
    })
}

fn successful(result: &BuildResult) -> bool {
    result.termination == Some(build_result::Termination::ExitCode(0))
        && !result.cancelled
        && result.cleanup_verified
        && result.process_error.is_empty()
        && result.output_error.is_empty()
        && result.image_error.is_empty()
        && result.cleanup_error.is_empty()
}
fn validate_record(attempt: &BuildAttempt, record: &BuildRecord) -> Result<()> {
    if record.image_id.is_empty()
        || record.selection != attempt.key.as_ref().and_then(|key| key.selection.clone())
        || record.source != attempt.source
    {
        return Err(invalid(
            "Build evidence must match this attempt's selection/source and contain an image ID",
        ));
    }
    Ok(())
}
fn slot(key: &BuildKey) -> String {
    host_filesystem::files::digest(&key.encode_to_vec())
}
fn absolute(value: &str, name: &str) -> Result<()> {
    if !Path::new(value).is_absolute() {
        return Err(invalid(&format!("{name} must be absolute")));
    }
    Ok(())
}
fn validate_selection(key: &BuildKey) -> Result<()> {
    if key
        .selection
        .as_ref()
        .is_none_or(|selection| selection.application.is_empty())
    {
        return Err(invalid("Build application selection is required"));
    }
    Ok(())
}
fn validate_id(id: &str) -> Result<()> {
    let mut components = Path::new(id).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(invalid(
            "Build attempt ID must be one normal path component",
        ));
    }
    Ok(())
}
fn missing() -> BuildLookup {
    BuildLookup {
        diagnostics: vec!["No Build attempt is available".into()],
        ..Default::default()
    }
}
async fn check_directory(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.is_dir() && metadata.permissions().mode() & 0o077 == 0 => Ok(true),
        Ok(_) => Err(invalid(&format!(
            "{} must be a private directory",
            path.display()
        ))),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io(path, "inspect Build directory", error)),
    }
}
async fn private_directory(path: &Path) -> Result<()> {
    match fs::DirBuilder::new().mode(0o700).create(path).await {
        Ok(()) => {
            if let Some(parent) = path.parent() {
                sync_directory(parent).await?;
            }
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
        Err(error) => return Err(io(path, "create Build directory", error)),
    }
    if !check_directory(path).await? {
        return Err(invalid("Build directory disappeared during creation"));
    }
    Ok(())
}
#[cfg(test)]
thread_local! {
    static FAIL_DIRECTORY_ACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
async fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(test)]
    if FAIL_DIRECTORY_ACK.with(|fail| fail.get()) {
        return Err(Error::message(
            "injected directory durability acknowledgement failure",
        ));
    }
    fs::File::open(path)
        .await
        .map_err(|error| io(path, "open Build directory", error))?
        .sync_all()
        .await
        .map_err(|error| io(path, "sync Build directory", error))
}
async fn replace(directory: &Path, attempt: &BuildAttempt) -> Result<()> {
    let temporary = directory.join(format!("latest.{}.next", attempt.id));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .await
        .map_err(|error| io(&temporary, "create Build publication", error))?;
    let result = async {
        file.write_all(&attempt.encode_to_vec())
            .await
            .map_err(|error| io(&temporary, "write Build publication", error))?;
        file.sync_all()
            .await
            .map_err(|error| io(&temporary, "sync Build publication", error))?;
        drop(file);
        fs::rename(&temporary, directory.join(LATEST))
            .await
            .map_err(|error| io(&temporary, "replace latest Build", error))
    }
    .await;
    if result.is_err() {
        match fs::remove_file(&temporary).await {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Error::message(format!(
                    "{}; remove Build publication {}: {error}",
                    result.unwrap_err(),
                    temporary.display()
                )));
            }
        }
    }
    result
}
fn invalid(message: &str) -> Error {
    Error::Configuration(message.into())
}
fn io(path: &Path, operation: &str, error: std::io::Error) -> Error {
    Error::message(format!("{operation} {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Selection;
    use std::{
        fs as sync_fs,
        os::unix::fs::DirBuilderExt,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct FailAck;
    impl FailAck {
        fn new() -> Self {
            FAIL_DIRECTORY_ACK.with(|fail| fail.set(true));
            Self
        }
    }
    impl Drop for FailAck {
        fn drop(&mut self) {
            FAIL_DIRECTORY_ACK.with(|fail| fail.set(false));
        }
    }

    #[tokio::test]
    async fn post_rename_failure_requires_admission_and_durability_ack_before_evidence() {
        // Arrange
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "holoscan-build-ack-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        sync_fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap();
        let repository = root.join("repository");
        sync_fs::create_dir(&repository).unwrap();
        let selection = Selection {
            application: "sample".into(),
            ..Default::default()
        };
        let store = BuildStoreRequest {
            root: root.join("builds").to_str().unwrap().into(),
            key: Some(BuildKey {
                repository_root: repository.to_str().unwrap().into(),
                selection: Some(selection.clone()),
            }),
        };
        let begin = |id: &str| BuildAttemptRequest {
            store: Some(store.clone()),
            id: id.into(),
            source: None,
        };
        let success = |image: &str| BuildResult {
            termination: Some(build_result::Termination::ExitCode(0)),
            cleanup_verified: true,
            record: Some(BuildRecord {
                selection: Some(selection.clone()),
                image_id: image.into(),
                source: None,
            }),
            ..Default::default()
        };
        let mut first = Attempt::begin(begin("first")).await.unwrap();
        first.finish(success("sha256:first")).await.unwrap();
        let mut next = Attempt::begin(begin("next")).await.unwrap();
        let failure = FailAck::new();
        // Act: the rename really completed, then its directory acknowledgement failed.
        assert!(next.finish(success("sha256:next")).await.is_err());
        let visible = BuildAttempt::decode(
            sync_fs::read(next.directory.join(LATEST))
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        // Assert: visible terminal bytes are not eligible while the writer retains admission.
        assert_eq!(visible.id, "next");
        assert!(visible.result.unwrap().record.is_some());
        assert!(lookup(store.clone()).await.is_err());
        assert!(Admission::acquire(&repository).is_err());
        drop(failure);
        // Act / Assert: an identical retry acknowledges the visible publication and releases.
        next.finish(success("sha256:next")).await.unwrap();
        assert_eq!(
            lookup(store.clone())
                .await
                .unwrap()
                .record
                .unwrap()
                .image_id,
            "sha256:next"
        );

        // Arrange another completed replacement whose writer disappears before acknowledgement.
        let mut last = Attempt::begin(begin("last")).await.unwrap();
        let failure = FailAck::new();
        assert!(last.finish(success("sha256:last")).await.is_err());
        drop(last);
        // Act / Assert: recovery must itself acknowledge durability; an error returns no evidence.
        let error = lookup(store.clone()).await.unwrap_err().to_string();
        assert!(error.contains("durability acknowledgement"));
        drop(failure);
        let recovered = lookup(store.clone()).await.unwrap();
        assert_eq!(recovered.latest.unwrap().id, "last");
        assert_eq!(recovered.record.unwrap().image_id, "sha256:last");
        let admission = Admission::acquire(&repository).unwrap();
        assert!(
            lookup_admitted(store, &admission)
                .await
                .unwrap()
                .record
                .is_some()
        );
        drop(admission);
        sync_fs::remove_dir_all(root).unwrap();
    }
}
