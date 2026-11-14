//! Per-program journal evidence. The index also retains identities for crash recovery.
use orchestrator::Step;
use prost::Message;
use std::{io, path::Path, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    process::Command,
};

include!(concat!(env!("OUT_DIR"), "/doramcap.journal.v1.rs"));

pub const DIRECTORY: &str = "logs";
pub const INDEX: &str = "index.pb";
/// Only diagnostics are bounded; exported entries have no size or count ceiling.
const DIAGNOSTIC_BYTES: u64 = 8192;
const MICROS_PER_SECOND: u64 = 1_000_000;
const READ_BUFFER_BYTES: usize = 8192;
const JOURNAL_SOCKET: &str = "/run/systemd/journal/io.systemd.journal";
const SYNCHRONIZE_REQUEST: &[u8] =
    b"{\"method\":\"io.systemd.Journal.Synchronize\",\"parameters\":{}}\0";

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SynchronizeReply {
    #[serde(rename = "parameters")]
    _parameters: Option<serde_json::Map<String, serde_json::Value>>,
    error: Option<String>,
    #[serde(default)]
    continues: bool,
}

/// Wait for journald to finish synchronization, as journalctl --sync does.
/// Socket, protocol and daemon errors fail the export; cancellation closes the connection.
/// There is no deadline, matching journalctl's synchronization call.
async fn synchronize(socket: &Path) -> io::Result<()> {
    let mut stream = UnixStream::connect(socket).await?;
    stream.write_all(SYNCHRONIZE_REQUEST).await?;
    let mut reply = Vec::new();
    BufReader::new(stream).read_until(0, &mut reply).await?;
    if reply.pop() != Some(0) {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "journal synchronization reply incomplete",
        ));
    }
    let reply: SynchronizeReply = serde_json::from_slice(&reply)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if reply.continues {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unexpected streaming journal synchronization reply",
        ));
    }
    if let Some(error) = reply.error {
        return Err(io::Error::other(error));
    }
    Ok(())
}

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
        export(input, Path::new(JOURNAL_SOCKET))
            .await
            .map_err(|error| orchestrator::Error::message(error.to_string()))
    }
}

async fn export(input: ExportJournalInput, journal_socket: &Path) -> io::Result<LogIndex> {
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
        synchronize(journal_socket)
            .await
            .map_err(|error| format!("journal sync: {error}"))
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
                // Count only complete entries that survived a partial write or prior crash.
                if let Ok(mut file) = tokio::fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(&path)
                    .await
                {
                    let mut bytes = [0u8; READ_BUFFER_BYTES];
                    program.entry_count = 0;
                    loop {
                        let read = match file.read(&mut bytes).await {
                            Ok(read) => read,
                            Err(error) => {
                                program
                                    .reason
                                    .push_str(&format!("; recount partial entries: {error}"));
                                break;
                            }
                        };
                        if read == 0 {
                            break;
                        }
                        program.entry_count +=
                            bytes[..read].iter().filter(|byte| **byte == b'\n').count() as u64;
                    }
                }
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
            for (position, field) in [
                "_SYSTEMD_INVOCATION_ID",
                "OBJECT_SYSTEMD_INVOCATION_ID",
                "INVOCATION_ID",
                "USER_INVOCATION_ID",
            ]
            .into_iter()
            .enumerate()
            {
                if position > 0 {
                    command.arg("+");
                }
                command.arg(format!("{field}={}", program.identity));
            }
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
        Ok::<_, io::Error>(())
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
    // Flush accepted writes even if parsing or reading a later entry failed.
    let persisted = async {
        file.flush().await?;
        file.sync_all().await
    }
    .await;
    read?;
    persisted?;
    let diagnostic = diagnostic?;
    let status = status?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "journalctl {status}: {diagnostic}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    use tokio::{net::UnixListener, sync::oneshot};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "journal-sync-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn socket(&self) -> PathBuf {
            self.0.join("journal.socket")
        }
        fn input(&self, states: &[LogState]) -> ExportJournalInput {
            ExportJournalInput {
                artifacts: self.0.to_str().unwrap().into(),
                index: Some(LogIndex {
                    programs: states
                        .iter()
                        .enumerate()
                        .map(|(number, state)| ProgramLog {
                            program_id: number.to_string(),
                            state: (*state).into(),
                            ..Default::default()
                        })
                        .collect(),
                }),
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    async fn request(listener: UnixListener) -> BufReader<UnixStream> {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut request = Vec::new();
        stream.read_until(0, &mut request).await.unwrap();
        assert_eq!(request.pop(), Some(0));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&request).unwrap(),
            serde_json::json!({"method": "io.systemd.Journal.Synchronize", "parameters": {}})
        );
        stream
    }

    #[tokio::test]
    async fn synchronization_waits_for_the_complete_reply() {
        // Arrange
        let fixture = Fixture::new();
        let socket = fixture.socket();
        let listener = UnixListener::bind(&socket).unwrap();
        let (partial, received) = oneshot::channel();
        let (finish, allowed) = oneshot::channel();
        let server = tokio::spawn(async move {
            let mut stream = request(listener).await;
            stream
                .get_mut()
                .write_all(b"{\"parameters\":{}")
                .await
                .unwrap();
            partial.send(()).unwrap();
            allowed.await.unwrap();
            stream.get_mut().write_all(b"}\0").await.unwrap();
        });
        // Act: JSON and its NUL terminator can arrive in separate socket reads.
        let client = tokio::spawn(async move { synchronize(&socket).await });
        received.await.unwrap();
        // Assert: sending the request is not completion.
        assert!(!client.is_finished());
        finish.send(()).unwrap();
        client.await.unwrap().unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn synchronization_reports_daemon_protocol_and_disconnect_errors() {
        for (reply, reason) in [
            (
                b"{\"error\":\"org.varlink.service.PermissionDenied\",\"parameters\":{}}\0"
                    .as_slice(),
                "PermissionDenied",
            ),
            (b"{\"parameters\":{},\"continues\":true}\0", "streaming"),
            (b"{\"parameters\":[]}\0", "invalid type"),
            (b"{\"unexpected\":true}\0", "unknown field"),
            (b"not JSON\0", "expected"),
            (b"{\"parameters\":{}}", "incomplete"),
            (b"", "incomplete"),
        ] {
            // Arrange
            let fixture = Fixture::new();
            let listener = UnixListener::bind(fixture.socket()).unwrap();
            let server = tokio::spawn(async move {
                let mut stream = request(listener).await;
                stream.get_mut().write_all(reply).await.unwrap();
            });
            // Act / Assert
            let error = synchronize(&fixture.socket()).await.unwrap_err();
            assert!(error.to_string().contains(reason), "{error}");
            server.await.unwrap();
        }
        // Arrange / Act / Assert: no daemon socket is an explicit failure.
        let fixture = Fixture::new();
        assert_eq!(
            synchronize(&fixture.socket()).await.unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[tokio::test]
    async fn cancellation_closes_the_sync_connection() {
        // Arrange
        let fixture = Fixture::new();
        let socket = fixture.socket();
        let listener = UnixListener::bind(&socket).unwrap();
        let client = tokio::spawn(async move { synchronize(&socket).await });
        let mut stream = request(listener).await;
        // Act
        client.abort();
        assert!(client.await.unwrap_err().is_cancelled());
        // Assert
        assert_eq!(stream.read(&mut [0]).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn sync_failure_persists_failed_programs_and_preserves_existing_evidence() {
        // Arrange
        let fixture = Fixture::new();
        let input = fixture.input(&[LogState::Pending, LogState::Skipped, LogState::Exported]);
        save(&fixture.0, input.index.as_ref().unwrap())
            .await
            .unwrap();
        let partial = fixture.0.join(DIRECTORY).join("0.log");
        tokio::fs::write(&partial, b"saved entry\n").await.unwrap();
        let listener = UnixListener::bind(fixture.socket()).unwrap();
        let server = tokio::spawn(async move {
            request(listener)
                .await
                .get_mut()
                .write_all(b"{\"error\":\"org.varlink.service.PermissionDenied\"}\0")
                .await
                .unwrap();
        });
        // Act
        let result = export(input, &fixture.socket()).await.unwrap();
        // Assert
        assert_eq!(result.programs[0].state, LogState::Failed as i32);
        assert!(
            result.programs[0]
                .reason
                .contains("journal sync: org.varlink.service.PermissionDenied")
        );
        assert_eq!(result.programs[0].entry_count, 1);
        assert_eq!(result.programs[0].size_bytes, 12);
        assert_eq!(result.programs[1].state, LogState::Skipped as i32);
        assert_eq!(result.programs[2].state, LogState::Exported as i32);
        assert_eq!(load(&fixture.0).await.unwrap(), result);
        assert_eq!(tokio::fs::read(partial).await.unwrap(), b"saved entry\n");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn terminal_programs_do_not_require_a_sync_socket() {
        // Arrange
        let fixture = Fixture::new();
        let input = fixture.input(&[LogState::Skipped, LogState::Exported, LogState::Failed]);
        let expected = input.index.clone().unwrap();
        // Act / Assert
        assert_eq!(export(input, &fixture.socket()).await.unwrap(), expected);
    }

    #[tokio::test]
    #[ignore = "requires root and a running host journald"]
    async fn native_sync_makes_queued_entries_readable() {
        // Arrange: submit through journald's native datagram protocol.
        let fixture = Fixture::new();
        let marker = fixture.0.file_name().unwrap().to_str().unwrap();
        let message = format!("SYSLOG_IDENTIFIER={marker}\nMESSAGE={marker}\n");
        std::os::unix::net::UnixDatagram::unbound()
            .unwrap()
            .send_to(message.as_bytes(), "/run/systemd/journal/socket")
            .unwrap();
        // Act: only the Rust socket path synchronizes the journal.
        synchronize(Path::new(JOURNAL_SOCKET)).await.unwrap();
        // Assert: journalctl is an independent reader after synchronization returns.
        let output = Command::new("journalctl")
            .args([
                "--no-pager",
                "--quiet",
                "--output=cat",
                &format!("SYSLOG_IDENTIFIER={marker}"),
            ])
            .output()
            .await
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), marker);
    }
}
