//! Owner-driven io_uring with independent pending and registration capacities.
use crate::timeline::Timeline;
use crate::{
    file_offsets::{FileRange, OffsetAllocator},
    managed_buffers::{Allocation, BufferLease, BufferPool},
    mapped::{MmapRegion, ReadLease},
    observability::{self, EventData, EventKind, Ids, Observer, elapsed_us},
    slabs::{Rejected, SealedSlab, SlabAllocation, SlabPool},
};
use io_uring::{IoUring, opcode, types};
use std::{collections::HashMap, fs::File, io, os::fd::AsRawFd, rc::Rc, sync::Arc};

/// Kernel fixed-buffer range limit. One allocation can occupy multiple entries.
#[cfg(not(test))]
const REGISTRATION_BYTES: usize = 1 << 30;
// Exercise multi-entry registration and rollback without pinning gigabytes in unit tests.
#[cfg(test)]
const REGISTRATION_BYTES: usize = 4096;
const CANCEL_TAG: u64 = 1 << 63;

#[derive(Clone, Copy, Debug)]
pub struct WriterConfig {
    pub queue_depth: u32,
    pub pending_capacity: usize,
    pub registration_capacity: u32,
    /// Maximum bytes per SQE; aligned down to the direct-I/O alignment.
    pub max_write_bytes: usize,
}
impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            queue_depth: 8,
            pending_capacity: 1024,
            registration_capacity: 1024,
            max_write_bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Debug)]
pub struct AppendError {
    pub error: io::Error,
    pub lease: BufferLease,
}
impl std::fmt::Display for AppendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for AppendError {}
/// Bounded diagnostic event. A CQE describes write completion, before fsync.
#[cfg(feature = "diagnostics")]
#[derive(Clone, Copy, Debug, Default)]
pub struct WriteEvent {
    pub sequence: u64,
    pub file: u64,
    pub elapsed_us: u64,
    pub offset: u64,
    pub len: u64,
    pub slot: usize,
    pub result: i32,
    /// False: SQE queued. True: corresponding write CQE retired.
    pub completion: bool,
    /// Submission-call to CQE observation, including owner polling delay.
    pub latency_us: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum RegionKey {
    Managed(usize),
    Slab(u64),
    Mapped(u64),
}
#[derive(Clone)]
enum AllocationOwner {
    Managed(Rc<Allocation>),
    Slab(Arc<SlabAllocation>),
    Mapped(MmapRegion),
}
impl AllocationOwner {
    fn region_id(&self) -> u64 {
        match self {
            Self::Managed(a) => a.region_id,
            Self::Slab(a) => a.id,
            Self::Mapped(a) => a.id(),
        }
    }
    fn key(&self) -> RegionKey {
        match self {
            Self::Managed(a) => RegionKey::Managed(Rc::as_ptr(a) as usize),
            Self::Slab(a) => RegionKey::Slab(a.id),
            Self::Mapped(a) => RegionKey::Mapped(a.id()),
        }
    }
    fn ptr(&self) -> *mut u8 {
        match self {
            Self::Managed(a) => a.ptr(),
            Self::Slab(a) => a.ptr(),
            Self::Mapped(a) => a.ptr(),
        }
    }
    fn capacity(&self) -> usize {
        match self {
            Self::Managed(a) => a.capacity(),
            Self::Slab(a) => a.capacity(),
            Self::Mapped(a) => a.len(),
        }
    }
}
enum WriteSource {
    Managed(BufferLease),
    Slab(SealedSlab),
    Mapped(ReadLease),
    Prefixed {
        prefix: BufferLease,
        payload: ReadLease,
        prefix_segments: Option<Rc<[RegistrationSegment]>>,
    },
}
impl WriteSource {
    fn region_offset(&self) -> usize {
        match self {
            Self::Mapped(a) | Self::Prefixed { payload: a, .. } => a.offset(),
            _ => 0,
        }
    }
    fn ids(&self) -> Ids {
        match self {
            Self::Managed(a) => Ids {
                region: a.allocation().region_id,
                ..Default::default()
            },
            Self::Slab(a) => a.observation_ids(),
            Self::Mapped(a) | Self::Prefixed { payload: a, .. } => a.ids(),
        }
    }
    fn into_pending(self) -> Self {
        match self {
            Self::Slab(a) => Self::Slab(a.into_io()),
            Self::Mapped(a) => Self::Mapped(a.into_io()),
            Self::Prefixed {
                prefix,
                payload,
                prefix_segments,
            } => Self::Prefixed {
                prefix,
                payload: payload.into_io(),
                prefix_segments,
            },
            managed => managed,
        }
    }
    // The external payload is validated and registered separately from its prefix.
    fn primary_bytes(&self) -> &[u8] {
        match self {
            Self::Managed(a) => a.as_slice(),
            Self::Slab(a) => a.as_slice(),
            Self::Mapped(a) | Self::Prefixed { payload: a, .. } => a.as_slice(),
        }
    }
    fn len(&self) -> usize {
        self.primary_bytes().len()
            + match self {
                Self::Prefixed { prefix, .. } => prefix.as_slice().len(),
                _ => 0,
            }
    }
    fn capacity(&self) -> usize {
        match self {
            Self::Managed(a) => a.capacity(),
            Self::Slab(a) => a.capacity(),
            Self::Mapped(a) => a.region().len(),
            Self::Prefixed {
                prefix, payload, ..
            } => prefix.capacity() + payload.region().len(),
        }
    }
    fn owner(&self) -> AllocationOwner {
        match self {
            Self::Managed(a) => AllocationOwner::Managed(a.allocation().clone()),
            Self::Slab(a) => AllocationOwner::Slab(a.allocation().clone()),
            Self::Mapped(a) | Self::Prefixed { payload: a, .. } => {
                AllocationOwner::Mapped(a.region().clone())
            }
        }
    }
}
// Private, ring-local handles. Entries cannot be retired while any Pending exists.
// A handle never escapes Writer; after retirement all cache handles are discarded.
struct RegistrationSegment {
    slot: u16,
    start: usize,
    len: usize,
}

#[derive(Default)]
struct RequestObservation {
    id: u64,
    accepted_at: Option<std::time::Instant>,
    queued_at: Option<std::time::Instant>,
    attempts: u64,
    retries: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteState {
    AwaitingSubmission,
    AwaitingCqe,
}
#[derive(Clone, Copy, Debug)]
pub struct ActiveWrite {
    pub ids: Ids,
    pub state: WriteState,
    pub offset: u64,
    pub len: u64,
    pub completed_bytes: usize,
    pub age_us: Option<u64>,
    pub attempts: u64,
    pub retries: u64,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Counters {
    pub accepted: u64,
    pub written: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub rejected_capacity: u64,
    pub rejected_invalid: u64,
    pub rejected_closed: u64,
    pub rejected_other: u64,
    pub queued_operations: u64,
    pub observed_completions: u64,
    pub short_retries: u64,
    pub registration_hits: u64,
    pub registration_misses: u64,
    pub registration_failures: u64,
    pub syncs: u64,
    pub sync_failures: u64,
    pub submit_failures: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct StageSnapshot {
    pub writer_id: u64,
    pub file_id: u64,
    pub sampled_at: std::time::Instant,
    pub pending_capacity: usize,
    pub pending: usize,
    pub pending_bytes: u64,
    /// Includes SQEs queued in userspace and submitted operations awaiting observation.
    pub awaiting_cqe: usize,
    pub awaiting_submission: usize,
    pub oldest_age_us: Option<u64>,
    pub registered_regions: usize,
    pub registered_bytes: usize,
    pub registration_updates: u64,
    /// Current file totals. Completion bytes may contain holes.
    pub accepted_bytes: u64,
    pub written_bytes: u64,
    /// Prefix covered by the last successful sync after draining all writes.
    pub synced_bytes: u64,
    /// Writer-lifetime totals across file rotation.
    pub counters: Counters,
}
struct Pending {
    timeline: Timeline,
    observed: RequestObservation,
    lease: WriteSource,
    segments: Option<Rc<[RegistrationSegment]>>,
    range: FileRange,
    done: usize,
    request: Option<u64>,
    issued: usize,
    #[cfg(feature = "diagnostics")]
    submitted_at: Option<std::time::Instant>,
}
impl Pending {
    fn ids(&self, writer: u64, file: u64) -> Ids {
        Ids {
            writer,
            file,
            write: self.observed.id,
            operation: self.request.unwrap_or(0),
            ..self.lease.ids()
        }
    }
    fn data(&self, writer: u64, file: u64) -> EventData {
        EventData {
            ids: self.ids(writer, file),
            offset: self.range.offset,
            len: self.range.len,
            ..Default::default()
        }
    }
    fn advance(&mut self, result: i32) -> io::Result<bool> {
        if result < 0 {
            return Err(io::Error::from_raw_os_error(-result));
        }
        if result == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        if result as usize > self.issued || result as usize > self.lease.len() - self.done {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.done += result as usize;
        Ok(self.done == self.lease.len())
    }
}
struct Registered {
    allocation: AllocationOwner,
    len: usize,
}

pub struct Writer {
    // Drop ring before registered allocation owners; persistent submit failures retain leases too.
    ring: IoUring,
    observer: Observer,
    observation_writer: u64,
    observation_file: u64,
    counters: Counters,
    synced_bytes: u64,
    file: File,
    offsets: OffsetAllocator,
    slots: Vec<Option<Pending>>,
    outstanding: usize,
    cancellations: usize,
    cancelling: bool,
    error: Option<io::ErrorKind>,
    closed: bool,
    written_bytes: u64,
    registered: Option<BufferPool>,
    registrations: Vec<Option<Registered>>,
    table_live: bool,
    registration_cache: HashMap<RegionKey, Rc<[RegistrationSegment]>>,
    registration_updates: u64,
    alignment: usize,
    max_write_bytes: usize,
    next_request: u64,
    #[cfg(feature = "diagnostics")]
    events: [WriteEvent; 256],
    #[cfg(feature = "diagnostics")]
    event_sequence: u64,
    #[cfg(feature = "diagnostics")]
    generation: u64,
    #[cfg(feature = "diagnostics")]
    epoch: std::time::Instant,
    #[cfg(feature = "diagnostics")]
    submitted: u64,
    #[cfg(feature = "diagnostics")]
    completions: u64,
    #[cfg(test)]
    fail_submit: bool,
    #[cfg(test)]
    fail_sync: bool,
    #[cfg(test)]
    fail_truncate: bool,
    #[cfg(test)]
    fail_registration_after: Option<usize>,
}
impl Writer {
    pub fn new(file: File, capacity: u32) -> io::Result<Self> {
        Self::with_config(
            file,
            WriterConfig {
                queue_depth: capacity,
                ..WriterConfig::default()
            },
        )
    }
    pub fn with_config(file: File, config: WriterConfig) -> io::Result<Self> {
        Self::with_observer(file, config, Observer::default())
    }
    pub(crate) fn with_observer(
        file: File,
        config: WriterConfig,
        observer: Observer,
    ) -> io::Result<Self> {
        if config.queue_depth == 0
            || config.pending_capacity == 0
            || config.registration_capacity == 0
            || config.registration_capacity > u16::MAX as u32
            || config.max_write_bytes == 0
            || config.max_write_bytes > i32::MAX as usize
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        if flags & libc::O_APPEND != 0 || !file.metadata()?.is_file() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            ring: IoUring::builder()
                .setup_single_issuer()
                .setup_defer_taskrun()
                .setup_taskrun_flag()
                .build(config.queue_depth)?,
            observer,
            observation_writer: observability::next_id(),
            observation_file: observability::next_id(),
            counters: Counters::default(),
            synced_bytes: 0,
            file,
            offsets: OffsetAllocator::new(0, 1).unwrap(),
            slots: (0..config.pending_capacity).map(|_| None).collect(),
            outstanding: 0,
            cancellations: 0,
            cancelling: false,
            error: None,
            closed: false,
            written_bytes: 0,
            registered: None,
            registrations: (0..config.registration_capacity).map(|_| None).collect(),
            table_live: false,
            registration_cache: HashMap::new(),
            registration_updates: 0,
            alignment: 1,
            max_write_bytes: config.max_write_bytes,
            next_request: 1,
            #[cfg(feature = "diagnostics")]
            events: [WriteEvent::default(); 256],
            #[cfg(feature = "diagnostics")]
            event_sequence: 0,
            #[cfg(feature = "diagnostics")]
            generation: 0,
            #[cfg(feature = "diagnostics")]
            epoch: std::time::Instant::now(),
            #[cfg(feature = "diagnostics")]
            submitted: 0,
            #[cfg(feature = "diagnostics")]
            completions: 0,
            #[cfg(test)]
            fail_submit: false,
            #[cfg(test)]
            fail_sync: false,
            #[cfg(test)]
            fail_truncate: false,
            #[cfg(test)]
            fail_registration_after: None,
        })
    }
    /// Attach at any time. Existing writes remain inspectable; their earlier ages may be unknown.
    pub fn set_observer(&mut self, observer: Observer) {
        self.observer = observer;
    }
    pub(crate) fn observer(&self) -> &Observer {
        &self.observer
    }
    pub fn writer_id(&self) -> u64 {
        self.observation_writer
    }
    pub fn file_id(&self) -> u64 {
        self.observation_file
    }
    pub fn active_writes(&self) -> impl Iterator<Item = ActiveWrite> + '_ {
        self.slots.iter().flatten().map(|p| ActiveWrite {
            ids: p.ids(self.observation_writer, self.observation_file),
            state: if p.request.is_some() {
                WriteState::AwaitingCqe
            } else {
                WriteState::AwaitingSubmission
            },
            offset: p.range.offset,
            len: p.range.len,
            completed_bytes: p.done,
            age_us: elapsed_us(p.observed.accepted_at),
            attempts: p.observed.attempts,
            retries: p.observed.retries,
        })
    }
    pub fn snapshot(&self) -> StageSnapshot {
        let mut awaiting_cqe = 0;
        let mut pending_bytes = 0;
        let mut oldest = None;
        for p in self.slots.iter().flatten() {
            awaiting_cqe += usize::from(p.request.is_some());
            pending_bytes += p.range.len - p.done as u64;
            oldest = oldest.max(elapsed_us(p.observed.accepted_at));
        }
        StageSnapshot {
            writer_id: self.observation_writer,
            file_id: self.observation_file,
            sampled_at: std::time::Instant::now(),
            pending_capacity: self.slots.len(),
            pending: self.outstanding,
            pending_bytes,
            awaiting_cqe,
            awaiting_submission: self.outstanding - awaiting_cqe,
            oldest_age_us: oldest,
            registered_regions: self.registration_cache.len(),
            registered_bytes: self.registered_bytes(),
            registration_updates: self.registration_updates,
            accepted_bytes: self.accepted_bytes(),
            written_bytes: self.written_bytes,
            synced_bytes: self.synced_bytes,
            counters: self.counters,
        }
    }
    pub(crate) fn reject(&mut self, error: io::ErrorKind, source: Ids, len: u64) {
        match error {
            io::ErrorKind::WouldBlock => self.counters.rejected_capacity += 1,
            io::ErrorKind::InvalidInput => self.counters.rejected_invalid += 1,
            io::ErrorKind::BrokenPipe => self.counters.rejected_closed += 1,
            _ => self.counters.rejected_other += 1,
        }
        self.observer
            .record(EventKind::Rejected(error), || EventData {
                ids: Ids {
                    writer: self.observation_writer,
                    file: self.observation_file,
                    ..source
                },
                len,
                ..Default::default()
            });
    }
    pub fn direct(file: File, pool: BufferPool, alignment: usize) -> io::Result<Self> {
        Self::direct_with_config(file, pool, alignment, WriterConfig::default())
    }
    pub fn direct_with_config(
        file: File,
        pool: BufferPool,
        alignment: usize,
        config: WriterConfig,
    ) -> io::Result<Self> {
        Self::direct_with_observer(file, pool, alignment, config, Observer::default())
    }
    pub(crate) fn direct_with_observer(
        file: File,
        pool: BufferPool,
        alignment: usize,
        config: WriterConfig,
        observer: Observer,
    ) -> io::Result<Self> {
        if !matches!(alignment, 512 | 4096) || config.max_write_bytes < alignment {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        if flags & libc::O_DIRECT == 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut this = Self::with_observer(file, config, observer)?;
        this.alignment = alignment;
        this.max_write_bytes -= this.max_write_bytes % alignment;
        this.offsets = OffsetAllocator::new(0, alignment as u64).unwrap();
        this.registered = Some(pool.clone());
        for allocation in pool.allocations()? {
            this.register_allocation(AllocationOwner::Managed(allocation))?;
        }
        Ok(this)
    }
    fn register_allocation(
        &mut self,
        allocation: AllocationOwner,
    ) -> io::Result<Rc<[RegistrationSegment]>> {
        let ids = Ids {
            writer: self.observation_writer,
            file: self.observation_file,
            region: allocation.region_id(),
            ..Default::default()
        };
        if let Some(segments) = self.registration_cache.get(&allocation.key()) {
            self.counters.registration_hits += 1;
            self.observer
                .record(EventKind::RegistrationHit, || EventData {
                    ids,
                    ..Default::default()
                });
            return Ok(segments.clone());
        }
        self.counters.registration_misses += 1;
        let len = allocation.capacity() as u64;
        let start = self.observer.now();
        let result = self.register_allocation_inner(allocation);
        let kind = match &result {
            Ok(_) => EventKind::Registered,
            Err(e) => {
                self.counters.registration_failures += 1;
                EventKind::RegistrationFailed(e.kind())
            }
        };
        self.observer.record(kind, || EventData {
            ids,
            len,
            duration_us: elapsed_us(start),
            ..Default::default()
        });
        result
    }
    fn register_allocation_inner(
        &mut self,
        allocation: AllocationOwner,
    ) -> io::Result<Rc<[RegistrationSegment]>> {
        if !(allocation.ptr() as usize).is_multiple_of(self.alignment)
            || !allocation.capacity().is_multiple_of(self.alignment)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let key = allocation.key();
        let needed = allocation.capacity().div_ceil(REGISTRATION_BYTES);
        if self.registrations.iter().filter(|r| r.is_none()).count() < needed {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        if !self.table_live {
            self.ring
                .submitter()
                .register_buffers_sparse(self.registrations.len() as u32)?;
            self.table_live = true;
        }
        let mut segments = Vec::with_capacity(needed);
        for start in (0..allocation.capacity()).step_by(REGISTRATION_BYTES) {
            let index = self.registrations.iter().position(Option::is_none).unwrap();
            let len = (allocation.capacity() - start).min(REGISTRATION_BYTES);
            let iov = libc::iovec {
                iov_base: unsafe { allocation.ptr().add(start) }.cast(),
                iov_len: len,
            };
            #[cfg(test)]
            let injected = self
                .fail_registration_after
                .is_some_and(|n| start / REGISTRATION_BYTES >= n);
            #[cfg(not(test))]
            let injected = false;
            // SAFETY: registration owner is retained before any I/O can be admitted.
            let result = if injected {
                Err(io::ErrorKind::Other.into())
            } else {
                self.registration_updates += 1;
                unsafe {
                    self.ring
                        .submitter()
                        .register_buffers_update(index as u32, &[iov], None)
                }
            };
            if let Err(error) = result {
                // These entries have never been submitted. Undo only this allocation's entries.
                for i in 0..self.registrations.len() {
                    if self.registrations[i]
                        .as_ref()
                        .is_some_and(|r| r.allocation.key() == key)
                    {
                        let empty = libc::iovec {
                            iov_base: std::ptr::null_mut(),
                            iov_len: 0,
                        };
                        self.registration_updates += 1;
                        if unsafe {
                            self.ring
                                .submitter()
                                .register_buffers_update(i as u32, &[empty], None)
                        }
                        .is_ok()
                        {
                            self.registrations[i] = None;
                        } else {
                            self.error = Some(io::ErrorKind::Other);
                        }
                    }
                }
                return Err(error);
            }
            segments.push(RegistrationSegment {
                slot: index as u16,
                start,
                len,
            });
            self.registrations[index] = Some(Registered {
                allocation: allocation.clone(),
                len,
            });
        }
        let segments: Rc<[RegistrationSegment]> = segments.into();
        self.registration_cache.insert(key, segments.clone());
        Ok(segments)
    }
    /// Register a new managed allocation without submitting or waiting for I/O.
    pub fn prepare(&mut self, lease: &BufferLease) -> io::Result<()> {
        self.check_open()?;
        if let Some(pool) = &self.registered {
            if !pool.owns(lease) {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            self.register_allocation(AllocationOwner::Managed(lease.allocation().clone()))?;
        }
        Ok(())
    }
    fn check_open(&self) -> io::Result<()> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.error.map_or(Ok(()), |e| Err(e.into()))
    }
    pub fn check_capacity(&self, count: usize) -> io::Result<()> {
        self.check_open()?;
        if count > self.slots.len() - self.outstanding {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        Ok(())
    }
    /// Pre-register a producer pool before capture. Buffered writers need no table.
    /// A failure preserves successful registrations and all producer ownership.
    pub fn prepare_slabs(&mut self, pool: &SlabPool) -> io::Result<()> {
        self.check_open()?;
        if self.registered.is_some() {
            for a in pool.allocations() {
                self.register_allocation(AllocationOwner::Slab(a.clone()))?;
            }
        }
        Ok(())
    }
    /// Validate and warm registration while the caller retains the read lease.
    pub fn prepare_slab(&mut self, slab: &SealedSlab) -> io::Result<()> {
        self.check_open()?;
        if !slab.as_slice().len().is_multiple_of(self.alignment) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        if self.registered.is_some() {
            self.register_allocation(AllocationOwner::Slab(slab.allocation().clone()))?;
        }
        Ok(())
    }
    /// Number of buffer-table update syscall attempts, including rollback/retirement.
    pub fn registration_updates(&self) -> u64 {
        self.registration_updates
    }
    /// Validate the range and warm its mapping registration.
    pub fn prepare_mapped(&mut self, lease: &ReadLease) -> io::Result<()> {
        if !(lease.as_slice().as_ptr() as usize).is_multiple_of(self.alignment)
            || !lease.as_slice().len().is_multiple_of(self.alignment)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.prepare_region(lease.region())
    }
    /// Cache registration by mapping identity; retain its owner until retirement.
    pub fn prepare_region(&mut self, region: &MmapRegion) -> io::Result<()> {
        self.check_open()?;
        if self.registered.is_some() {
            self.register_allocation(AllocationOwner::Mapped(region.clone()))?;
        }
        Ok(())
    }
    /// Remove cached entries while idle; existing readers retain their mapping.
    pub fn retire_region(&mut self, region: &MmapRegion) -> io::Result<()> {
        self.retire_key(RegionKey::Mapped(region.id()))
    }
    /// Retire pool entries after all writer requests retire; readers remain valid.
    pub fn retire_slabs(&mut self, pool: &SlabPool) -> io::Result<()> {
        for allocation in pool.allocations() {
            self.retire_key(RegionKey::Slab(allocation.id))?;
        }
        Ok(())
    }
    fn retire_key(&mut self, key: RegionKey) -> io::Result<()> {
        if self.outstanding + self.cancellations != 0 {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        for (slot, entry) in self.registrations.iter_mut().enumerate() {
            if entry.as_ref().is_some_and(|r| r.allocation.key() == key) {
                let empty = libc::iovec {
                    iov_base: std::ptr::null_mut(),
                    iov_len: 0,
                };
                self.registration_updates += 1;
                // SAFETY: every write has retired; retain owner on update failure.
                if let Err(error) = unsafe {
                    self.ring
                        .submitter()
                        .register_buffers_update(slot as u32, &[empty], None)
                } {
                    self.error = Some(error.kind());
                    return Err(error);
                }
                let r = entry.as_ref().unwrap();
                self.observer
                    .record(EventKind::RegistrationRetired, || EventData {
                        ids: Ids {
                            writer: self.observation_writer,
                            file: self.observation_file,
                            region: r.allocation.region_id(),
                            ..Default::default()
                        },
                        len: r.len as u64,
                        ..Default::default()
                    });
                *entry = None;
            }
        }
        self.registration_cache.remove(&key);
        Ok(())
    }
    pub fn try_append_mapped(
        &mut self,
        lease: ReadLease,
    ) -> Result<FileRange, Rejected<ReadLease>> {
        self.try_append_source(WriteSource::Mapped(lease), "mapped", &Timeline::default())
            .map_err(|(error, source)| {
                let WriteSource::Mapped(value) = source else {
                    unreachable!()
                };
                Rejected { error, value }
            })
    }
    pub(crate) fn try_append_prefixed(
        &mut self,
        prefix: BufferLease,
        payload: ReadLease,
    ) -> Result<FileRange, Rejected<ReadLease>> {
        self.try_append_source(
            WriteSource::Prefixed {
                prefix,
                payload,
                prefix_segments: None,
            },
            "mapped",
            &Timeline::default(),
        )
        .map_err(|(error, source)| {
            let WriteSource::Prefixed { payload, .. } = source else {
                unreachable!()
            };
            Rejected {
                error,
                value: payload,
            }
        })
    }
    /// Retains caller ownership on rejected admission. No file range is consumed.
    pub fn try_append(&mut self, lease: BufferLease) -> Result<FileRange, AppendError> {
        self.try_append_tagged(lease, "managed", &Timeline::default())
    }
    pub(crate) fn try_append_tagged(
        &mut self,
        lease: BufferLease,
        path: &'static str,
        cause: &Timeline,
    ) -> Result<FileRange, AppendError> {
        self.try_append_source(WriteSource::Managed(lease), path, cause)
            .map_err(|(error, source)| {
                let WriteSource::Managed(lease) = source else {
                    unreachable!()
                };
                AppendError { error, lease }
            })
    }
    /// Submit the entire sealed byte range, retaining it through all split writes.
    pub fn try_append_slab(&mut self, slab: SealedSlab) -> Result<FileRange, Rejected<SealedSlab>> {
        self.try_append_source(WriteSource::Slab(slab), "slab", &Timeline::default())
            .map_err(|(error, source)| {
                let WriteSource::Slab(value) = source else {
                    unreachable!()
                };
                Rejected { error, value }
            })
    }
    fn try_append_source(
        &mut self,
        mut lease: WriteSource,
        path: &'static str,
        cause: &Timeline,
    ) -> Result<FileRange, (io::Error, WriteSource)> {
        let validation = (|| {
            self.check_capacity(1)?;
            if lease.primary_bytes().is_empty()
                || !(lease.primary_bytes().as_ptr() as usize).is_multiple_of(self.alignment)
                || !lease.primary_bytes().len().is_multiple_of(self.alignment)
            {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            if let (Some(pool), WriteSource::Managed(managed)) = (&self.registered, &lease)
                && !pool.owns(managed)
            {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            if let WriteSource::Prefixed {
                prefix,
                payload,
                prefix_segments,
            } = &mut lease
            {
                if prefix.as_slice().is_empty()
                    || !(prefix.as_slice().as_ptr() as usize).is_multiple_of(self.alignment)
                    || !prefix.as_slice().len().is_multiple_of(self.alignment)
                    || prefix
                        .as_slice()
                        .len()
                        .checked_add(payload.as_slice().len())
                        .is_none()
                {
                    return Err(io::ErrorKind::InvalidInput.into());
                }
                if self.registered.is_some() {
                    *prefix_segments = Some(self.register_allocation(AllocationOwner::Managed(
                        prefix.allocation().clone(),
                    ))?);
                }
            }
            if self.registered.is_some() {
                self.register_allocation(lease.owner()).map(Some)
            } else {
                Ok(None)
            }
        })();
        let segments = match validation {
            Ok(segments) => segments,
            Err(error) => {
                self.reject(error.kind(), lease.ids(), lease.len() as u64);
                return Err((error, lease));
            }
        };
        let reservation = match self.offsets.reserve(lease.len() as u64) {
            Ok(r) => r,
            Err(_) => {
                self.reject(io::ErrorKind::InvalidInput, lease.ids(), lease.len() as u64);
                return Err((io::ErrorKind::InvalidInput.into(), lease));
            }
        };
        let range = reservation.range();
        let index = self.slots.iter().position(Option::is_none).unwrap();
        let write_id = observability::next_id();
        let timeline = Timeline::write(
            Ids {
                writer: self.observation_writer,
                file: self.observation_file,
                write: write_id,
                ..lease.ids()
            },
            path,
            range,
            self.registered.is_some(),
            cause,
        );
        self.slots[index] = Some(Pending {
            timeline,
            observed: RequestObservation {
                id: write_id,
                accepted_at: self.observer.now(),
                ..Default::default()
            },
            lease: lease.into_pending(),
            segments,
            range,
            done: 0,
            request: None,
            issued: 0,
            #[cfg(feature = "diagnostics")]
            submitted_at: None,
        });
        reservation.commit();
        self.outstanding += 1;
        self.counters.accepted += 1;
        let pending = self.slots[index].as_ref().unwrap();
        self.observer.record(EventKind::Accepted, || {
            pending.data(self.observation_writer, self.observation_file)
        });
        Ok(range)
    }
    /// Compatibility API: rejected ownership is returned to the pool.
    pub fn append(&mut self, lease: BufferLease) -> io::Result<FileRange> {
        self.try_append(lease).map_err(|e| e.error)
    }
    fn schedule(&mut self) -> io::Result<()> {
        if self.cancelling {
            return Ok(());
        }
        for index in 0..self.slots.len() {
            let Some(p) = self.slots[index].as_mut() else {
                continue;
            };
            if p.request.is_some() {
                continue;
            }
            if self.next_request >= CANCEL_TAG {
                return Err(io::ErrorKind::Other.into());
            }
            let (bytes, position, segments) = match &p.lease {
                WriteSource::Prefixed {
                    prefix,
                    payload,
                    prefix_segments,
                } => {
                    let n = prefix.as_slice().len();
                    if p.done < n {
                        (&prefix.as_slice()[p.done..], p.done, prefix_segments)
                    } else {
                        (
                            &payload.as_slice()[p.done - n..],
                            payload.offset() + p.done - n,
                            &p.segments,
                        )
                    }
                }
                source => (
                    &source.primary_bytes()[p.done..],
                    source.region_offset() + p.done,
                    &p.segments,
                ),
            };
            let mut len = bytes.len().min(self.max_write_bytes);
            let entry = if let Some(segments) = segments {
                let r = &segments[position / REGISTRATION_BYTES];
                len = len.min(r.start + r.len - position);
                opcode::WriteFixed::new(
                    types::Fd(self.file.as_raw_fd()),
                    bytes.as_ptr(),
                    len as u32,
                    r.slot,
                )
                .offset(p.range.offset + p.done as u64)
                .build()
            } else {
                opcode::Write::new(types::Fd(self.file.as_raw_fd()), bytes.as_ptr(), len as u32)
                    .offset(p.range.offset + p.done as u64)
                    .build()
            }
            .user_data(self.next_request);
            // SAFETY: pending owns the allocation through this request's completion.
            if unsafe { self.ring.submission().push(&entry) }.is_err() {
                break;
            }
            p.timeline
                .queued(self.next_request, p.range.offset + p.done as u64, len);
            p.request = Some(self.next_request);
            p.issued = len;
            p.observed.attempts += 1;
            p.observed.queued_at = self.observer.now();
            self.counters.queued_operations += 1;
            self.observer.record(EventKind::SqeQueued, || EventData {
                offset: p.range.offset + p.done as u64,
                len: len as u64,
                ..p.data(self.observation_writer, self.observation_file)
            });
            #[cfg(feature = "diagnostics")]
            {
                p.submitted_at = Some(std::time::Instant::now());
            }
            self.next_request += 1;
            #[cfg(feature = "diagnostics")]
            {
                let range = FileRange {
                    offset: p.range.offset + p.done as u64,
                    len: len as u64,
                };
                self.event(index, range, 0, false);
            }
        }
        Ok(())
    }
    pub fn progress(&mut self, wait: bool) -> io::Result<()> {
        self.drain_progress(wait)?;
        self.error.map_or(Ok(()), |e| Err(e.into()))
    }
    /// Ownership has already transferred. Report submission failures on the next
    /// admission or barrier, rather than falsely returning rejected ownership.
    pub(crate) fn kick_accepted(&mut self) {
        if let Err(error) = self.progress(false) {
            self.error.get_or_insert(error.kind());
        }
    }
    fn drain_progress(&mut self, wait: bool) -> io::Result<()> {
        #[cfg(test)]
        if self.fail_submit {
            self.counters.submit_failures += 1;
            self.observer
                .record(EventKind::SubmitFailed(io::ErrorKind::Other), || {
                    EventData {
                        ids: Ids {
                            writer: self.observation_writer,
                            file: self.observation_file,
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                });
            return Err(io::ErrorKind::Other.into());
        }
        self.schedule()?;
        loop {
            let active = self.slots.iter().flatten().any(|p| p.request.is_some());
            match self
                .ring
                .submit_and_wait(usize::from(wait && (active || self.cancellations > 0)))
            {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                result => {
                    let submitted = match result {
                        Ok(n) => n,
                        Err(error) => {
                            self.counters.submit_failures += 1;
                            self.observer
                                .record(EventKind::SubmitFailed(error.kind()), || EventData {
                                    ids: Ids {
                                        writer: self.observation_writer,
                                        file: self.observation_file,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                });
                            return Err(error);
                        }
                    };
                    #[cfg(feature = "diagnostics")]
                    {
                        self.submitted += submitted as u64;
                    }
                    #[cfg(not(feature = "diagnostics"))]
                    let _ = submitted;
                    break;
                }
            }
        }
        loop {
            let completion = self
                .ring
                .completion()
                .next()
                .map(|c| (c.user_data(), c.result()));
            let Some((id, result)) = completion else {
                break;
            };
            self.retire_completion(id, result);
        }
        Ok(())
    }
    fn retire_completion(&mut self, id: u64, result: i32) {
        if id & CANCEL_TAG != 0 {
            self.cancellations -= 1;
            return;
        }
        let index = self
            .slots
            .iter()
            .position(|p| p.as_ref().is_some_and(|p| p.request == Some(id)))
            .expect("unique request owns completion");
        #[cfg(feature = "diagnostics")]
        {
            let p = self.slots[index].as_ref().unwrap();
            let range = FileRange {
                offset: p.range.offset + p.done as u64,
                len: p.issued as u64,
            };
            self.completions += 1;
            self.event(index, range, result, true);
        }
        let p = self.slots[index].as_mut().unwrap();
        self.counters.observed_completions += 1;
        self.observer.record(EventKind::CqeObserved, || EventData {
            offset: p.range.offset + p.done as u64,
            len: p.issued as u64,
            result: Some(result),
            duration_us: elapsed_us(p.observed.queued_at),
            ..p.data(self.observation_writer, self.observation_file)
        });
        p.timeline.completed(result);
        p.request = None;
        let outcome = p.advance(result);
        match outcome {
            Ok(false) if !self.cancelling && p.done.is_multiple_of(self.alignment) => {
                if (result as usize) < p.issued {
                    p.observed.retries += 1;
                    self.counters.short_retries += 1;
                    p.timeline.outcome(EventKind::Retry);
                    self.observer.record(EventKind::Retry, || {
                        p.data(self.observation_writer, self.observation_file)
                    });
                }
            }
            other => {
                let kind = match &other {
                    Ok(true) => {
                        self.counters.written += 1;
                        EventKind::Written
                    }
                    _ if self.cancelling => {
                        self.counters.cancelled += 1;
                        EventKind::Cancelled
                    }
                    Err(e) => {
                        self.counters.failed += 1;
                        EventKind::Failed(e.kind())
                    }
                    _ => {
                        self.counters.failed += 1;
                        EventKind::Failed(io::ErrorKind::InvalidData)
                    }
                };
                p.timeline.outcome(kind);
                self.observer.record(kind, || {
                    p.data(self.observation_writer, self.observation_file)
                });
                match other {
                    Ok(true) => self.written_bytes += p.range.len,
                    Err(e) => {
                        self.error.get_or_insert(e.kind());
                    }
                    _ => {
                        self.error.get_or_insert(if self.cancelling {
                            io::ErrorKind::Interrupted
                        } else {
                            io::ErrorKind::InvalidData
                        });
                    }
                }
                self.slots[index] = None;
                self.outstanding -= 1;
            }
        }
    }
    pub fn flush_written(&mut self) -> io::Result<()> {
        while self.outstanding + self.cancellations > 0 {
            self.drain_progress(true)?;
        }
        self.error.map_or(Ok(()), |e| Err(e.into()))
    }
    pub fn outstanding(&self) -> usize {
        self.outstanding
    }
    pub fn pending_capacity(&self) -> usize {
        self.slots.len()
    }
    pub fn written_bytes(&self) -> u64 {
        self.written_bytes
    }
    pub fn accepted_bytes(&self) -> u64 {
        self.offsets.next_offset()
    }
    pub fn registered_bytes(&self) -> usize {
        self.registrations.iter().flatten().map(|r| r.len).sum()
    }
    pub fn registered_ranges(&self) -> usize {
        self.registrations.iter().flatten().count()
    }
    pub fn in_flight_bytes(&self) -> usize {
        self.slots
            .iter()
            .flatten()
            .map(|p| p.lease.capacity())
            .sum()
    }
    pub fn sqpoll_enabled(&self) -> bool {
        self.ring.params().is_setup_sqpoll()
    }
    /// Explicit idle-only trim. Ordinary completion never frees retained allocations.
    pub fn trim(&mut self, pool: &BufferPool) -> io::Result<usize> {
        if self.outstanding + self.cancellations != 0 || pool.available() != pool.capacity() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        if let Some(registered) = &self.registered {
            // Compare through a temporary handle-free snapshot rather than borrowing bytes.
            if !registered.same_pool(pool) {
                return Err(io::ErrorKind::InvalidInput.into());
            }
        }
        if self.table_live {
            self.ring.submitter().unregister_buffers()?;
            self.table_live = false;
            self.registration_cache.clear();
            for r in &mut self.registrations {
                if let Some(entry) = r.as_ref() {
                    self.observer
                        .record(EventKind::RegistrationRetired, || EventData {
                            ids: Ids {
                                writer: self.observation_writer,
                                file: self.observation_file,
                                region: entry.allocation.region_id(),
                                ..Default::default()
                            },
                            len: entry.len as u64,
                            ..Default::default()
                        });
                }
                *r = None;
            }
        }
        pool.trim().map_err(Into::into)
    }
    fn sync_file(&mut self, covered: u64) -> io::Result<()> {
        let start = self.observer.now();
        #[cfg(test)]
        let injected = self.fail_sync;
        #[cfg(not(test))]
        let injected = false;
        let result = if injected {
            Err(io::Error::from_raw_os_error(libc::EIO))
        } else {
            self.file.sync_all()
        };
        self.counters.syncs += 1;
        let kind = match &result {
            Ok(()) => {
                self.synced_bytes = covered;
                EventKind::SyncCompleted
            }
            Err(e) => {
                self.counters.sync_failures += 1;
                EventKind::SyncFailed(e.kind())
            }
        };
        self.observer.record(kind, || EventData {
            ids: Ids {
                writer: self.observation_writer,
                file: self.observation_file,
                ..Default::default()
            },
            len: covered,
            duration_us: elapsed_us(start),
            ..Default::default()
        });
        result
    }
    pub fn sync(&mut self) -> io::Result<()> {
        self.flush_written()?;
        self.sync_file(self.accepted_bytes())
    }
    pub fn cancel_and_drain(&mut self) -> io::Result<()> {
        self.closed = true;
        self.cancelling = true;
        for index in 0..self.slots.len() {
            let Some(p) = &self.slots[index] else {
                continue;
            };
            let Some(id) = p.request else {
                self.counters.cancelled += 1;
                p.timeline.outcome(EventKind::Cancelled);
                self.observer.record(EventKind::Cancelled, || {
                    p.data(self.observation_writer, self.observation_file)
                });
                self.slots[index] = None;
                self.outstanding -= 1;
                continue;
            };
            let entry = opcode::AsyncCancel::new(id)
                .build()
                .user_data(CANCEL_TAG | id);
            if unsafe { self.ring.submission().push(&entry) }.is_err() {
                let submitted = self.ring.submit()?;
                #[cfg(feature = "diagnostics")]
                {
                    self.submitted += submitted as u64;
                }
                #[cfg(not(feature = "diagnostics"))]
                let _ = submitted;
                unsafe { self.ring.submission().push(&entry) }
                    .map_err(|_| io::ErrorKind::WouldBlock)?;
            }
            self.cancellations += 1;
        }
        self.flush_written()
    }
    pub fn finalize_at(&mut self, logical_len: u64) -> io::Result<()> {
        self.closed = true;
        if logical_len > self.accepted_bytes() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.flush_written()?;
        #[cfg(test)]
        if self.fail_truncate {
            return Err(io::Error::from_raw_os_error(libc::EIO));
        }
        self.file.set_len(logical_len)?;
        self.synced_bytes = self.synced_bytes.min(logical_len);
        self.sync_file(logical_len)
    }
    pub fn close_at(mut self, logical_len: u64) -> io::Result<()> {
        self.finalize_at(logical_len)
    }
    pub fn close(mut self) -> io::Result<()> {
        self.closed = true;
        self.sync()
    }
    pub fn replace_file(&mut self, file: File) -> io::Result<()> {
        if !self.closed || self.outstanding + self.cancellations != 0 || self.error.is_some() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        if flags & libc::O_APPEND != 0
            || !file.metadata()?.is_file()
            || (self.alignment > 1 && flags & libc::O_DIRECT == 0)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.file = file;
        self.observation_file = observability::next_id();
        self.synced_bytes = 0;
        self.observer.record(EventKind::FileRotated, || EventData {
            ids: Ids {
                writer: self.observation_writer,
                file: self.observation_file,
                ..Default::default()
            },
            ..Default::default()
        });
        self.offsets = OffsetAllocator::new(0, self.alignment as u64).unwrap();
        self.written_bytes = 0;
        self.closed = false;
        #[cfg(feature = "diagnostics")]
        {
            self.generation += 1;
        }
        Ok(())
    }
    #[cfg(feature = "diagnostics")]
    fn event(&mut self, slot: usize, range: FileRange, result: i32, completion: bool) {
        self.event_sequence += 1;
        self.events[((self.event_sequence - 1) % 256) as usize] = WriteEvent {
            sequence: self.event_sequence,
            file: self.generation,
            elapsed_us: self.epoch.elapsed().as_micros().min(u64::MAX as u128) as u64,
            offset: range.offset,
            len: range.len,
            slot,
            result,
            completion,
            latency_us: if completion {
                self.slots[slot]
                    .as_ref()
                    .and_then(|p| p.submitted_at)
                    .map(|t| t.elapsed().as_micros().min(u64::MAX as u128) as u64)
            } else {
                None
            },
        };
    }
    /// Latest 256 events in order; sequence gaps reveal overwritten history.
    #[cfg(feature = "diagnostics")]
    pub fn events(&self) -> impl Iterator<Item = WriteEvent> + '_ {
        let first = self.event_sequence.saturating_sub(256);
        (first..self.event_sequence).map(|n| self.events[(n % 256) as usize])
    }
    #[cfg(feature = "diagnostics")]
    pub fn submitted_operations(&self) -> u64 {
        self.submitted
    }
    #[cfg(feature = "diagnostics")]
    pub fn completion_count(&self) -> u64 {
        self.completions
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        if self.flush_written().is_err() && self.outstanding != 0 {
            // A persistent ring-enter failure prevents proving retirement. Retain
            // allocations rather than free memory potentially still read by Linux.
            for p in &mut self.slots {
                if let Some(p) = p.take() {
                    std::mem::forget(p);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_buffers::BufferPool;
    #[test]
    fn prefixed_payload_holds_both_owners_through_short_write_and_failure() {
        use crate::mapped::MmapRegion;
        let path = std::env::temp_dir().join(format!("prefixed-failure-{}", std::process::id()));
        let file = File::options()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        let mut w = Writer::new(file, 1).unwrap();
        let prefixes = BufferPool::new(1, 4096, 4096).unwrap();
        let region = MmapRegion::new(
            memmap2::MmapOptions::new().len(8192).map_anon().unwrap(),
            Default::default(),
        );
        let lease = region.acquire(4096..8192).unwrap().seal(4096).unwrap();
        let ptr = lease.as_slice().as_ptr();
        let range = w
            .try_append_prefixed(prefixes.reserve(4096).unwrap(), lease)
            .unwrap();
        assert_eq!(range.len, 8192);
        let p = w.slots[0].as_mut().unwrap();
        assert_eq!(p.lease.primary_bytes().as_ptr(), ptr);
        p.request = Some(1);
        p.issued = 4096;
        w.retire_completion(1, 512);
        assert_eq!(prefixes.available(), 0);
        assert!(region.acquire(4096..8192).is_err());
        let p = w.slots[0].as_mut().unwrap();
        p.request = Some(2);
        p.issued = 3584;
        w.retire_completion(2, 3584);
        assert_eq!(w.slots[0].as_ref().unwrap().done, 4096);
        assert_eq!(prefixes.available(), 0);
        assert!(region.acquire(4096..8192).is_err());
        let p = w.slots[0].as_mut().unwrap();
        assert_eq!(p.lease.primary_bytes().as_ptr(), ptr);
        p.request = Some(3);
        p.issued = 4096;
        w.retire_completion(3, -libc::EIO);
        assert_eq!(
            w.progress(false).unwrap_err().kind(),
            io::Error::from_raw_os_error(libc::EIO).kind()
        );
        assert_eq!(w.outstanding(), 0);
        assert_eq!(prefixes.available(), 1);
        assert!(region.acquire(4096..8192).is_ok());
        drop(w);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn prefixed_registration_rejection_and_cancellation_keep_leases_safe() {
        use crate::mapped::MmapRegion;
        use std::os::unix::fs::OpenOptionsExt;
        let path = std::env::temp_dir().join(format!("prefixed-cancel-{}", std::process::id()));
        let file = File::options()
            .create_new(true)
            .write(true)
            .custom_flags(libc::O_DIRECT)
            .open(&path)
            .unwrap();
        let mut w = Writer::direct_with_config(
            file,
            BufferPool::new(1, 4096, 4096).unwrap(),
            4096,
            WriterConfig::default(),
        )
        .unwrap();
        let prefixes = BufferPool::new(1, 4096, 4096).unwrap();
        let region = MmapRegion::new(
            memmap2::MmapOptions::new().len(16384).map_anon().unwrap(),
            Default::default(),
        );
        let lease = region.acquire(4096..12288).unwrap().seal(8192).unwrap();
        let reader = lease.clone();
        w.fail_registration_after = Some(1);
        let rejected = w
            .try_append_prefixed(prefixes.reserve(4096).unwrap(), lease)
            .unwrap_err();
        assert_eq!(w.accepted_bytes(), 0);
        assert_eq!(prefixes.available(), 1);
        assert_eq!(
            rejected.value.as_slice().as_ptr(),
            reader.as_slice().as_ptr()
        );
        w.fail_registration_after = None;
        w.try_append_prefixed(prefixes.reserve(4096).unwrap(), rejected.value)
            .unwrap();
        w.fail_submit = true;
        assert!(w.progress(false).is_err());
        assert_eq!(prefixes.available(), 0);
        assert!(region.acquire(4096..12288).is_err());
        w.fail_submit = false;
        let _ = w.cancel_and_drain();
        assert_eq!(prefixes.available(), 1);
        assert!(region.acquire(4096..12288).is_err());
        drop(reader);
        assert!(region.acquire(4096..12288).is_ok());
        drop(w);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn mapped_segment_offsets_rollback_and_cancellation() {
        use crate::mapped::MmapRegion;
        use memmap2::MmapOptions;
        use std::os::unix::fs::OpenOptionsExt;
        let region = MmapRegion::new(
            MmapOptions::new().len(16384).map_anon().unwrap(),
            Default::default(),
        );
        let path = std::env::temp_dir().join(format!("mapped-unit-{}", std::process::id()));
        let file = File::options()
            .custom_flags(libc::O_DIRECT)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut w = Writer::direct_with_config(
            file,
            BufferPool::new(1, 4096, 4096).unwrap(),
            4096,
            WriterConfig {
                max_write_bytes: 8192,
                ..Default::default()
            },
        )
        .unwrap();
        w.fail_registration_after = Some(1);
        let rejected = w
            .try_append_mapped(region.acquire(4096..12288).unwrap().seal(8192).unwrap())
            .unwrap_err();
        assert_eq!(w.accepted_bytes(), 0);
        assert_eq!(w.registered_bytes(), 4096);
        assert!(region.acquire(4096..12288).is_err());
        w.fail_registration_after = None;
        w.try_append_mapped(rejected.value).unwrap();
        w.flush_written().unwrap();
        assert_eq!(w.snapshot().counters.queued_operations, 2); // crosses 4-KiB test registration segments
        assert_eq!(std::fs::read(&path).unwrap(), vec![0; 8192]);
        let lease = region.acquire(4096..12288).unwrap().seal(8192).unwrap();
        let reader = lease.clone();
        w.try_append_mapped(lease).unwrap();
        w.fail_submit = true;
        assert!(w.progress(false).is_err());
        assert!(region.acquire(4096..12288).is_err());
        w.fail_submit = false;
        let _ = w.cancel_and_drain();
        assert!(region.acquire(4096..12288).is_err());
        drop(reader);
        assert!(region.acquire(4096..12288).is_ok());
        drop(w);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn submission_failure_retains_ownership() {
        let path = std::env::temp_dir().join(format!("doramcap-submit-{}", std::process::id()));
        let file = File::options()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        let pool = BufferPool::new(1, 512, 512).unwrap();
        let mut writer = Writer::new(file, 1).unwrap();
        writer.append(pool.reserve(512).unwrap()).unwrap();
        writer.fail_submit = true;
        assert!(writer.progress(false).is_err());
        assert_eq!(pool.available(), 0);
        assert_eq!(writer.outstanding(), 1);
        writer.fail_submit = false;
        writer.close().unwrap();
        assert_eq!(pool.available(), 1);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn finalization_failures_are_observable() {
        for truncate in [true, false] {
            let path = std::env::temp_dir().join(format!(
                "doramcap-finalize-{}-{truncate}",
                std::process::id()
            ));
            let file = File::options()
                .write(true)
                .create_new(true)
                .open(&path)
                .unwrap();
            let pool = BufferPool::new(1, 512, 512).unwrap();
            let mut writer = Writer::new(file, 1).unwrap();
            writer.append(pool.reserve(512).unwrap()).unwrap();
            writer.fail_truncate = truncate;
            writer.fail_sync = !truncate;
            assert!(writer.close_at(1).is_err());
            assert_eq!(pool.available(), 1);
            assert_eq!(
                std::fs::metadata(&path).unwrap().len(),
                if truncate { 512 } else { 1 }
            );
            std::fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn registration_failure_preserves_existing_requests() {
        use std::os::unix::fs::OpenOptionsExt;
        let path =
            std::env::temp_dir().join(format!("doramcap-registration-fail-{}", std::process::id()));
        let file = File::options()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_DIRECT)
            .open(&path)
            .unwrap();
        let pool = BufferPool::new(1, 4096, 4096).unwrap();
        let mut writer = Writer::direct(file, pool.clone(), 4096).unwrap();
        writer.append(pool.reserve(4096).unwrap()).unwrap();
        let lease = pool.reserve(8192).unwrap();
        writer.fail_registration_after = Some(1);
        let failed = writer.try_append(lease).unwrap_err();
        assert_eq!(writer.registered_ranges(), 1);
        assert_eq!(writer.accepted_bytes(), 4096);
        assert_eq!(writer.outstanding(), 1);
        writer.fail_registration_after = None;
        writer.try_append(failed.lease).unwrap();
        writer.close().unwrap();
        assert_eq!(pool.available(), 2);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn cancellation_of_queued_and_unscheduled_split_writes() {
        let path =
            std::env::temp_dir().join(format!("doramcap-cancel-split-{}", std::process::id()));
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let pool = BufferPool::new(2, 8192, 4096).unwrap();
        let mut writer = Writer::with_config(
            file,
            WriterConfig {
                queue_depth: 1,
                max_write_bytes: 4096,
                ..WriterConfig::default()
            },
        )
        .unwrap();
        writer.append(pool.reserve(8192).unwrap()).unwrap();
        writer.append(pool.reserve(8192).unwrap()).unwrap();
        writer.schedule().unwrap(); // SQ contains a real write; the second is still pending.
        assert!(writer.slots.iter().flatten().any(|p| p.request.is_some()));
        let _ = writer.cancel_and_drain();
        assert_eq!(writer.outstanding(), 0);
        assert_eq!(writer.cancellations, 0);
        assert_eq!(pool.available(), 2);
        drop(writer);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn slab_registration_rollback_and_allocation_guard_lifetime() {
        use std::os::unix::fs::OpenOptionsExt;
        let path =
            std::env::temp_dir().join(format!("doramcap-slab-rollback-{}", std::process::id()));
        let file = File::options()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_DIRECT)
            .open(&path)
            .unwrap();
        let managed = BufferPool::new(1, 4096, 4096).unwrap();
        let collector =
            crate::observability::Collector::new(64, crate::observability::Mode::Inspect).unwrap();
        let slabs = SlabPool::with_observer(1, 8192, 4096, collector.observer()).unwrap();
        let allocation = Arc::downgrade(&slabs.allocations()[0]);
        let mut writer = Writer::direct(file, managed.clone(), 4096).unwrap();
        writer.append(managed.reserve(4096).unwrap()).unwrap();
        let slab = slabs.acquire().unwrap().seal(8192).unwrap();
        writer.fail_registration_after = Some(1);
        let rejected = writer.try_append_slab(slab).unwrap_err();
        assert_eq!(writer.registered_ranges(), 1);
        assert_eq!(writer.registration_cache.len(), 1);
        assert_eq!(writer.accepted_bytes(), 4096);
        assert_eq!(slabs.available(), 0);
        writer.fail_registration_after = None;
        writer.try_append_slab(rejected.value).unwrap();
        writer.flush_written().unwrap();
        assert_eq!(slabs.available(), 1);
        drop(slabs);
        assert!(allocation.upgrade().is_some()); // kernel registration still owns allocation
        writer.trim(&managed).unwrap();
        assert!(allocation.upgrade().is_none());
        assert!(!collector.read_after(0, 64).events.is_empty());
        writer.close().unwrap();
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn slab_cancellation_and_submit_failure_hold_leases() {
        for fail in [false, true] {
            let path = std::env::temp_dir().join(format!(
                "doramcap-slab-cancel-{}-{fail}",
                std::process::id()
            ));
            let file = File::options()
                .write(true)
                .create_new(true)
                .open(&path)
                .unwrap();
            let slabs = SlabPool::new(2, 8192, 4096).unwrap();
            let mut writer = Writer::with_config(
                file,
                WriterConfig {
                    queue_depth: 1,
                    max_write_bytes: 4096,
                    ..WriterConfig::default()
                },
            )
            .unwrap();
            let first = slabs.acquire().unwrap().seal(8192).unwrap();
            let reader = first.clone();
            writer.try_append_slab(first).unwrap();
            writer
                .try_append_slab(slabs.acquire().unwrap().seal(8192).unwrap())
                .unwrap();
            writer.schedule().unwrap();
            assert!(writer.slots.iter().flatten().any(|p| p.request.is_some()));
            assert_eq!(
                writer.slots[0]
                    .as_ref()
                    .unwrap()
                    .lease
                    .primary_bytes()
                    .as_ptr(),
                reader.as_slice().as_ptr()
            );
            if fail {
                writer.fail_submit = true;
                assert!(writer.progress(false).is_err());
                assert_eq!(slabs.available(), 0);
                writer.fail_submit = false;
            }
            let _ = writer.cancel_and_drain();
            assert_eq!(writer.outstanding(), 0);
            assert_eq!(writer.cancellations, 0);
            assert_eq!(slabs.available(), 1);
            drop(reader);
            assert_eq!(slabs.available(), 2);
            drop(writer);
            std::fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn slab_short_completion_retains_source_until_terminal_retirement() {
        let slabs = SlabPool::new(2, 4096, 4096).unwrap();
        let mut pending: Vec<_> = (0..2)
            .map(|_| Pending {
                timeline: Timeline::default(),
                observed: Default::default(),
                lease: WriteSource::Slab(slabs.acquire().unwrap().seal(4096).unwrap()),
                segments: None,
                range: FileRange {
                    offset: 0,
                    len: 4096,
                },
                done: 0,
                request: None,
                issued: 4096,
                #[cfg(feature = "diagnostics")]
                submitted_at: None,
            })
            .collect();
        // Reverse retirement order and an injected aligned short completion.
        assert!(!pending[1].advance(512).unwrap());
        assert_eq!(slabs.available(), 0);
        pending[1].issued = 3584;
        assert!(pending[1].advance(3584).unwrap());
        drop(pending.pop());
        assert_eq!(slabs.available(), 1);
        assert!(pending[0].advance(-libc::EIO).is_err());
        drop(pending);
        assert_eq!(slabs.available(), 2);
    }
    #[test]
    fn observation_reports_registration_submit_and_sync_failures() {
        use crate::observability::{Collector, Mode};
        use std::os::unix::fs::OpenOptionsExt;
        let path = std::env::temp_dir().join(format!(
            "doramcap-observation-failures-{}",
            std::process::id()
        ));
        let file = File::options()
            .create_new(true)
            .write(true)
            .custom_flags(libc::O_DIRECT)
            .open(&path)
            .unwrap();
        let pool = BufferPool::new(1, 4096, 4096).unwrap();
        let collector = Collector::new(64, Mode::Inspect).unwrap();
        let mut writer = Writer::direct(file, pool.clone(), 4096).unwrap();
        writer.set_observer(collector.observer());
        writer.fail_registration_after = Some(1);
        let rejected = writer.try_append(pool.reserve(8192).unwrap()).unwrap_err();
        assert_eq!(writer.snapshot().counters.registration_failures, 1);
        assert_eq!(writer.snapshot().counters.accepted, 0);
        writer.fail_registration_after = None;
        writer.try_append(rejected.lease).unwrap();
        let id = writer.active_writes().next().unwrap().ids.write;
        writer.fail_submit = true;
        assert!(writer.progress(false).is_err());
        assert_eq!(writer.active_writes().next().unwrap().ids.write, id);
        assert_eq!(writer.snapshot().counters.submit_failures, 1);
        writer.fail_submit = false;
        writer.flush_written().unwrap();
        writer.fail_sync = true;
        assert!(writer.sync().is_err());
        assert_eq!(writer.snapshot().synced_bytes, 0);
        assert_eq!(writer.snapshot().counters.sync_failures, 1);
        writer.fail_sync = false;
        writer.sync().unwrap();
        assert_eq!(writer.snapshot().synced_bytes, 8192);
        writer.finalize_at(1).unwrap();
        assert_eq!(writer.snapshot().synced_bytes, 1);
        let events = collector.read_after(0, 64).events;
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::RegistrationFailed(_)))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::SubmitFailed(_)))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::SyncFailed(_)))
        );
        assert!(
            events
                .iter()
                .any(|e| e.kind == EventKind::Written && e.data.ids.write == id)
        );
        drop(writer);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn observation_short_and_failed_completions_keep_write_identity() {
        use crate::observability::{Collector, Mode};
        let path =
            std::env::temp_dir().join(format!("doramcap-short-observation-{}", std::process::id()));
        let file = File::options()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        let pool = BufferPool::new(1, 8192, 4096).unwrap();
        let c = Collector::new(32, Mode::Inspect).unwrap();
        let mut writer = Writer::new(file, 2).unwrap();
        writer.set_observer(c.observer());
        writer.try_append(pool.reserve(8192).unwrap()).unwrap();
        let id = writer.active_writes().next().unwrap().ids.write;
        // Inject completion-boundary inputs without submitting kernel operations.
        let p = writer.slots[0].as_mut().unwrap();
        p.request = Some(1);
        p.issued = 4096;
        writer.retire_completion(1, 512);
        let active = writer.active_writes().next().unwrap();
        assert_eq!(
            (active.ids.write, active.completed_bytes, active.retries),
            (id, 512, 1)
        );
        assert_eq!(writer.snapshot().counters.short_retries, 1);
        assert_eq!(pool.available(), 0);
        let p = writer.slots[0].as_mut().unwrap();
        p.request = Some(2);
        p.issued = 4096;
        writer.retire_completion(2, -libc::EIO);
        assert_eq!(writer.snapshot().counters.failed, 1);
        assert_eq!(writer.outstanding(), 0);
        assert_eq!(pool.available(), 1);
        let events = c.read_after(0, 32).events;
        assert!(
            events
                .iter()
                .any(|e| e.kind == EventKind::Retry && e.data.ids.write == id)
        );
        let completions: Vec<_> = events
            .iter()
            .filter(|e| e.kind == EventKind::CqeObserved)
            .collect();
        assert_eq!(
            completions
                .iter()
                .map(|e| e.data.ids.operation)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Failed(_)) && e.data.ids.write == id)
        );
        drop(writer);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn completion_contract() {
        let pool = BufferPool::new(2, 16, 512).unwrap();
        let mut p = Pending {
            timeline: Timeline::default(),
            observed: Default::default(),
            lease: WriteSource::Managed(pool.reserve(16).unwrap()),
            segments: None,
            range: FileRange { offset: 0, len: 16 },
            done: 0,
            request: None,
            issued: 16,
            #[cfg(feature = "diagnostics")]
            submitted_at: None,
        };
        assert!(!p.advance(7).unwrap());
        assert_eq!(pool.available(), 1);
        assert!(p.advance(9).unwrap());
        drop(p);
        assert_eq!(pool.available(), 2);
        for result in [0, -libc::EIO, -libc::ECANCELED, 17] {
            let mut p = Pending {
                timeline: Timeline::default(),
                observed: Default::default(),
                lease: WriteSource::Managed(pool.reserve(16).unwrap()),
                segments: None,
                range: FileRange { offset: 0, len: 16 },
                done: 0,
                request: None,
                issued: 16,
                #[cfg(feature = "diagnostics")]
                submitted_at: None,
            };
            assert!(p.advance(result).is_err());
            assert_eq!(p.done, 0);
        }
    }
}
