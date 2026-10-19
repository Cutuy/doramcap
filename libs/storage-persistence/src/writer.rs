//! Owner-driven io_uring with independent pending and registration capacities.
use crate::{
    file_offsets::{FileRange, OffsetAllocator},
    managed_buffers::{Allocation, BufferLease, BufferPool},
};
use io_uring::{IoUring, opcode, types};
use std::{fs::File, io, os::fd::AsRawFd, rc::Rc};

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

struct Pending {
    lease: BufferLease,
    range: FileRange,
    done: usize,
    request: Option<u64>,
    issued: usize,
    #[cfg(feature = "diagnostics")]
    submitted_at: Option<std::time::Instant>,
}
impl Pending {
    fn advance(&mut self, result: i32) -> io::Result<bool> {
        if result < 0 {
            return Err(io::Error::from_raw_os_error(-result));
        }
        if result == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        if result as usize > self.issued
            || result as usize > self.lease.as_slice().len() - self.done
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.done += result as usize;
        Ok(self.done == self.lease.as_slice().len())
    }
}
struct Registered {
    allocation: Rc<Allocation>,
    start: usize,
    len: usize,
}

pub struct Writer {
    // Drop ring before registered allocation owners; persistent submit failures retain leases too.
    ring: IoUring,
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
            ring: IoUring::new(config.queue_depth)?,
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
    pub fn direct(file: File, pool: BufferPool, alignment: usize) -> io::Result<Self> {
        Self::direct_with_config(file, pool, alignment, WriterConfig::default())
    }
    pub fn direct_with_config(
        file: File,
        pool: BufferPool,
        alignment: usize,
        config: WriterConfig,
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
        let mut this = Self::with_config(file, config)?;
        this.alignment = alignment;
        this.max_write_bytes -= this.max_write_bytes % alignment;
        this.offsets = OffsetAllocator::new(0, alignment as u64).unwrap();
        this.registered = Some(pool.clone());
        for allocation in pool.allocations()? {
            this.register_allocation(&allocation)?;
        }
        Ok(this)
    }
    fn register_allocation(&mut self, allocation: &Rc<Allocation>) -> io::Result<()> {
        if !(allocation.ptr() as usize).is_multiple_of(self.alignment)
            || !allocation.capacity().is_multiple_of(self.alignment)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let needed = allocation.capacity().div_ceil(REGISTRATION_BYTES);
        let existing = self
            .registrations
            .iter()
            .flatten()
            .filter(|r| Rc::ptr_eq(&r.allocation, allocation))
            .count();
        if existing == needed {
            return Ok(());
        }
        if existing != 0 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.registrations.iter().filter(|r| r.is_none()).count() < needed {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        if !self.table_live {
            self.ring
                .submitter()
                .register_buffers_sparse(self.registrations.len() as u32)?;
            self.table_live = true;
        }
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
                        .is_some_and(|r| Rc::ptr_eq(&r.allocation, allocation))
                    {
                        let empty = libc::iovec {
                            iov_base: std::ptr::null_mut(),
                            iov_len: 0,
                        };
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
            self.registrations[index] = Some(Registered {
                allocation: allocation.clone(),
                start,
                len,
            });
        }
        Ok(())
    }
    /// Register a new managed allocation without submitting or waiting for I/O.
    pub fn prepare(&mut self, lease: &BufferLease) -> io::Result<()> {
        self.check_open()?;
        if let Some(pool) = &self.registered {
            if !pool.owns(lease) {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            self.register_allocation(lease.allocation())?;
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
    /// Retains caller ownership on any rejected admission. No file range is consumed.
    pub fn try_append(&mut self, lease: BufferLease) -> Result<FileRange, AppendError> {
        let validation = (|| {
            self.check_capacity(1)?;
            if lease.as_slice().is_empty() || !lease.as_slice().len().is_multiple_of(self.alignment)
            {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            self.prepare(&lease)
        })();
        if let Err(error) = validation {
            return Err(AppendError { error, lease });
        }
        let reservation = match self.offsets.reserve(lease.as_slice().len() as u64) {
            Ok(r) => r,
            Err(_) => {
                return Err(AppendError {
                    error: io::ErrorKind::InvalidInput.into(),
                    lease,
                });
            }
        };
        let range = reservation.range();
        let index = self.slots.iter().position(Option::is_none).unwrap();
        self.slots[index] = Some(Pending {
            lease,
            range,
            done: 0,
            request: None,
            issued: 0,
            #[cfg(feature = "diagnostics")]
            submitted_at: None,
        });
        reservation.commit();
        self.outstanding += 1;
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
            let bytes = &p.lease.as_slice()[p.done..];
            let mut len = bytes.len().min(self.max_write_bytes);
            let entry = if self.registered.is_some() {
                let (slot, r) = self
                    .registrations
                    .iter()
                    .enumerate()
                    .find_map(|(i, r)| {
                        r.as_ref()
                            .filter(|r| {
                                Rc::ptr_eq(&r.allocation, p.lease.allocation())
                                    && p.done >= r.start
                                    && p.done < r.start + r.len
                            })
                            .map(|r| (i, r))
                    })
                    .expect("prepared allocation remains registered");
                len = len.min(r.start + r.len - p.done);
                opcode::WriteFixed::new(
                    types::Fd(self.file.as_raw_fd()),
                    bytes.as_ptr(),
                    len as u32,
                    slot as u16,
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
            p.request = Some(self.next_request);
            p.issued = len;
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
        #[cfg(test)]
        if self.fail_submit {
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
                    let submitted = result?;
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
            if id & CANCEL_TAG != 0 {
                self.cancellations -= 1;
                continue;
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
            p.request = None;
            let outcome = p.advance(result);
            match outcome {
                Ok(false) if !self.cancelling && p.done.is_multiple_of(self.alignment) => {}
                other => {
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
        Ok(())
    }
    pub fn flush_written(&mut self) -> io::Result<()> {
        while self.outstanding + self.cancellations > 0 {
            self.progress(true)?;
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
            for r in &mut self.registrations {
                *r = None;
            }
        }
        pool.trim().map_err(Into::into)
    }
    fn sync_file(&self) -> io::Result<()> {
        #[cfg(test)]
        if self.fail_sync {
            return Err(io::Error::from_raw_os_error(libc::EIO));
        }
        self.file.sync_all()
    }
    pub fn sync(&mut self) -> io::Result<()> {
        self.flush_written()?;
        self.sync_file()
    }
    pub fn cancel_and_drain(&mut self) -> io::Result<()> {
        self.closed = true;
        self.cancelling = true;
        for index in 0..self.slots.len() {
            let Some(p) = &self.slots[index] else {
                continue;
            };
            let Some(id) = p.request else {
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
        self.sync_file()
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
    fn completion_contract() {
        let pool = BufferPool::new(2, 16, 512).unwrap();
        let mut p = Pending {
            lease: pool.reserve(16).unwrap(),
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
                lease: pool.reserve(16).unwrap(),
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
