//! Bounded observational sampling; filesystem I/O runs outside execution's owner.
use crate::{
    policy::*,
    protocol::{Phase, RunMetric, Status},
    session::Session,
};
use prost::Message;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};
use telemetry_protocol::{
    ALL_CPUS, FLAG_INCLUSIVE, FLAG_READ_ERROR, FLAG_ROOT, FLAG_RUNTIME, FLAG_VIRTUAL_KERNEL,
};

type Counters = BTreeMap<(u64, u32), u64>;

#[derive(Default)]
struct Reader {
    offset: u64,
    identity: Option<(u64, u64)>,
    previous: Option<(String, u64, Counters)>,
    size: Option<(u64, Instant)>,
    incomplete: bool,
    latest: Vec<telemetry_protocol::pb::Snapshot>,
    names: Vec<crate::protocol::CpuGroupName>,
}
impl Reader {
    fn sample(&mut self, directory: &Path, elapsed: Duration) -> RunMetric {
        let mut sample = RunMetric {
            elapsed_ns: elapsed.as_nanos().min(u64::MAX as u128) as u64,
            ..Default::default()
        };
        let Ok(root) = workload_docker::evidence::root(directory) else {
            return sample;
        };
        if let Ok(file) = workload_docker::evidence::open(&root, "cpu.mcap", false)
            && let Ok(metadata) = file.metadata()
        {
            let now = Instant::now();
            let bytes = metadata.len();
            sample.recording_bytes = Some(bytes);
            if let Some((before, time)) = self.size {
                if bytes >= before {
                    sample.write_bytes_per_second = Some(
                        (bytes - before) as f64
                            / now
                                .duration_since(time)
                                .as_secs_f64()
                                .max(MIN_RATE_INTERVAL.as_secs_f64()),
                    );
                } else {
                    self.incomplete = true;
                }
            }
            self.size = Some((bytes, now));
        }
        if let Err(error) = self.cpu(&root, &mut sample)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            self.incomplete = true;
            self.previous = None;
        }
        sample
    }

    fn cpu(&mut self, root: &File, sample: &mut RunMetric) -> std::io::Result<()> {
        let mut file = workload_docker::evidence::open(root, "cpu.pb", false)?;
        let metadata = file.metadata()?;
        let identity = (metadata.dev(), metadata.ino());
        if self.identity.is_some_and(|old| old != identity) || metadata.len() < self.offset {
            self.offset = 0;
            self.previous = None;
            self.latest.clear();
            self.names.clear();
            self.incomplete = true;
        }
        self.identity = Some(identity);
        let mut frames = Vec::new();
        let mut scanned = 0;
        for _ in 0..MAX_CPU_HEADERS_PER_TICK {
            if self.offset + 4 > metadata.len() {
                break;
            }
            file.seek(SeekFrom::Start(self.offset))?;
            let mut header = [0; 4];
            file.read_exact(&mut header)?;
            let length = u32::from_le_bytes(header) as u64;
            if length == 0 || length > MAX_CPU_FRAME_BYTES {
                return Err(std::io::Error::other("invalid CPU frame length"));
            }
            if self.offset + 4 + length > metadata.len() {
                break;
            }
            // Scan headers only; decode the last two complete frames per tick.
            frames.push((self.offset + 4, length));
            if frames.len() > MAX_CPU_SNAPSHOTS {
                frames.remove(0);
            }
            self.offset += 4 + length;
            scanned += 1;
        }
        if scanned == MAX_CPU_HEADERS_PER_TICK && self.offset + 4 <= metadata.len() {
            self.incomplete = true;
        }
        for (offset, length) in frames {
            file.seek(SeekFrom::Start(offset))?;
            let mut bytes = vec![0; length as usize];
            file.read_exact(&mut bytes)?;
            let frame = telemetry_protocol::decode(&bytes).map_err(std::io::Error::other)?;
            if self
                .latest
                .last()
                .is_none_or(|last| last.sequence != frame.snapshot.sequence)
                && let Some(telemetry_protocol::pb::packet::Body::Snapshot(packet)) =
                    telemetry_protocol::snapshot_packet(&frame).body
                && let Some(snapshot) = packet.snapshot
            {
                if self.latest.len() == MAX_CPU_SNAPSHOTS {
                    self.latest.remove(0);
                }
                self.latest.push(snapshot);
            }
            self.incomplete |= frame.dictionary_overflow != 0;
            let parents: BTreeSet<_> = frame
                .snapshot
                .records
                .iter()
                .map(|row| row.parent_id)
                .collect();
            let leaves: BTreeSet<_> = frame
                .snapshot
                .records
                .iter()
                .map(|row| row.cgroup_id)
                .filter(|id| !parents.contains(id))
                .collect();
            self.names = frame
                .dictionary
                .iter()
                .filter(|entry| leaves.contains(&entry.id))
                .filter_map(|entry| {
                    Some(crate::protocol::CpuGroupName {
                        id: entry.id,
                        name: public_cgroup_name(entry.path.as_deref()?)?,
                    })
                })
                .take(MAX_CPU_GROUP_NAMES + 1)
                .collect();
            if self.names.len() > MAX_CPU_GROUP_NAMES {
                self.names.truncate(MAX_CPU_GROUP_NAMES);
                self.incomplete = true;
            }
            let snapshot = &frame.snapshot;
            sample.cpu_cores = snapshot
                .records
                .iter()
                .map(|r| r.cpu)
                .filter(|cpu| *cpu != ALL_CPUS)
                .collect::<BTreeSet<_>>()
                .len() as u32;
            // Inclusive ancestor rows cover their descendants. Root-unavailable,
            // read-error and virtual kernel rows cannot enter the total.
            let records: BTreeMap<_, _> = snapshot
                .records
                .iter()
                .filter(|r| {
                    r.cpu != ALL_CPUS
                        && r.flags & FLAG_RUNTIME != 0
                        && r.flags & (FLAG_ROOT | FLAG_READ_ERROR | FLAG_VIRTUAL_KERNEL) == 0
                })
                .map(|r| ((r.cgroup_id, r.cpu), r))
                .collect();
            let parents: BTreeMap<_, _> = snapshot
                .records
                .iter()
                .map(|r| (r.cgroup_id, r.parent_id))
                .collect();
            let mut counters = BTreeMap::new();
            let mut valid = snapshot.capacity_overflow == 0
                && snapshot.dropped_batches == 0
                && !snapshot
                    .records
                    .iter()
                    .any(|r| r.flags & FLAG_READ_ERROR != 0 && r.flags & FLAG_ROOT == 0);
            for (&key, row) in &records {
                let mut parent = row.parent_id;
                let mut covered = false;
                let mut seen = BTreeSet::new();
                while parent != 0 && parent != row.cgroup_id {
                    if !seen.insert(parent) || seen.len() > MAX_CGROUP_ANCESTORS {
                        valid = false;
                        break;
                    }
                    if records
                        .get(&(parent, row.cpu))
                        .is_some_and(|ancestor| ancestor.flags & FLAG_INCLUSIVE != 0)
                    {
                        covered = true;
                        break;
                    }
                    let Some(next) = parents.get(&parent) else {
                        break;
                    };
                    if *next == parent {
                        break;
                    }
                    parent = *next;
                }
                if !covered {
                    counters.insert(key, row.runtime_ns);
                }
            }
            sample.cpu_percent = None;
            if valid && !counters.is_empty() {
                if let Some((session, time, previous)) = &self.previous
                    && session == &frame.session_id
                    && snapshot.monotonic_end_ns > *time
                    && previous.len() == counters.len()
                {
                    let delta = counters.iter().try_fold(0_u128, |sum, (key, value)| {
                        Some(sum + value.checked_sub(*previous.get(key)?)? as u128)
                    });
                    sample.cpu_percent =
                        delta.map(|d| d as f64 / (snapshot.monotonic_end_ns - time) as f64 * 100.0);
                }
                self.previous = Some((frame.session_id, snapshot.monotonic_end_ns, counters));
            } else {
                self.incomplete = true;
                self.previous = None;
            }
        }
        Ok(())
    }
}

fn public_cgroup_name(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let leaf = path
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or("/");
    let name: String = leaf
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_CPU_GROUP_NAME_CHARS)
        .collect();
    (!name.is_empty() && name != "." && name != "..").then_some(name)
}

fn terminal(phase: Phase) -> bool {
    matches!(phase, Phase::Finished | Phase::Blocked | Phase::Idle)
}

/// Sample and publish at the configured interval within the compile-time retention budgets.
pub(crate) fn watch(
    session: Rc<RefCell<Session>>,
    directory: PathBuf,
    enabled: bool,
    interval: Duration,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let (stop, mut stopped) = tokio::sync::oneshot::channel();
    let task = tokio::task::spawn_local(async move {
        let started = Instant::now();
        let run_id = session.borrow().marker.as_ref().map(|m| m.run_id.clone());
        let mut reader = Reader::default();
        let mut finishing = false;
        loop {
            let tick = Instant::now();
            let sample = if enabled {
                let path = directory.clone();
                let elapsed = started.elapsed();
                let Ok((next, sample)) = tokio::task::spawn_blocking(move || {
                    let sample = reader.sample(&path, elapsed);
                    (reader, sample)
                })
                .await
                else {
                    break;
                };
                reader = next;
                Some(sample)
            } else {
                None
            };
            {
                let mut current = session.borrow_mut();
                if current.marker.as_ref().map(|m| &m.run_id) != run_id.as_ref() {
                    break;
                }
                if let Some(sample) = sample {
                    if current.metrics.len() == MAX_METRIC_SAMPLES {
                        current.metrics.remove(0);
                    }
                    current.metrics.push(sample);
                    current.cpu_snapshots = reader.latest.clone();
                    current.cpu_group_names = reader.names.clone();
                    if current
                        .cpu_snapshots
                        .iter()
                        .map(Message::encoded_len)
                        .sum::<usize>()
                        > MAX_CPU_SNAPSHOT_BYTES
                    {
                        current.cpu_snapshots.clear();
                        current.metrics_incomplete = true;
                    }
                    current.metrics_incomplete |= reader.incomplete;
                }
                current.publish();
            }
            if finishing {
                break;
            }
            tokio::select! {
                _ = &mut stopped => finishing = true,
                _ = tokio::time::sleep(interval.saturating_sub(tick.elapsed())) => {}
            }
        }
    });
    (stop, task)
}

/// Checkpoint the redacted terminal view before shutdown is acknowledged.
pub(crate) async fn checkpoint(directory: PathBuf, status: Status) -> bool {
    tokio::task::spawn_blocking(move || save(&directory, &status))
        .await
        .is_ok_and(|result| result.is_ok())
}

fn save(directory: &Path, status: &Status) -> std::io::Result<()> {
    let bytes = status.encode_to_vec();
    if bytes.len() as u64 > MAX_ACTIVITY_BYTES {
        return Err(std::io::Error::other("activity too large"));
    }
    let root = workload_docker::evidence::root(directory)?;
    let fd = rustix::fs::openat(
        &root,
        "activity.pb",
        rustix::fs::OFlags::WRONLY
            | rustix::fs::OFlags::CREATE
            | rustix::fs::OFlags::EXCL
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )?;
    let mut file = File::from(fd);
    file.write_all(&bytes)?;
    file.sync_all()?;
    root.sync_all()
}

/// Restore only matching terminal activity. Missing snapshots remain unavailable.
pub(crate) fn restore(session: &mut Session, data_root: &Path) {
    let Some(marker) = &session.marker else {
        return;
    };
    if !terminal(session.phase)
        || marker.run_id.len() != RUN_ID_HEX_CHARS
        || !marker.run_id.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return;
    }
    let result = (|| -> std::io::Result<Status> {
        let root = workload_docker::evidence::root(&data_root.join("runs"))?;
        let file = workload_docker::evidence::open(
            &root,
            &format!("{}/activity.pb", marker.run_id),
            false,
        )?;
        let mut bytes = Vec::new();
        file.take(MAX_ACTIVITY_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_ACTIVITY_BYTES {
            return Err(std::io::Error::other("activity too large"));
        }
        Status::decode(bytes.as_slice()).map_err(std::io::Error::other)
    })();
    if let Ok(status) = result
        && status.run_id == marker.run_id
        && status.phase == session.phase as i32
        && status.steps.len() <= MAX_PROGRESS_ROWS
        && status.metrics.len() <= MAX_METRIC_SAMPLES
        && status.cpu_snapshots.len() <= MAX_CPU_SNAPSHOTS
        && status.cpu_group_names.len() <= MAX_CPU_GROUP_NAMES
    {
        session.progress.restore(&status);
        session.metrics = status.metrics;
        session.cpu_snapshots = status.cpu_snapshots;
        session.cpu_group_names = status.cpu_group_names;
        // Older checkpoints omitted names; read the matching final producer dictionary.
        if session.cpu_group_names.is_empty() && !session.cpu_snapshots.is_empty() {
            let mut reader = Reader::default();
            if let Ok(root) =
                workload_docker::evidence::root(&data_root.join("runs").join(&marker.run_id))
                && reader.cpu(&root, &mut RunMetric::default()).is_ok()
                && reader.latest.last().map(|s| s.sequence)
                    == session.cpu_snapshots.last().map(|s| s.sequence)
            {
                session.metrics_incomplete |= reader.incomplete;
                session.cpu_group_names = reader.names;
            }
        }
        session.metrics_incomplete |= status.metrics_incomplete;
        session
            .loss
            .store(status.lost_events, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Marker;
    use telemetry_protocol::pb;

    #[tokio::test(flavor = "current_thread")]
    async fn project_only_runs_publish_progress_without_sampling_cpu_files() {
        // Arrange
        tokio::task::LocalSet::new()
            .run_until(async {
                let session = Rc::new(RefCell::new(Session::new(
                    Phase::Running,
                    Some(Marker {
                        run_id: "run".into(),
                        ..Default::default()
                    }),
                )));
                let (stop, done) = watch(
                    session.clone(),
                    PathBuf::from("/does-not-exist"),
                    false,
                    Duration::from_millis(1),
                );
                session
                    .borrow_mut()
                    .progress
                    .apply(workflow_protocol::Event {
                        version: workflow_protocol::VERSION,
                        sequence: 1,
                        invocation: 1,
                        state: workflow_protocol::event::State::Succeeded as i32,
                        ..Default::default()
                    });
                // Act
                tokio::time::timeout(Duration::from_secs(1), async {
                    loop {
                        if session.borrow().snapshot.lock().unwrap().sequence == 1 {
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .unwrap();
                let _ = stop.send(());
                done.await.unwrap();
                // Assert
                let current = session.borrow();
                let snapshot = current.snapshot.lock().unwrap();
                assert_eq!(snapshot.steps.len(), 1);
                assert!(snapshot.metrics.is_empty());
                assert!(snapshot.cpu_snapshots.is_empty());
            })
            .await;
    }

    #[test]
    fn group_name_limit_reports_partial_observation_only_on_overflow() {
        for count in [MAX_CPU_GROUP_NAMES, MAX_CPU_GROUP_NAMES + 1] {
            // Arrange
            let root = tempfile::tempdir().unwrap();
            let ids = 1..=count as u64;
            let packet = pb::Packet {
                body: Some(pb::packet::Body::Snapshot(pb::SnapshotFrame {
                    version: telemetry_protocol::VERSION,
                    session_id: "capture".into(),
                    dictionary_full: true,
                    dictionary: ids
                        .clone()
                        .map(|id| pb::DictionaryEntry {
                            id,
                            parent_id: 0,
                            path: Some(format!("/private/worker-{id}")),
                        })
                        .collect(),
                    snapshot: Some(pb::Snapshot {
                        schema_version: 1,
                        records: ids
                            .map(|id| pb::CpuRecord {
                                cgroup_id: id,
                                flags: FLAG_RUNTIME,
                                ..Default::default()
                            })
                            .collect(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })),
            };
            append(&root.path().join("cpu.pb"), &packet.encode_to_vec());
            let mut reader = Reader::default();
            // Act
            reader.sample(root.path(), Duration::ZERO);
            // Assert
            assert_eq!(reader.names.len(), MAX_CPU_GROUP_NAMES);
            assert_eq!(reader.incomplete, count > MAX_CPU_GROUP_NAMES);
            assert!(reader.names.iter().all(|entry| !entry.name.contains('/')));
        }
    }

    fn frame(sequence: u64, runtime: u64) -> Vec<u8> {
        let mut records = Vec::new();
        for cpu in 0..2 {
            for (id, parent, flags, counter) in [
                (1, 0, 20, 0),
                (10, 1, 5, runtime),
                (11, 10, 5, runtime / 2),
                (0, 0, 129, runtime),
            ] {
                records.push(pb::CpuRecord {
                    cgroup_id: id,
                    parent_id: parent,
                    cpu,
                    flags,
                    runtime_ns: counter,
                    ..Default::default()
                });
            }
        }
        pb::Packet {
            body: Some(pb::packet::Body::Snapshot(pb::SnapshotFrame {
                version: 2,
                session_id: "capture".into(),
                dictionary_full: true,
                dictionary: vec![pb::DictionaryEntry {
                    id: 11,
                    parent_id: 10,
                    path: Some("/private/ancestor/worker.service".into()),
                }],
                snapshot: Some(pb::Snapshot {
                    schema_version: 1,
                    sequence,
                    monotonic_end_ns: (sequence + 1) * 100_000_000,
                    records,
                    ..Default::default()
                }),
                ..Default::default()
            })),
        }
        .encode_to_vec()
    }
    fn append(path: &Path, bytes: &[u8]) {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        file.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
        file.write_all(bytes).unwrap();
    }
    #[test]
    fn names_are_bounded_leaf_names_and_legacy_checkpoints_can_restore_them() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let id = "a".repeat(32);
        let directory = root.path().join("runs").join(&id);
        std::fs::create_dir_all(&directory).unwrap();
        let stream = directory.join("cpu.pb");
        append(&stream, &frame(0, 100));
        append(&stream, &frame(1, 200));
        let mut reader = Reader::default();
        reader.sample(&directory, Duration::ZERO);
        let status = Status {
            phase: Phase::Finished as i32,
            run_id: id.clone(),
            cpu_snapshots: reader.latest,
            ..Default::default()
        };
        save(&directory, &status).unwrap();
        let mut session = Session::new(
            Phase::Finished,
            Some(Marker {
                run_id: id,
                ..Default::default()
            }),
        );
        // Act
        restore(&mut session, root.path());
        // Assert
        assert_eq!(session.cpu_group_names[0].name, "worker.service");
        assert_eq!(
            public_cgroup_name("/private/parent/worker.service\n").as_deref(),
            Some("worker.service")
        );
        assert_eq!(public_cgroup_name("/").as_deref(), Some("/"));
        assert!(public_cgroup_name("").is_none());
        assert_eq!(
            public_cgroup_name(&"é".repeat(200))
                .unwrap()
                .chars()
                .count(),
            128
        );
        let encoded = Status {
            cpu_group_names: session.cpu_group_names,
            ..Default::default()
        }
        .encode_to_vec();
        assert!(
            !encoded
                .windows(b"private".len())
                .any(|bytes| bytes == b"private")
        );
    }

    #[test]
    fn totals_exclude_inclusive_descendants_and_virtual_kernel_preserving_u64_deltas() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let stream = root.path().join("cpu.pb");
        let counter = (1_u64 << 53) + 3;
        append(&stream, &frame(0, counter));
        append(&stream, &frame(1, counter + 10_000_000));
        std::fs::write(root.path().join("cpu.mcap"), [0; 100]).unwrap();
        let mut reader = Reader::default();
        // Act
        let first = reader.sample(root.path(), Duration::from_secs(1));
        // Assert
        assert_eq!(first.cpu_percent, Some(20.0));
        assert_eq!(first.cpu_cores, 2);
        assert_eq!(reader.names[0].id, 11);
        assert_eq!(reader.names[0].name, "worker.service");
        assert_eq!(first.recording_bytes, Some(100));
        assert!(first.write_bytes_per_second.is_none());
        // Act: a partial writer frame must wait without corrupting the cursor.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&stream)
            .unwrap();
        let next = frame(2, counter + 20_000_000);
        file.write_all(&(next.len() as u32).to_le_bytes()).unwrap();
        file.write_all(&next[..10]).unwrap();
        let partial = reader.sample(root.path(), Duration::from_secs(2));
        // Assert
        assert!(partial.cpu_percent.is_none());
        assert!(!reader.incomplete);
        // Act
        file.write_all(&next[10..]).unwrap();
        std::fs::write(root.path().join("cpu.mcap"), [0; 200]).unwrap();
        let complete = reader.sample(root.path(), Duration::from_secs(3));
        // Assert
        assert_eq!(complete.cpu_percent, Some(20.0));
        assert_eq!(complete.recording_bytes, Some(200));
        assert!(complete.write_bytes_per_second.unwrap() > 0.0);
    }
    #[test]
    fn resets_invalid_frames_and_symlinks_never_produce_false_zero_cpu() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let stream = root.path().join("cpu.pb");
        append(&stream, &frame(0, 100));
        append(&stream, &frame(1, 1));
        let mut reader = Reader::default();
        // Act / Assert
        assert!(
            reader
                .sample(root.path(), Duration::ZERO)
                .cpu_percent
                .is_none()
        );
        // Act: reject oversized allocations before reading payload.
        std::fs::write(&stream, (MAX_CPU_FRAME_BYTES as u32 + 1).to_le_bytes()).unwrap();
        assert!(
            reader
                .sample(root.path(), Duration::ZERO)
                .cpu_percent
                .is_none()
        );
        assert!(reader.incomplete);
        // Act: no symlink traversal even for known sidecars.
        std::fs::remove_file(&stream).unwrap();
        std::os::unix::fs::symlink("/dev/zero", &stream).unwrap();
        assert!(
            reader
                .sample(root.path(), Duration::ZERO)
                .cpu_percent
                .is_none()
        );
    }
    #[test]
    fn terminal_activity_restores_matching_run_and_redacts_event_payloads() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let id = "a".repeat(32);
        let directory = root.path().join("runs").join(&id);
        std::fs::create_dir_all(&directory).unwrap();
        let status = Status {
            phase: Phase::Finished as i32,
            run_id: id.clone(),
            sequence: 12,
            steps: vec![workflow_protocol::Event {
                version: workflow_protocol::VERSION,
                sequence: 12,
                state: 2,
                invocation: (1_u64 << 53) + 1,
                payload_json: Some("secret".into()),
                error: Some("private diagnostic".into()),
                ..Default::default()
            }],
            metrics: vec![RunMetric {
                recording_bytes: Some(123),
                ..Default::default()
            }],
            ..Default::default()
        };
        save(&directory, &status).unwrap();
        let marker = Marker {
            run_id: id,
            ..Default::default()
        };
        let mut session = Session::new(Phase::Finished, Some(marker));
        // Act
        restore(&mut session, root.path());
        // Assert
        assert_eq!(session.metrics[0].recording_bytes, Some(123));
        assert_eq!(session.progress.rows()[0].invocation, (1_u64 << 53) + 1);
        assert_eq!(session.progress.rows()[0].payload_json, None);
        assert_eq!(
            session.progress.rows()[0].error.as_deref(),
            Some("step failed")
        );
        // Act / Assert: another marker cannot load this run's activity.
        session.marker.as_mut().unwrap().run_id = "b".repeat(32);
        session.metrics.clear();
        restore(&mut session, root.path());
        assert!(session.metrics.is_empty());
    }
}
