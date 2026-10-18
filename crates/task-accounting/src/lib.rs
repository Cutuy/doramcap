//! Experimental native scheduler-runtime diagnostics, separate from cgroup totals.
//!
//! Read README.md before enabling BPF: compile success is not target qualification.
//! All counters begin at attachment and include only runtime charged thereafter.
use std::fmt;

#[cfg(feature = "bpf")]
mod native;
#[cfg(feature = "bpf")]
pub use native::Backend;

pub const KERNEL_THREAD: u32 = 1;

/// Identity within one boot/session. Never use TID alone: Linux reuses TIDs.
/// A cgroup or CPU migration produces a distinct key, preserving attribution.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub struct TaskIdentity {
    pub start_boottime_ns: u64,
    pub cgroup_id: u64,
    pub tid: u32,
    pub tgid: u32,
    pub cpu: u32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub struct TaskRecord {
    pub identity: TaskIdentity,
    pub charged_runtime_ns: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub struct Losses {
    pub dropped_events: u64,
    pub dropped_runtime_ns: u64,
    pub identity_errors: u64,
}

#[derive(Debug)]
pub enum Error {
    InvalidCapacity,
    OutputCapacity {
        required: usize,
        actual: usize,
    },
    Unsupported(&'static str),
    Io(std::io::Error),
    #[cfg(feature = "bpf")]
    Bpf(libbpf_rs::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity => write!(f, "task capacity must be 1..=u32::MAX"),
            Self::OutputCapacity { required, actual } => write!(
                f,
                "task output capacity {actual} is smaller than {required}"
            ),
            Self::Unsupported(reason) => f.write_str(reason),
            Self::Io(e) => e.fmt(f),
            #[cfg(feature = "bpf")]
            Self::Bpf(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
#[cfg(feature = "bpf")]
impl From<libbpf_rs::Error> for Error {
    fn from(e: libbpf_rs::Error) -> Self {
        Self::Bpf(e)
    }
}
pub type Result<T> = std::result::Result<T, Error>;

/// Replay/reference accumulator for validation of identity and capacity semantics.
/// It does not observe the system; the optional BPF backend performs acquisition.
/// Allocation occurs only in new(). A linear lookup keeps the reference simple.
pub struct RuntimeAccumulator {
    records: Vec<TaskRecord>,
    capacity: usize,
    losses: Losses,
}
impl RuntimeAccumulator {
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity > u32::MAX as usize {
            return Err(Error::InvalidCapacity);
        }
        Ok(Self {
            records: Vec::with_capacity(capacity),
            capacity,
            losses: Losses::default(),
        })
    }
    pub fn charge(&mut self, identity: TaskIdentity, runtime_ns: u64) {
        if runtime_ns == 0 {
            return;
        }
        if let Some(record) = self.records.iter_mut().find(|r| r.identity == identity) {
            record.charged_runtime_ns = record.charged_runtime_ns.saturating_add(runtime_ns);
        } else if self.records.len() < self.capacity {
            self.records.push(TaskRecord {
                identity,
                charged_runtime_ns: runtime_ns,
            });
        } else {
            self.losses.dropped_events = self.losses.dropped_events.saturating_add(1);
            self.losses.dropped_runtime_ns =
                self.losses.dropped_runtime_ns.saturating_add(runtime_ns);
        }
    }
    pub fn records(&self) -> &[TaskRecord] {
        &self.records
    }
    pub fn losses(&self) -> Losses {
        self.losses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> TaskIdentity {
        TaskIdentity {
            start_boottime_ns: 100,
            cgroup_id: 10,
            tid: 20,
            tgid: 20,
            cpu: 0,
            flags: 0,
        }
    }
    #[test]
    fn migration_pid_reuse_and_kernel_threads_remain_distinct() {
        let first = identity();
        let migrated = TaskIdentity { cpu: 1, ..first };
        let moved = TaskIdentity {
            cgroup_id: 11,
            ..migrated
        };
        let reused = TaskIdentity {
            start_boottime_ns: 200,
            ..first
        };
        let kernel = TaskIdentity {
            tid: 30,
            tgid: 30,
            flags: KERNEL_THREAD,
            ..first
        };
        let mut accumulator = RuntimeAccumulator::new(5).unwrap();
        for key in [first, migrated, moved, reused, kernel] {
            accumulator.charge(key, 10);
        }
        accumulator.charge(first, 15);
        assert_eq!(accumulator.records().len(), 5);
        assert_eq!(accumulator.records()[0].charged_runtime_ns, 25);
        assert_eq!(accumulator.records()[4].identity.flags, KERNEL_THREAD);
        assert_eq!(accumulator.losses(), Losses::default());
    }
    #[test]
    fn capacity_loss_preserves_existing_cumulative_records() {
        let mut accumulator = RuntimeAccumulator::new(1).unwrap();
        accumulator.charge(identity(), 10);
        accumulator.charge(
            TaskIdentity {
                tid: 99,
                ..identity()
            },
            25,
        );
        accumulator.charge(identity(), 50);
        assert_eq!(accumulator.records()[0].charged_runtime_ns, 60);
        assert_eq!(accumulator.losses().dropped_events, 1);
        assert_eq!(accumulator.losses().dropped_runtime_ns, 25);
    }
    #[test]
    fn collection_does_not_invent_uncharged_running_time() {
        let mut accumulator = RuntimeAccumulator::new(1).unwrap();
        accumulator.charge(identity(), 100);
        for _ in 0..10 {
            assert_eq!(accumulator.records()[0].charged_runtime_ns, 100);
        }
        // A long-running task's next accounting charge can span collection windows.
        accumulator.charge(identity(), 1_000);
        assert_eq!(accumulator.records()[0].charged_runtime_ns, 1_100);
    }
    #[test]
    fn c_abi_sizes_are_fixed() {
        assert_eq!(std::mem::size_of::<TaskIdentity>(), 32);
        assert_eq!(std::mem::size_of::<Losses>(), 24);
    }
}
