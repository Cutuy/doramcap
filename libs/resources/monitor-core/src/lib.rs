//! Numeric collection contracts and a reusable bounded handoff.
//!
//! Parent cgroup values are always supplied by the kernel backend. This crate
//! performs no hierarchy aggregation. Collection never serializes names.
use crossbeam_queue::ArrayQueue;
use serde::Serialize;
use std::sync::Arc;

pub const SCHEMA_VERSION: u32 = 1;
pub const FLAG_RUNTIME: u32 = 1;
pub const FLAG_USER_SYSTEM: u32 = 2;
pub const FLAG_INCLUSIVE: u32 = 4;
pub const FLAG_SYNTHETIC: u32 = 8;
pub const FLAG_ROOT: u32 = 16;
pub const FLAG_RAW_USER_SYSTEM: u32 = 32;
pub const FLAG_READ_ERROR: u32 = 64;
/// Virtual reporting namespace: id=0 denotes all PF_KTHREAD runtime.
/// Never a real cgroup ID; never add it to overlapping native cgroup totals.
pub const FLAG_VIRTUAL_KERNEL: u32 = 128;
pub const ALL_CPUS: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct CpuRecord {
    pub cgroup_id: u64,
    pub parent_id: u64,
    /// Logical CPU number; ALL_CPUS denotes an explicitly aggregate record.
    pub cpu: u32,
    /// Availability and semantics, not inferred from a numeric zero.
    pub flags: u32,
    pub runtime_ns: u64,
    pub user_ns: u64,
    pub system_ns: u64,
}

/// Host-wide cumulative USER_HZ ticks collected by the monitor.
#[derive(Clone, Copy, Debug, Default, Serialize, serde::Deserialize)]
pub struct CpuTotals {
    pub busy_ticks: u64,
    pub total_ticks: u64,
    pub cpu_cores: u32,
}

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub sequence: u64,
    pub capture_unix_ns: u64,
    pub monotonic_start_ns: u64,
    pub monotonic_end_ns: u64,
    pub dropped_batches: u64,
    pub capacity_overflow: u64,
    pub records: Vec<CpuRecord>,
    pub task_records: Vec<task_accounting::TaskRecord>,
    pub task_losses: task_accounting::Losses,
    pub cpu_totals: Option<CpuTotals>,
}

impl Snapshot {
    pub fn with_capacity(records: usize) -> Self {
        Self::with_capacities(records, 0)
    }
    pub fn with_capacities(records: usize, tasks: usize) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            sequence: 0,
            capture_unix_ns: 0,
            monotonic_start_ns: 0,
            monotonic_end_ns: 0,
            dropped_batches: 0,
            capacity_overflow: 0,
            records: Vec::with_capacity(records),
            task_records: Vec::with_capacity(tasks),
            task_losses: task_accounting::Losses::default(),
            cpu_totals: None,
        }
    }

    pub fn reset(&mut self) {
        self.records.clear();
        self.cpu_totals = None;
        self.task_records.clear();
        self.task_losses = task_accounting::Losses::default();
        self.capacity_overflow = 0;
    }

    /// Never grows the record buffer. A full buffer becomes a visible loss.
    pub fn push(&mut self, record: CpuRecord) -> bool {
        if self.records.len() == self.records.capacity() {
            self.capacity_overflow = self.capacity_overflow.saturating_add(1);
            false
        } else {
            self.records.push(record);
            true
        }
    }
}

pub trait Monitor {
    type Error: std::fmt::Display;
    fn collect(&mut self, snapshot: &mut Snapshot) -> Result<(), Self::Error>;
}

struct Shared {
    free: ArrayQueue<Snapshot>,
    ready: ArrayQueue<Snapshot>,
}

/// A single producer owns this endpoint. No cloning or blocking send API.
pub struct Producer(Arc<Shared>);
/// A single consumer owns this endpoint.
pub struct Consumer(Arc<Shared>);

pub fn handoff(
    slots: usize,
    records_per_slot: usize,
) -> Result<(Producer, Consumer), &'static str> {
    handoff_with_tasks(slots, records_per_slot, 0)
}
pub fn handoff_with_tasks(
    slots: usize,
    records_per_slot: usize,
    tasks_per_slot: usize,
) -> Result<(Producer, Consumer), &'static str> {
    if slots == 0 || records_per_slot == 0 {
        return Err("handoff slots and record capacity must be nonzero");
    }
    let shared = Arc::new(Shared {
        free: ArrayQueue::new(slots),
        ready: ArrayQueue::new(slots),
    });
    for _ in 0..slots {
        shared
            .free
            .push(Snapshot::with_capacities(records_per_slot, tasks_per_slot))
            .unwrap();
    }
    Ok((Producer(shared.clone()), Consumer(shared)))
}

pub struct WriteLease<'a> {
    owner: &'a Producer,
    snapshot: Option<Snapshot>,
}

impl Producer {
    pub fn try_acquire(&self) -> Option<WriteLease<'_>> {
        let mut snapshot = self.0.free.pop()?;
        snapshot.reset();
        Some(WriteLease {
            owner: self,
            snapshot: Some(snapshot),
        })
    }
}
impl WriteLease<'_> {
    pub fn snapshot_mut(&mut self) -> &mut Snapshot {
        self.snapshot.as_mut().unwrap()
    }
    pub fn publish(mut self) {
        // A free slot was removed before this publish; ready cannot be full.
        let snapshot = self.snapshot.take().unwrap();
        if let Err(snapshot) = self.owner.0.ready.push(snapshot) {
            self.snapshot = Some(snapshot);
            panic!("handoff ownership invariant violated");
        }
    }
}
impl Drop for WriteLease<'_> {
    fn drop(&mut self) {
        if let Some(snapshot) = self.snapshot.take() {
            assert!(self.owner.0.free.push(snapshot).is_ok());
        }
    }
}

pub struct ReadLease<'a> {
    owner: &'a Consumer,
    snapshot: Option<Snapshot>,
}
impl Consumer {
    pub fn try_recv(&self) -> Option<ReadLease<'_>> {
        self.0.ready.pop().map(|snapshot| ReadLease {
            owner: self,
            snapshot: Some(snapshot),
        })
    }
}
impl ReadLease<'_> {
    pub fn snapshot(&self) -> &Snapshot {
        self.snapshot.as_ref().unwrap()
    }
}
impl Drop for ReadLease<'_> {
    fn drop(&mut self) {
        if let Some(snapshot) = self.snapshot.take() {
            assert!(self.owner.0.free.push(snapshot).is_ok());
        }
    }
}

/// Explicit test backend. Values never masquerade as native kernel counters.
pub struct SyntheticMonitor {
    elapsed_ns: u64,
    cores: u32,
}
impl SyntheticMonitor {
    pub fn new(cores: u32) -> Self {
        Self {
            elapsed_ns: 0,
            cores,
        }
    }
}
impl Monitor for SyntheticMonitor {
    type Error = std::convert::Infallible;
    fn collect(&mut self, snapshot: &mut Snapshot) -> Result<(), Self::Error> {
        self.elapsed_ns = self.elapsed_ns.saturating_add(100_000_000);
        for cpu in 0..self.cores {
            snapshot.push(CpuRecord {
                cgroup_id: 2,
                parent_id: 1,
                cpu,
                flags: FLAG_RUNTIME | FLAG_USER_SYSTEM | FLAG_INCLUSIVE | FLAG_SYNTHETIC,
                runtime_ns: self.elapsed_ns / 2,
                user_ns: self.elapsed_ns / 4,
                system_ns: self.elapsed_ns / 4,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_pool_requires_consumer_release() {
        let (producer, consumer) = handoff(1, 2).unwrap();
        let mut write = producer.try_acquire().unwrap();
        write.snapshot_mut().sequence = 42;
        write.publish();
        assert!(producer.try_acquire().is_none());
        let read = consumer.try_recv().unwrap();
        assert_eq!(read.snapshot().sequence, 42);
        assert!(producer.try_acquire().is_none());
        drop(read);
        assert!(producer.try_acquire().is_some());
    }
    #[test]
    fn overflow_is_visible_and_buffer_never_grows() {
        let mut snapshot = Snapshot::with_capacity(1);
        assert!(snapshot.push(CpuRecord::default()));
        assert!(!snapshot.push(CpuRecord::default()));
        assert_eq!(snapshot.records.capacity(), 1);
        assert_eq!(snapshot.capacity_overflow, 1);
    }
    #[test]
    fn abandoned_write_is_recycled() {
        let (producer, _) = handoff(1, 1).unwrap();
        drop(producer.try_acquire().unwrap());
        assert!(producer.try_acquire().is_some());
    }
    #[test]
    fn concurrent_transfer_preserves_order_and_ownership() {
        let (producer, consumer) = handoff(4, 2).unwrap();
        std::thread::scope(|scope| {
            scope.spawn(move || {
                for sequence in 0..5000 {
                    loop {
                        if let Some(mut lease) = producer.try_acquire() {
                            lease.snapshot_mut().sequence = sequence;
                            lease.publish();
                            break;
                        }
                        std::thread::yield_now();
                    }
                }
            });
            for sequence in 0..5000 {
                loop {
                    if let Some(lease) = consumer.try_recv() {
                        assert_eq!(lease.snapshot().sequence, sequence);
                        break;
                    }
                    std::thread::yield_now();
                }
            }
        });
    }
}

/// Publication-side reduction; retained kernel task identities never cross Dora.
pub fn kernel_reporting_snapshot(source: &Snapshot) -> Snapshot {
    let mut result = source.clone();
    result.task_records.clear();
    let mut totals = std::collections::BTreeMap::<u32, u64>::new();
    for row in &source.records {
        if row.cpu != ALL_CPUS {
            totals.entry(row.cpu).or_default();
        }
    }
    for task in &source.task_records {
        if task.identity.flags & task_accounting::KERNEL_THREAD == 0 {
            continue;
        }
        let value = totals.entry(task.identity.cpu).or_default();
        *value = value.saturating_add(task.charged_runtime_ns);
    }
    for (cpu, runtime_ns) in totals {
        result.records.push(CpuRecord {
            cpu,
            runtime_ns,
            flags: FLAG_RUNTIME | FLAG_VIRTUAL_KERNEL,
            ..Default::default()
        });
    }
    result
}

#[cfg(test)]
mod reporting_tests {
    use super::*;
    #[test]
    fn kernel_virtual_group_has_its_own_namespace_and_no_task_payload() {
        let mut input = Snapshot::with_capacities(2, 3);
        input.records.push(CpuRecord {
            cgroup_id: 0,
            cpu: 0,
            runtime_ns: 500,
            flags: FLAG_RUNTIME,
            ..Default::default()
        });
        input.records.push(CpuRecord {
            cgroup_id: 9,
            cpu: 1,
            ..Default::default()
        });
        for (cpu, flags, runtime) in [(0, 1, 10), (0, 1, 20), (0, 0, 999)] {
            input.task_records.push(task_accounting::TaskRecord {
                identity: task_accounting::TaskIdentity {
                    cpu,
                    flags,
                    ..Default::default()
                },
                charged_runtime_ns: runtime,
            });
        }
        input.task_losses.dropped_events = 3;
        let output = kernel_reporting_snapshot(&input);
        assert!(output.task_records.is_empty());
        assert_eq!(input.task_records.len(), 3);
        assert_eq!(output.records[0].runtime_ns, 500);
        let virtuals: Vec<_> = output
            .records
            .iter()
            .filter(|r| r.flags & FLAG_VIRTUAL_KERNEL != 0)
            .collect();
        assert_eq!(virtuals.len(), 2);
        assert_eq!(virtuals[0].runtime_ns, 30);
        assert_eq!(virtuals[1].runtime_ns, 0);
        assert_eq!(output.task_losses.dropped_events, 3);
    }
}
