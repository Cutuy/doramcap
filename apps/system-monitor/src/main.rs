use eyre::{Result, WrapErr, bail, eyre};
use monitor_core::{CpuRecord, Monitor, Snapshot, SyntheticMonitor, handoff_with_tasks};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use telemetry_protocol::{DictionaryEntry, Frame as Envelope, VERSION};

const PERIOD: Duration = Duration::from_millis(100);

struct Options {
    backend: String,
    samples: u64,
    records: usize,
    slots: usize,
    tasks: usize,
    dora: bool,
    check: bool,
    output: Option<PathBuf>,
    publisher_delay: Duration,
}
impl Options {
    fn parse() -> Result<Self> {
        let mut opts = Self {
            backend: "bpf".into(),
            samples: 0,
            records: 4096,
            slots: 8,
            tasks: 0,
            dora: false,
            check: false,
            output: None,
            publisher_delay: Duration::ZERO,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--backend" => {
                    opts.backend = args
                        .next()
                        .ok_or_else(|| eyre!("--backend requires a value"))?
                }
                "--samples" => opts.samples = value(&mut args, &arg)?.parse()?,
                "--records" => opts.records = value(&mut args, &arg)?.parse()?,
                "--task-capacity" => opts.tasks = value(&mut args, &arg)?.parse()?,
                "--queue-slots" => opts.slots = value(&mut args, &arg)?.parse()?,
                "--publisher-delay-ms" => {
                    opts.publisher_delay = Duration::from_millis(value(&mut args, &arg)?.parse()?)
                }
                "--output" => opts.output = Some(value(&mut args, &arg)?.into()),
                "--dora" => opts.dora = true,
                "--check" => opts.check = true,
                "--help" | "-h" => {
                    println!(
                        "system-monitor [--check] [--backend bpf|synthetic] [--samples N]\n  [--records N] [--queue-slots N] [--task-capacity N] [--dora]\n  [--output FILE] [--publisher-delay-ms N]\nCollection is fixed at 10 Hz. samples=0 runs until Dora stop or interruption.\nDefault backend bpf requires --features bpf and privileged BPF access.\nTask diagnostics are experimental and require --features task-bpf."
                    );
                    std::process::exit(0);
                }
                _ => bail!("unknown argument: {arg}"),
            }
        }
        if !matches!(opts.backend.as_str(), "bpf" | "synthetic") {
            bail!("backend must be bpf or synthetic");
        }
        if opts.records == 0
            || opts.records > 1_048_576
            || opts.slots == 0
            || opts.slots > 1024
            || opts.tasks > 1_048_576
        {
            bail!(
                "invalid capacities: records 1..1048576, queue-slots 1..1024, task-capacity 0..1048576"
            );
        }
        let bytes = opts
            .records
            .checked_mul(std::mem::size_of::<CpuRecord>())
            .and_then(|n| {
                opts.tasks
                    .checked_mul(std::mem::size_of::<task_accounting::TaskRecord>())
                    .and_then(|t| n.checked_add(t))
            })
            .and_then(|n| n.checked_mul(opts.slots + 2))
            .ok_or_else(|| eyre!("capacity overflow"))?;
        if bytes > 512 * 1024 * 1024 {
            bail!("configured acquisition storage exceeds 512 MiB; reduce capacities");
        }
        if opts.tasks != 0 && opts.backend == "synthetic" {
            bail!("experimental task diagnostics require the native bpf backend");
        }
        Ok(opts)
    }
}
fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next().ok_or_else(|| eyre!("{flag} requires a value"))
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

enum Backend {
    Synthetic(SyntheticMonitor),
    Native {
        monitor: cgroup_bpf::Backend,
        scratch: Vec<cgroup_bpf::CgroupRecord>,
    },
}
impl Backend {
    fn new(opts: &Options) -> Result<Self> {
        if opts.backend == "synthetic" {
            return Ok(Self::Synthetic(SyntheticMonitor::new(4)));
        }
        let cap = capabilities()?;
        if !cap.kernel_version_ok || !cap.cgroup_v2 || !cap.btf_readable {
            bail!(
                "native backend requires Linux >=6.8, cgroup v2 and readable kernel BTF; run --check"
            );
        }
        Ok(Self::Native {
            monitor: cgroup_bpf::Backend::new(opts.records)?,
            scratch: Vec::with_capacity(opts.records),
        })
    }
    fn collect(&mut self, snapshot: &mut Snapshot) -> Result<(), String> {
        match self {
            Self::Synthetic(monitor) => {
                monitor.collect(snapshot).unwrap();
            }
            Self::Native { monitor, scratch } => {
                let stats = monitor.collect_into(scratch).map_err(|e| e.to_string())?;
                snapshot.capacity_overflow = stats.overflow;
                for row in scratch.iter() {
                    let mut flags = monitor_core::FLAG_INCLUSIVE;
                    if row.flags & cgroup_bpf::VALID != 0 {
                        flags |= monitor_core::FLAG_RUNTIME;
                    }
                    if row.flags & cgroup_bpf::RAW_USER_SYSTEM != 0 {
                        flags |=
                            monitor_core::FLAG_USER_SYSTEM | monitor_core::FLAG_RAW_USER_SYSTEM;
                    }
                    if row.flags & cgroup_bpf::ROOT_UNAVAILABLE != 0 {
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
            }
        }
        Ok(())
    }
}

struct Dictionary {
    entries: Vec<DictionaryEntry>,
    index: HashMap<u64, usize>,
    unresolved: HashSet<u64>,
    capacity: usize,
    revision: u64,
    // Cumulative unresolved unique identities per published batch, not distinct
    // identities over the session. Never retain an unbounded overflow-ID set.
    overflow: u64,
    next_refresh: Instant,
}
impl Dictionary {
    fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            index: HashMap::with_capacity(capacity),
            unresolved: HashSet::with_capacity(capacity),
            capacity,
            revision: 0,
            overflow: 0,
            next_refresh: Instant::now(),
        }
    }
    fn update(&mut self, snapshot: &Snapshot, synthetic: bool) -> usize {
        let mut first_changed = self.entries.len();
        let now = Instant::now();
        // Discovery is rate limited even if unresolved/new IDs appear in every
        // batch. It also revisits names so renames and temporary failures recover.
        let mut paths = HashMap::new();
        let refresh = !synthetic && now >= self.next_refresh;
        if refresh {
            scan_cgroups(Path::new("/sys/fs/cgroup"), &mut paths, self.capacity, 0);
            self.next_refresh = now + Duration::from_secs(2);
            for (index, entry) in self.entries.iter_mut().enumerate() {
                if let Some(path) = paths.get(&entry.id) {
                    if entry.path.as_ref() != Some(path) {
                        entry.path = Some(path.clone());
                        first_changed = first_changed.min(index);
                        self.revision = self.revision.saturating_add(1);
                    }
                }
            }
        }
        self.unresolved.clear();
        for record in &snapshot.records {
            if self.index.contains_key(&record.cgroup_id) {
                continue;
            }
            if self.entries.len() == self.capacity {
                self.unresolved.insert(record.cgroup_id);
                continue;
            }
            let path = if synthetic {
                Some("/synthetic/cpu-load".into())
            } else {
                paths.remove(&record.cgroup_id)
            };
            self.index.insert(record.cgroup_id, self.entries.len());
            self.entries.push(DictionaryEntry {
                id: record.cgroup_id,
                parent_id: record.parent_id,
                path,
            });
            self.revision = self.revision.saturating_add(1);
        }
        self.overflow = self.overflow.saturating_add(self.unresolved.len() as u64);
        first_changed
    }
}
// Bound metadata strings and traversal depth as well as entry count. Deeper or
// longer paths remain unresolved (numeric records still carry identity).
fn scan_cgroups(path: &Path, paths: &mut HashMap<u64, String>, capacity: usize, depth: usize) {
    if paths.len() >= capacity || depth > 64 || path.as_os_str().len() > 4096 {
        return;
    }
    if let Ok(meta) = fs::metadata(path) {
        let relative = path.strip_prefix("/sys/fs/cgroup").unwrap_or(path);
        let name = if relative.as_os_str().is_empty() {
            "/".into()
        } else {
            format!("/{}", relative.to_string_lossy())
        };
        paths.insert(meta.ino(), name);
    }
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if paths.len() >= capacity {
                break;
            }
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                scan_cgroups(&entry.path(), paths, capacity, depth + 1);
            }
        }
    }
}
fn run(opts: Options) -> Result<()> {
    if opts.check {
        println!("{}", serde_json::to_string_pretty(&capabilities()?)?);
        return Ok(());
    }
    if opts.tasks != 0 && !cfg!(feature = "task-bpf") {
        bail!("--task-capacity requires --features task-bpf");
    }
    let backend = Backend::new(&opts)?;
    #[cfg(feature = "task-bpf")]
    let tasks = if opts.tasks != 0 {
        Some(task_accounting::Backend::new_experimental(opts.tasks)?)
    } else {
        None
    };
    #[cfg(feature = "dora")]
    let mut publisher = if opts.dora {
        Some(system_monitor_dora::Publisher::new_from_env()?)
    } else {
        None
    };
    #[cfg(not(feature = "dora"))]
    if opts.dora {
        bail!("--dora requires --features dora");
    }
    let mut output: Option<Box<dyn Write>> = if let Some(path) = opts.output.as_ref() {
        Some(Box::new(BufWriter::new(File::create(path)?)))
    } else if !opts.dora {
        Some(Box::new(BufWriter::new(std::io::stdout())))
    } else {
        None
    };
    let (producer, consumer) =
        handoff_with_tasks(opts.slots, opts.records, opts.tasks).map_err(|e| eyre!(e))?;
    let stop = Arc::new(AtomicBool::new(false));
    let signal_stop = stop.clone();
    ctrlc::set_handler(move || signal_stop.store(true, Ordering::Release))
        .wrap_err("installing shutdown signal handler")?;
    let done = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let worker_done = done.clone();
    let record_capacity = opts.records;
    let task_capacity = opts.tasks;
    let limit = opts.samples;
    // Preallocate the discard batch before entering the periodic loop.
    let mut discard = Snapshot::with_capacities(record_capacity, task_capacity);
    let worker = std::thread::Builder::new()
        .name("monitor-collector".into())
        .spawn(move || {
            let result = (|| -> Result<(u64, u64), String> {
                let mut backend = backend;
                #[cfg(feature = "task-bpf")]
                let mut tasks = tasks;
                let origin = Instant::now();
                let mut deadline = origin;
                let mut sequence = 0u64;
                let mut dropped = 0u64;
                while !worker_stop.load(Ordering::Acquire) && (limit == 0 || sequence < limit) {
                    if let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
                        std::thread::sleep(remaining);
                    }
                    if worker_stop.load(Ordering::Acquire) {
                        break;
                    }
                    let mut lease = producer.try_acquire();
                    let snapshot = match lease.as_mut() {
                        Some(lease) => lease.snapshot_mut(),
                        None => {
                            discard.reset();
                            &mut discard
                        }
                    };
                    snapshot.sequence = sequence;
                    snapshot.capture_unix_ns = unix_ns();
                    snapshot.monotonic_start_ns = elapsed_ns(origin);
                    snapshot.dropped_batches = dropped;
                    backend.collect(snapshot)?;
                    #[cfg(feature = "task-bpf")]
                    if let Some(tasks) = tasks.as_mut() {
                        snapshot.task_losses = tasks
                            .collect_into(&mut snapshot.task_records)
                            .map_err(|e| e.to_string())?;
                    }
                    snapshot.monotonic_end_ns = elapsed_ns(origin);
                    if let Some(lease) = lease {
                        lease.publish();
                    } else {
                        dropped += 1;
                    }
                    sequence += 1;
                    deadline += PERIOD;
                    // Avoid a catch-up storm after a slow collection. Keep future deadlines on the grid.
                    let now = Instant::now();
                    while deadline < now {
                        deadline += PERIOD;
                    }
                }
                Ok((sequence, dropped))
            })();
            worker_done.store(true, Ordering::Release);
            result
        })?;
    let mut dictionary = Dictionary::new(opts.records);
    let mut payload = Vec::new();
    let session = format!("{}-{}", unix_ns(), std::process::id());
    let mut published = 0u64;
    let mut dictionary_sent_at: Option<Instant> = None;
    let publish_result = (|| -> Result<()> {
        loop {
            #[cfg(feature = "dora")]
            if let Some(publisher) = publisher.as_mut() {
                if publisher.stop_requested()? {
                    stop.store(true, Ordering::Release);
                }
            }
            let finished = done.load(Ordering::Acquire) || worker.is_finished();
            if let Some(lease) = consumer.try_recv() {
                let snapshot = lease.snapshot();
                let dictionary_due =
                    dictionary_sent_at.is_none_or(|time| time.elapsed() >= Duration::from_secs(2));
                if dictionary_due {
                    dictionary.update(snapshot, opts.backend == "synthetic");
                    #[cfg(feature = "dora")]
                    if let Some(publisher) = publisher.as_mut() {
                        let packet = telemetry_protocol::dictionary_packet(
                            &telemetry_protocol::DictionaryFrame {
                                version: VERSION,
                                session_id: session.clone(),
                                revision: dictionary.revision,
                                overflow: dictionary.overflow,
                                entries: &dictionary.entries,
                                capture_unix_ns: snapshot.capture_unix_ns,
                            },
                        );
                        publisher.publish_dictionary(&packet)?;
                    }
                    dictionary_sent_at = Some(Instant::now());
                }
                let report = if opts.tasks != 0 {
                    Some(monitor_core::kernel_reporting_snapshot(snapshot))
                } else {
                    None
                };
                let snapshot = report.as_ref().unwrap_or(snapshot);
                let mut envelope = Envelope {
                    version: VERSION,
                    viewer_ingress: None,
                    snapshot,
                    source: opts.backend.clone(),
                    task_collection_enabled: opts.tasks != 0,
                    task_capacity: opts.tasks,
                    session_id: session.clone(),
                    dictionary_revision: dictionary.revision,
                    dictionary_full: false,
                    dictionary_overflow: dictionary.overflow,
                    dictionary: &[] as &[DictionaryEntry],
                };
                #[cfg(feature = "dora")]
                if let Some(publisher) = publisher.as_mut() {
                    publisher.publish_packet(
                        snapshot.sequence,
                        snapshot.capture_unix_ns,
                        &telemetry_protocol::snapshot_packet(&envelope),
                    )?;
                }
                if let Some(output) = output.as_mut() {
                    // Standalone files remain self-contained; Dora uses separate topics.
                    envelope.dictionary = &dictionary.entries;
                    envelope.dictionary_full = true;
                    telemetry_protocol::encode(&envelope, &mut payload)?;
                    output.write_all(&(payload.len() as u32).to_le_bytes())?;
                    output.write_all(&payload)?;
                }
                published += 1;
                if !opts.publisher_delay.is_zero() {
                    std::thread::sleep(opts.publisher_delay);
                }
            } else if finished {
                break;
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        if let Some(output) = output.as_mut() {
            output.flush()?;
        }
        #[cfg(feature = "dora")]
        if let Some(publisher) = publisher.as_mut() {
            publisher.finish()?;
        }
        Ok(())
    })();
    stop.store(true, Ordering::Release);
    let collected = worker
        .join()
        .map_err(|_| eyre!("collection thread panicked"))?
        .map_err(|e| eyre!(e));
    publish_result?;
    let (collected, dropped) = collected?;
    eprintln!("session complete: collected={collected} published={published} dropped={dropped}");
    Ok(())
}
fn main() -> Result<()> {
    run(Options::parse()?).wrap_err("system monitor failed")
}
