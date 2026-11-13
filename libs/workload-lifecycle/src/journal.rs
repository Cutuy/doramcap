//! Per-program journal evidence. The index also retains identities for crash recovery.
use orchestrator::Step;
use prost::Message;
use std::{io, path::Path, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

include!(concat!(env!("OUT_DIR"), "/doramcap.journal.v1.rs"));

pub const DIRECTORY: &str = "logs";
pub const INDEX: &str = "index.pb";
/// Only diagnostics are bounded; exported entries have no size or count ceiling.
const DIAGNOSTIC_BYTES: u64 = 8192;
const MICROS_PER_SECOND: u64 = 1_000_000;

/// Read the authoritative index. An absent index is an older run without saved identities.
/// Decode and filesystem failures are returned; callers must not infer an empty successful export.
pub async fn load(artifacts: &Path) -> io::Result<LogIndex> {
    let mut file = tokio::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(artifacts.join(DIRECTORY).join(INDEX))
        .await?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).await?;
    LogIndex::decode(bytes.as_slice()).map_err(io::Error::other)
}

/// Save newly observed identities without changing terminal results or numbered file positions.
/// Missing indexes are created; unreadable indexes are preserved and reported as errors.
pub async fn remember(
    artifacts: &Path,
    programs: impl IntoIterator<Item = ProgramLog>,
) -> io::Result<()> {
    let mut index = match load(artifacts).await {
        Ok(index) => index,
        Err(error) if error.kind() == io::ErrorKind::NotFound => LogIndex::default(),
        Err(error) => return Err(error),
    };
    let mut changed = false;
    for program in programs {
        if let Some(existing) = index.programs.iter_mut().find(|existing| {
            existing.program_id == program.program_id
                && (existing.match_kind == JournalMatchKind::SystemdInvocation as i32
                    || existing.identity == program.identity)
        }) {
            if existing.state == LogState::Pending as i32 && existing.identity.is_empty() {
                *existing = program;
                changed = true;
            }
        } else {
            index.programs.push(program);
            changed = true;
        }
    }
    if changed {
        save(artifacts, &index).await
    } else {
        Ok(())
    }
}

/// Export a previously saved run through its executor. Failures remain independent of cleanup.
/// Missing indexes are accepted for runs created before journal export existed.
pub async fn finish(executor: &orchestrator::LocalExecutor, artifacts: &Path) -> io::Result<()> {
    let index = match load(artifacts).await {
        Ok(index) => index,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    executor
        .run::<ExportJournal>(ExportJournalInput {
            artifacts: artifacts
                .to_str()
                .ok_or_else(|| io::Error::other("run path is not UTF-8"))?
                .into(),
            index: Some(index),
        })
        .await
        .map_err(io::Error::other)?;
    Ok(())
}

/// Atomically replace the private index and sync its file and directory.
/// Call under the workflow's repository admission; failures leave the prior index readable.
pub async fn save(artifacts: &Path, index: &LogIndex) -> io::Result<()> {
    let directory = artifacts.join(DIRECTORY);
    match tokio::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .await
    {
        Ok(()) => (),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(error),
    }
    if !tokio::fs::symlink_metadata(&directory).await?.is_dir() {
        return Err(io::Error::other("log directory is not a directory"));
    }
    let temporary = directory.join("index.pb.tmp");
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&temporary)
        .await?;
    file.write_all(&index.encode_to_vec()).await?;
    file.flush().await?;
    file.sync_all().await?;
    tokio::fs::rename(temporary, directory.join(INDEX)).await?;
    tokio::fs::File::open(directory).await?.sync_all().await
}

/// Export pending programs after all writers stop. Terminal entries and existing files stay.
/// Per-program failures enter the index; index persistence failures fail the step.
/// Cancellation leaves pending entries and partial files for the next cleanup to preserve.
pub struct ExportJournal;
impl Step for ExportJournal {
    const DISPLAY_NAME: Option<&'static str> = Some("Export journal logs");
    type Input = ExportJournalInput;
    type Output = LogIndex;

    async fn run(&mut self, input: ExportJournalInput) -> orchestrator::Result<LogIndex> {
        export(input)
            .await
            .map_err(|error| orchestrator::Error::message(error.to_string()))
    }
}

async fn export(input: ExportJournalInput) -> io::Result<LogIndex> {
    let artifacts = Path::new(&input.artifacts);
    let mut index = input
        .index
        .ok_or_else(|| io::Error::other("journal index missing"))?;
    save(artifacts, &index).await?;
    let sync = if index
        .programs
        .iter()
        .any(|program| program.state == LogState::Pending as i32)
    {
        match Command::new("journalctl").arg("--sync").output().await {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => Err(format!(
                "journal sync {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )),
            Err(error) => Err(format!("journal sync: {error}")),
        }
    } else {
        Ok(())
    };
    for number in 0..index.programs.len() {
        let program = &mut index.programs[number];
        if program.state != LogState::Pending as i32 {
            continue;
        }
        let path = artifacts.join(DIRECTORY).join(format!("{number}.log"));
        let result = match &sync {
            Ok(()) => export_program(program, &path).await,
            Err(error) => Err(io::Error::other(error.clone())),
        };
        match result {
            Ok(()) => program.state = LogState::Exported.into(),
            Err(error) => {
                program.state = LogState::Failed.into();
                program.reason = error.to_string();
            }
        }
        program.size_bytes = tokio::fs::metadata(&path)
            .await
            .map_or(0, |metadata| metadata.len());
        save(artifacts, &index).await?;
    }
    Ok(index)
}

async fn export_program(program: &mut ProgramLog, path: &Path) -> io::Result<()> {
    let length = match JournalMatchKind::try_from(program.match_kind) {
        Ok(JournalMatchKind::SystemdInvocation) => 32,
        Ok(JournalMatchKind::Container) => 64,
        _ => return Err(io::Error::other("journal match kind unavailable")),
    };
    if program.identity.len() != length
        || !program
            .identity
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(io::Error::other("journal identity unavailable or invalid"));
    }
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .await
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                io::Error::other("interrupted export; existing file preserved")
            } else {
                error
            }
        })?;
    let mut command = Command::new("journalctl");
    command.args([
        "--no-pager",
        "--quiet",
        "--all",
        "--output=json",
        "--output-fields=__REALTIME_TIMESTAMP,MESSAGE",
    ]);
    match JournalMatchKind::try_from(program.match_kind) {
        Ok(JournalMatchKind::SystemdInvocation) => {
            command
                .arg(format!("_SYSTEMD_INVOCATION_ID={}", program.identity))
                .arg("+")
                .arg(format!("INVOCATION_ID={}", program.identity));
        }
        _ => {
            command.arg(format!("CONTAINER_ID_FULL={}", program.identity));
        }
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("journal stdout missing"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("journal stderr missing"))?;
    let read = async {
        let mut lines = BufReader::new(stdout).lines();
        while let Some(line) = lines.next_line().await? {
            let entry: serde_json::Value = serde_json::from_str(&line).map_err(io::Error::other)?;
            let micros = entry["__REALTIME_TIMESTAMP"]
                .as_str()
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or_else(|| io::Error::other("journal timestamp missing"))?;
            let message = match &entry["MESSAGE"] {
                serde_json::Value::String(message) => message.clone(),
                serde_json::Value::Array(bytes) => {
                    let bytes = bytes
                        .iter()
                        .map(|byte| {
                            byte.as_u64()
                                .and_then(|byte| u8::try_from(byte).ok())
                                .ok_or_else(|| io::Error::other("invalid binary journal message"))
                        })
                        .collect::<io::Result<Vec<_>>>()?;
                    String::from_utf8_lossy(&bytes).into_owned()
                }
                _ => return Err(io::Error::other("journal message missing")),
            };
            // Escape embedded line breaks so entry counts correspond to text lines.
            let message = message
                .replace('\\', "\\\\")
                .replace('\r', "\\r")
                .replace('\n', "\\n");
            let line = format!(
                "{}.{:06} {message}\n",
                micros / MICROS_PER_SECOND,
                micros % MICROS_PER_SECOND
            );
            file.write_all(line.as_bytes()).await?;
            program.entry_count += 1;
        }
        file.flush().await?;
        file.sync_all().await
    };
    let diagnostic = async {
        let mut stderr = stderr;
        let mut bytes = Vec::new();
        (&mut stderr)
            .take(DIAGNOSTIC_BYTES)
            .read_to_end(&mut bytes)
            .await?;
        tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await?;
        Ok::<_, io::Error>(String::from_utf8_lossy(&bytes).into_owned())
    };
    // Drain both pipes and reap the child even when reading or writing an entry fails.
    let (read, diagnostic, status) = tokio::join!(read, diagnostic, child.wait());
    read?;
    let diagnostic = diagnostic?;
    let status = status?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "journalctl {status}: {diagnostic}"
        )));
    }
    Ok(())
}
