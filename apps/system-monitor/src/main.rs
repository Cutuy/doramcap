use config::MonitorConfig;
use crossbeam_channel::{Select, bounded};
use eyre::{Result, WrapErr, bail, ensure, eyre};
use monitor_core::{CgroupDictionary, CpuRecord, Snapshot, handoff_with_tasks};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use telemetry_protocol::{DictionaryEntry, Frame as Envelope, VERSION};

mod config;
mod recording;

const PERIOD: Duration = Duration::from_millis(100);

const DEFAULT_CHECK_RECORDS: usize = 4096;
const SOURCE: &str = "bpf";
const USAGE: &str = "system-monitor --config FILE.pbtxt\nsystem-monitor --check\nsystem-monitor --check-bpf [--records N]\nCollection is fixed at 10 Hz. Configuration schema: proto/monitor.proto.\nBPF collection requires --features bpf and privileged BPF access.\nTask diagnostics are experimental and require --features task-bpf.";

/// One process invocation: a capability probe or a configured session.
enum Launch {
    Check { attach_records: Option<usize> },
    Run(MonitorConfig),
}
impl Launch {
    fn parse(args: &[String]) -> Result<Self> {
        let args: Vec<_> = args.iter().map(String::as_str).collect();
        match args.as_slice() {
            ["--config", path] => Ok(Self::Run(config::load(Path::new(path))?)),
            ["--check"] => Ok(Self::Check {
                attach_records: None,
            }),
            ["--check-bpf"] => Ok(Self::Check {
                attach_records: Some(DEFAULT_CHECK_RECORDS),
            }),
            ["--check-bpf", "--records", records] => {
                let records = records.parse()?;
                ensure!(
                    (1..=1_048_576).contains(&records),
                    "--records must be in 1..1048576"
                );
                Ok(Self::Check {
                    attach_records: Some(records),
                })
            }
            ["--help" | "-h"] => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            _ => bail!("{USAGE}"),
        }
    }
}
fn unix_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .min(u64::MAX as u128) as u64
}
fn elapsed_ns(start: Instant) -> u64 {
    start.elapsed().as_nanos().min(u64::MAX as u128) as u64
}

#[derive(Serialize)]
struct Capabilities {
    architecture: &'static str,
    kernel: String,
    minimum_kernel: &'static str,
    kernel_version_ok: bool,
    cgroup_v2: bool,
    btf_readable: bool,
    possible_cpus: String,
    bpf_compiled: bool,
    task_bpf_compiled: bool,
    dora_compiled: bool,
    attachment_verified: bool,
}
fn capabilities() -> Result<Capabilities> {
    let kernel = fs::read_to_string("/proc/sys/kernel/osrelease")?
        .trim()
        .to_owned();
    let mut parts = kernel.split(['.', '-']);
    let major = parts.next().unwrap_or("0").parse::<u32>().unwrap_or(0);
    let minor = parts.next().unwrap_or("0").parse::<u32>().unwrap_or(0);
    Ok(Capabilities {
        architecture: std::env::consts::ARCH,
        kernel_version_ok: (major, minor) >= (6, 8),
        kernel,
        minimum_kernel: "6.8",
        cgroup_v2: Path::new("/sys/fs/cgroup/cgroup.controllers").exists(),
        btf_readable: File::open("/sys/kernel/btf/vmlinux").is_ok(),
        possible_cpus: fs::read_to_string("/sys/devices/system/cpu/possible")
            .unwrap_or_default()
            .trim()
            .into(),
        bpf_compiled: cfg!(feature = "bpf"),
        task_bpf_compiled: cfg!(feature = "task-bpf"),
        dora_compiled: cfg!(feature = "dora"),
        attachment_verified: false,
    })
}

struct Backend {
    monitor: cgroup_bpf::Backend,
    scratch: Vec<cgroup_bpf::CgroupRecord>,
}
impl Backend {
    fn new(records: usize) -> Result<Self> {
        let cap = capabilities()?;
        if !cap.kernel_version_ok || !cap.cgroup_v2 || !cap.btf_readable {
            bail!(
                "native backend requires Linux >=6.8, cgroup v2 and readable kernel BTF; run --check"
            );
        }
        Ok(Self {
            monitor: cgroup_bpf::Backend::new(records)?,
            scratch: Vec::with_capacity(records),
        })
    }
    fn collect(&mut self, snapshot: &mut Snapshot) -> Result<(), String> {
        let stats = self
            .monitor
            .collect_into(&mut self.scratch)
            .map_err(|e| e.to_string())?;
        snapshot.capacity_overflow = stats.overflow;
        for row in &self.scratch {
            let mut flags = monitor_core::FLAG_INCLUSIVE;
            if row.flags & cgroup_bpf::VALID != 0 {
                flags |= monitor_core::FLAG_RUNTIME;
            }
            if row.flags & cgroup_bpf::RAW_USER_SYSTEM != 0 {
                flags |= monitor_core::FLAG_USER_SYSTEM | monitor_core::FLAG_RAW_USER_SYSTEM;
            }
            if row.flags & cgroup_bpf::ROOT != 0 {
                flags |= monitor_core::FLAG_ROOT;
            }
            if row.flags & cgroup_bpf::READ_ERROR != 0 {
                flags |= monitor_core::FLAG_READ_ERROR;
            }
            snapshot.push(CpuRecord {
                cgroup_id: row.id,
                parent_id: row.parent_id,
                cpu: row.cpu,
                flags,
                runtime_ns: row.runtime_ns,
                user_ns: row.user_ns,
                system_ns: row.system_ns,
            });
        }
        Ok(())
    }
}

// Keep collecting into the preallocated discard buffer when the handoff is full.
// The next accepted snapshot reports the cumulative number of discarded samples.
fn collect_sample(
    producer: &monitor_core::Producer,
    discard: &mut Snapshot,
    sequence: u64,
    dropped: &mut u64,
    origin: Instant,
    collect: impl FnOnce(&mut Snapshot) -> Result<(), String>,
) -> Result<bool, String> {
    let mut lease = producer.try_acquire();
    let snapshot = match lease.as_mut() {
        Some(lease) => lease.snapshot_mut(),
        None => {
            discard.reset();
            discard
        }
    };
    snapshot.sequence = sequence;
    snapshot.capture_unix_ns = unix_ns();
    snapshot.monotonic_start_ns = elapsed_ns(origin);
    snapshot.dropped_batches = *dropped;
    collect(snapshot)?;
    snapshot.monotonic_end_ns = elapsed_ns(origin);
    if let Some(lease) = lease {
        lease.publish();
        Ok(true)
    } else {
        *dropped += 1;
        Ok(false)
    }
}

fn check(attach_records: Option<usize>) -> Result<()> {
    let mut capabilities = capabilities()?;
    if let Some(records) = attach_records {
        // Probe real kernel attachment before a lifecycle owner launches recording.
        let _backend = Backend::new(records)?;
        capabilities.attachment_verified = true;
    }
    println!("{}", serde_json::to_string_pretty(&capabilities)?);
    Ok(())
}

/// Collect on a fixed sampling grid; stop signals interrupt the wait between samples.
fn collect_until_stopped(
    stop: crossbeam_channel::Receiver<()>,
    limit: u64,
    period: Duration,
    mut collect: impl FnMut(u64, Instant) -> Result<(), String>,
) -> Result<u64, String> {
    let origin = Instant::now();
    let mut deadline = origin;
    let mut sequence = 0;
    while limit == 0 || sequence < limit {
        match stop.recv_deadline(deadline) {
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            _ => break,
        }
        collect(sequence, origin)?;
        sequence += 1;
        deadline += period;
        // Keep the sampling grid after slow collection without a catch-up storm.
        let now = Instant::now();
        while deadline < now {
            deadline += period;
        }
    }
    Ok(sequence)
}

fn run(config: MonitorConfig) -> Result<()> {
    let records = config.records as usize;
    let slots = config.queue_slots as usize;
    let tasks = config.task_capacity as usize;
    let dora = config.dora.is_some();
    let (control_tx, control_rx) = bounded::<std::result::Result<(), String>>(1);
    if tasks != 0 && !cfg!(feature = "task-bpf") {
        bail!("task_capacity requires --features task-bpf");
    }
    let backend = Backend::new(records)?;
    #[cfg(feature = "task-bpf")]
    let task_backend = if tasks != 0 {
        Some(task_accounting::Backend::new_experimental(tasks)?)
    } else {
        None
    };
    #[cfg(feature = "dora")]
    let mut publisher = if dora {
        Some(system_monitor_dora::Publisher::new({
            let control_tx = control_tx.clone();
            move |event| {
                let _ = control_tx.try_send(event);
            }
        })?)
    } else {
        None
    };
    #[cfg(not(feature = "dora"))]
    if dora {
        bail!("the dora section requires --features dora");
    }
    let mut output: Option<Box<dyn Write>> = if !config.output_path.is_empty() {
        Some(Box::new(BufWriter::new(File::create(&config.output_path)?)))
    } else if !dora && config.recording.is_none() {
        Some(Box::new(BufWriter::new(std::io::stdout())))
    } else {
        None
    };
    let mut recorder = config
        .recording
        .as_ref()
        .map(recording::Recorder::create)
        .transpose()?;
    let (producer, consumer) = handoff_with_tasks(slots, records, tasks).map_err(|e| eyre!(e))?;
    let (stop_tx, stop_rx) = bounded::<()>(1);
    ctrlc::set_handler(move || {
        let _ = control_tx.try_send(Ok(()));
    })
    .wrap_err("installing shutdown signal handler")?;
    let limit = config.samples;
    // Preallocate the discard batch before entering the periodic loop.
    let mut discard = Snapshot::with_capacities(records, tasks);
    let worker = std::thread::Builder::new()
        .name("monitor-collector".into())
        .spawn(move || {
            (|| -> Result<(u64, u64), String> {
                let mut backend = backend;
                #[cfg(feature = "task-bpf")]
                let mut task_backend = task_backend;
                let mut dropped = 0u64;
                let sequence =
                    collect_until_stopped(stop_rx, limit, PERIOD, |sequence, origin| {
                        collect_sample(
                            &producer,
                            &mut discard,
                            sequence,
                            &mut dropped,
                            origin,
                            |snapshot| {
                                backend.collect(snapshot)?;
                                #[cfg(feature = "task-bpf")]
                                if let Some(tasks) = task_backend.as_mut() {
                                    snapshot.task_losses = tasks
                                        .collect_into(&mut snapshot.task_records)
                                        .map_err(|e| e.to_string())?;
                                }
                                Ok(())
                            },
                        )?;
                        Ok(())
                    })?;
                Ok((sequence, dropped))
            })()
        })?;
    let mut event_error = None;
    let mut dictionary = CgroupDictionary::new(records);
    let mut dictionary_entries: Vec<DictionaryEntry> = Vec::with_capacity(records);
    let mut payload = Vec::new();
    let session = format!("{}-{}", unix_ns(), std::process::id());
    let mut published = 0u64;
    let mut dictionary_sent_at: Option<Instant> = None;
    let publish_result = (|| -> Result<()> {
        dora_app::environment::ready()?;
        loop {
            if let Some(recorder) = recorder.as_mut() {
                recorder.poll()?;
            }
            let mut select = Select::new_biased();
            let control = select.recv(&control_rx);
            let sample = consumer.register_recv(&mut select);
            let selected = if let Some(deadline) = recorder
                .as_ref()
                .and_then(recording::Recorder::next_flush_deadline)
            {
                match select.select_deadline(deadline) {
                    Ok(selected) => selected,
                    Err(_) => continue,
                }
            } else {
                select.select()
            };
            if selected.index() == control {
                if let Ok(Err(error)) = selected.recv(&control_rx) {
                    event_error.get_or_insert(error);
                }
                let _ = stop_tx.try_send(());
                continue;
            }
            debug_assert_eq!(selected.index(), sample);
            if let Ok(lease) = consumer.recv_selected(selected) {
                let snapshot = lease.snapshot();
                let rotated = recorder.as_ref().is_some_and(|r| r.rotation_due());
                if rotated {
                    recorder.as_mut().unwrap().rotate()?;
                }
                let dictionary_due = rotated
                    || dictionary_sent_at
                        .is_none_or(|time| time.elapsed() >= Duration::from_secs(2));
                if dictionary_due {
                    dictionary.update(snapshot);
                    dictionary_entries.clear();
                    dictionary_entries.extend(dictionary.entries().iter().map(Into::into));
                    let packet = telemetry_protocol::dictionary_packet(
                        &telemetry_protocol::DictionaryFrame {
                            version: VERSION,
                            session_id: session.clone(),
                            revision: dictionary.revision(),
                            overflow: dictionary.overflow(),
                            entries: &dictionary_entries,
                            capture_unix_ns: snapshot.capture_unix_ns,
                        },
                    );
                    if let Some(recorder) = recorder.as_mut() {
                        recorder.record(
                            true,
                            dictionary.revision(),
                            snapshot.capture_unix_ns,
                            &packet,
                            |_payload| {
                                #[cfg(feature = "dora")]
                                if let Some(publisher) = publisher.as_mut() {
                                    publisher.publish_serialized(
                                        true,
                                        dictionary.revision(),
                                        snapshot.capture_unix_ns,
                                        _payload.len(),
                                        |out| out.copy_from_slice(_payload),
                                    )?;
                                }
                                Ok(())
                            },
                        )?;
                    } else {
                        #[cfg(feature = "dora")]
                        if let Some(publisher) = publisher.as_mut() {
                            publisher.publish_dictionary(&packet)?;
                        }
                    }
                    dictionary_sent_at = Some(Instant::now());
                }
                let report = if tasks != 0 {
                    Some(monitor_core::kernel_reporting_snapshot(snapshot))
                } else {
                    None
                };
                let snapshot = report.as_ref().unwrap_or(snapshot);
                let mut envelope = Envelope {
                    version: VERSION,
                    snapshot,
                    source: SOURCE.into(),
                    task_collection_enabled: tasks != 0,
                    task_capacity: tasks,
                    session_id: session.clone(),
                    dictionary_revision: dictionary.revision(),
                    dictionary_full: false,
                    dictionary_overflow: dictionary.overflow(),
                    dictionary: &[] as &[DictionaryEntry],
                };
                if recorder.is_some() || dora {
                    let packet = telemetry_protocol::snapshot_packet(&envelope);
                    if let Some(recorder) = recorder.as_mut() {
                        recorder.record(
                            false,
                            snapshot.sequence,
                            snapshot.capture_unix_ns,
                            &packet,
                            |_payload| {
                                #[cfg(feature = "dora")]
                                if let Some(publisher) = publisher.as_mut() {
                                    publisher.publish_serialized(
                                        false,
                                        snapshot.sequence,
                                        snapshot.capture_unix_ns,
                                        _payload.len(),
                                        |out| out.copy_from_slice(_payload),
                                    )?;
                                }
                                Ok(())
                            },
                        )?;
                    } else {
                        #[cfg(feature = "dora")]
                        if let Some(publisher) = publisher.as_mut() {
                            publisher.publish_packet(
                                snapshot.sequence,
                                snapshot.capture_unix_ns,
                                &packet,
                            )?;
                        }
                    }
                }
                if let Some(output) = output.as_mut() {
                    // Standalone files remain self-contained; Dora uses separate topics.
                    envelope.dictionary = &dictionary_entries;
                    envelope.dictionary_full = true;
                    telemetry_protocol::encode(&envelope, &mut payload)?;
                    output.write_all(&(payload.len() as u32).to_le_bytes())?;
                    output.write_all(&payload)?;
                    output.flush()?;
                }
                published += 1;
            } else {
                break;
            }
        }
        if let Some(output) = output.as_mut() {
            output.flush()?;
        }
        if let Some(recorder) = recorder.take() {
            recorder.finish()?;
        }
        #[cfg(feature = "dora")]
        if let Some(publisher) = publisher.as_mut() {
            publisher.finish()?;
        }
        if let Some(error) = event_error {
            bail!(error);
        }
        Ok(())
    })();
    let _ = stop_tx.try_send(());
    let collected = worker
        .join()
        .map_err(|_| eyre!("collection thread panicked"))?
        .map_err(|e| eyre!(e));
    publish_result?;
    let (collected, dropped) = collected?;
    if !config.published_count_path.is_empty() {
        fs::write(&config.published_count_path, format!("{published}\n"))?;
    }
    eprintln!("session complete: collected={collected} published={published} dropped={dropped}");
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match Launch::parse(&args)? {
        Launch::Check { attach_records } => check(attach_records),
        Launch::Run(config) => run(config),
    }
    .wrap_err("system monitor failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn bounded_overload_counts_drops_and_recovers_after_drain() {
        // Arrange: the consumer cannot dequeue until explicitly released.
        let (producer, consumer) = monitor_core::handoff(2, 1).unwrap();
        let (release, commands) = mpsc::channel::<usize>();
        let (drained, batches) = mpsc::channel();
        let timeout = Duration::from_secs(30); // Hang guard only.
        let reader = std::thread::spawn(move || {
            for count in commands {
                let mut batch = Vec::new();
                for _ in 0..count {
                    let lease = consumer.try_recv().expect("published sample");
                    batch.push((lease.snapshot().sequence, lease.snapshot().dropped_batches));
                }
                assert!(consumer.try_recv().is_none());
                drained.send(batch).unwrap();
            }
        });
        let mut discard = Snapshot::with_capacity(1);
        let mut dropped = 0;
        let mut collected = Vec::new();
        let origin = Instant::now();

        // Act: fill both slots, then submit exactly three excess samples.
        let accepted: Vec<_> = (0..5)
            .map(|sequence| {
                collect_sample(
                    &producer,
                    &mut discard,
                    sequence,
                    &mut dropped,
                    origin,
                    |s| {
                        collected.push(s.sequence);
                        assert!(s.push(CpuRecord::default()));
                        Ok(())
                    },
                )
                .unwrap()
            })
            .collect();

        // Assert: collection continues even when publication is rejected.
        assert_eq!(accepted, [true, true, false, false, false]);
        assert_eq!(collected, [0, 1, 2, 3, 4]);
        assert_eq!(dropped, 3);
        assert_eq!(discard.records.len(), 1);
        assert_eq!(discard.capacity_overflow, 0);

        // Act: allow the consumer to drain the retained samples.
        release.send(2).unwrap();
        let retained = batches.recv_timeout(timeout).unwrap();

        // Assert
        assert_eq!(retained, [(0, 0), (1, 0)]);

        // Act: publish again after receiving the drain acknowledgement.
        let accepted =
            collect_sample(&producer, &mut discard, 5, &mut dropped, origin, |_| Ok(())).unwrap();
        release.send(1).unwrap();
        let recovered = batches.recv_timeout(timeout).unwrap();

        // Assert: the next accepted sample carries the exact cumulative loss.
        assert!(accepted);
        assert_eq!(recovered, [(5, 3)]);
        assert_eq!(dropped, 3);
        drop(release);
        reader.join().unwrap();
    }
}

#[cfg(test)]
mod wait_tests {
    use super::*;

    #[test]
    fn stop_interrupts_sampling_before_the_next_tick() {
        // Arrange
        let (stop, stop_rx) = bounded(1);
        let (sample, samples) = bounded(1);
        let (done, result) = bounded(1);
        let worker = std::thread::spawn(move || {
            let count =
                collect_until_stopped(stop_rx, 0, Duration::from_secs(30), |sequence, _| {
                    sample.send(sequence).unwrap();
                    Ok(())
                });
            done.send(count).unwrap();
        });
        assert_eq!(samples.recv_timeout(Duration::from_secs(2)).unwrap(), 0);

        // Act
        stop.send(()).unwrap();
        let count = result
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();

        // Assert
        assert_eq!(count, 1);
        worker.join().unwrap();
    }

    #[test]
    fn collection_failure_is_returned_and_closes_the_snapshot_stream() {
        // Arrange
        let (producer, consumer) = monitor_core::handoff(2, 1).unwrap();
        let (_stop, stop_rx) = bounded(1);
        let worker = std::thread::spawn(move || {
            collect_until_stopped(stop_rx, 0, PERIOD, |_, _| {
                producer.try_acquire().unwrap().publish();
                Err("collector failed".into())
            })
        });

        // Act
        let selected = {
            let mut select = Select::new();
            consumer.register_recv(&mut select);
            consumer
                .recv_selected(select.select_timeout(Duration::from_secs(2)).unwrap())
                .is_ok()
        };
        let error = worker.join().unwrap().unwrap_err();

        // Assert
        assert!(selected);
        assert_eq!(error, "collector failed");
        assert!(consumer.recv().is_err());
    }
}
