//! One owner-driven io_uring with bounded write slots and retained buffer leases.
use crate::{
    file_offsets::{FileRange, OffsetAllocator},
    managed_buffers::{BufferLease, BufferPool},
};
use io_uring::{IoUring, opcode, types};
use std::{fs::File, io, os::fd::AsRawFd};

struct Pending {
    lease: BufferLease,
    range: FileRange,
    done: usize,
}
impl Pending {
    fn advance(&mut self, result: i32) -> io::Result<bool> {
        if result < 0 {
            return Err(io::Error::from_raw_os_error(-result));
        }
        let remaining = self.lease.as_slice().len() - self.done;
        if result == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        if result as usize > remaining {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.done += result as usize;
        Ok(self.done == self.lease.as_slice().len())
    }
}
/// Buffered-file baseline. `flush_written` retires writes; `sync` additionally
/// requests filesystem durability. Drop drains writes, but cannot report errors.
/// Call `close` to observe final errors. SQPOLL is never enabled.
pub struct Writer {
    ring: IoUring,
    file: File,
    offsets: OffsetAllocator,
    slots: Vec<Option<Pending>>,
    outstanding: usize,
    cancellations: usize,
    error: Option<io::ErrorKind>,
    closed: bool,
    written_bytes: u64,
    registered: Option<BufferPool>,
    alignment: usize,
    #[cfg(test)]
    fail_submit: bool,
    #[cfg(test)]
    fail_sync: bool,
    #[cfg(test)]
    fail_truncate: bool,
}
impl Writer {
    /// The caller supplies a writable, seekable regular file without O_APPEND.
    /// File ranges start at zero. Existing contents should be truncated by caller.
    pub fn new(file: File, capacity: u32) -> io::Result<Self> {
        if capacity == 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // SAFETY: F_GETFL only reads flags of the owned, live descriptor.
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        if flags & libc::O_APPEND != 0 || !file.metadata()?.is_file() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let ring = IoUring::new(capacity)?;
        let slots = (0..capacity).map(|_| None).collect();
        Ok(Self {
            ring,
            file,
            offsets: OffsetAllocator::new(0, 1).unwrap(),
            slots,
            outstanding: 0,
            cancellations: 0,
            error: None,
            closed: false,
            written_bytes: 0,
            registered: None,
            alignment: 1,
            #[cfg(test)]
            fail_submit: false,
            #[cfg(test)]
            fail_sync: false,
            #[cfg(test)]
            fail_truncate: false,
        })
    }
    /// Register an idle pool and require O_DIRECT with an explicit alignment.
    pub fn direct(file: File, pool: BufferPool, alignment: usize) -> io::Result<Self> {
        if !matches!(alignment, 512 | 4096) || pool.capacity() > u16::MAX as usize {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // SAFETY: reads descriptor flags only.
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) } & libc::O_DIRECT == 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut this = Self::new(file, pool.capacity() as u32)?;
        let buffers = pool
            .registration()
            .map_err(|_| io::ErrorKind::InvalidInput)?;
        if buffers.iter().any(|b| {
            !(b.iov_base as usize).is_multiple_of(alignment) || !b.iov_len.is_multiple_of(alignment)
        }) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // SAFETY: this retains the pool through all I/O and ring destruction;
        // leases prevent mutation while requests reference the registered pages.
        unsafe { this.ring.submitter().register_buffers(&buffers) }?;
        this.registered = Some(pool);
        this.alignment = alignment;
        this.offsets = OffsetAllocator::new(0, alignment as u64).unwrap();
        Ok(this)
    }
    fn entry(
        fd: i32,
        lease: &BufferLease,
        done: usize,
        offset: u64,
        fixed: bool,
    ) -> io_uring::squeue::Entry {
        let bytes = &lease.as_slice()[done..];
        if fixed {
            opcode::WriteFixed::new(
                types::Fd(fd),
                bytes.as_ptr(),
                bytes.len() as u32,
                lease.index() as u16,
            )
            .offset(offset)
            .build()
        } else {
            opcode::Write::new(types::Fd(fd), bytes.as_ptr(), bytes.len() as u32)
                .offset(offset)
                .build()
        }
    }
    pub fn outstanding(&self) -> usize {
        self.outstanding
    }
    pub fn written_bytes(&self) -> u64 {
        self.written_bytes
    }
    pub fn accepted_bytes(&self) -> u64 {
        self.offsets.next_offset()
    }
    pub fn sqpoll_enabled(&self) -> bool {
        self.ring.params().is_setup_sqpoll()
    }
    /// Transfers the lease. Rejected writes release the lease without reserving
    /// file space. Accepted leases stay immutable until completion retirement.
    pub fn append(&mut self, lease: BufferLease) -> io::Result<FileRange> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if let Some(error) = self.error {
            return Err(error.into());
        }
        if self
            .registered
            .as_ref()
            .is_some_and(|pool| !pool.owns(&lease))
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let len = lease.as_slice().len();
        if len == 0 || len > u32::MAX as usize {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let index = self
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(io::ErrorKind::WouldBlock)?;
        let reservation = self
            .offsets
            .reserve(len as u64)
            .map_err(|_| io::ErrorKind::InvalidInput)?;
        let range = reservation.range();
        let entry = Self::entry(
            self.file.as_raw_fd(),
            &lease,
            0,
            range.offset,
            self.registered.is_some(),
        )
        .user_data(index as u64);
        // SAFETY: lease is moved into an occupied slot before any submission.
        // The kernel only reads it; the slot retains its storage through the CQE.
        unsafe { self.ring.submission().push(&entry) }.map_err(|_| io::ErrorKind::WouldBlock)?;
        self.slots[index] = Some(Pending {
            lease,
            range,
            done: 0,
        });
        reservation.commit();
        self.outstanding += 1;
        Ok(range)
    }
    fn retry(&mut self, index: usize) -> io::Result<()> {
        let p = self.slots[index].as_ref().unwrap();
        if !p.done.is_multiple_of(self.alignment) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let entry = Self::entry(
            self.file.as_raw_fd(),
            &p.lease,
            p.done,
            p.range.offset + p.done as u64,
            self.registered.is_some(),
        )
        .user_data(index as u64);
        // SAFETY: the original request retired; the same slot owns the remainder.
        unsafe { self.ring.submission().push(&entry) }.map_err(|_| io::ErrorKind::WouldBlock)?;
        Ok(())
    }
    /// Submit queued operations and process available completions. `wait` waits
    /// for at least one completion when requests are outstanding.
    pub fn progress(&mut self, wait: bool) -> io::Result<()> {
        #[cfg(test)]
        if self.fail_submit {
            return Err(io::ErrorKind::Other.into());
        }
        loop {
            match self.ring.submit_and_wait(usize::from(
                wait && (self.outstanding + self.cancellations > 0),
            )) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                result => {
                    result?;
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
            if id >= self.slots.len() as u64 {
                self.cancellations -= 1;
                continue;
            }
            let index = id as usize;
            let p = self.slots[index]
                .as_mut()
                .expect("write completion must own a slot");
            match p.advance(result) {
                Ok(false) => {
                    if let Err(e) = self.retry(index) {
                        // Queue pressure after retirement: no kernel reference remains.
                        self.error.get_or_insert(e.kind());
                        self.slots[index] = None;
                        self.outstanding -= 1;
                    }
                }
                outcome => {
                    match outcome {
                        Ok(true) => self.written_bytes += p.range.len,
                        Err(e) => {
                            self.error.get_or_insert(e.kind());
                        }
                        _ => unreachable!(),
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
    /// Stops accepting writes, cancels outstanding requests and retires both
    /// original and cancellation CQEs. Completion races may still write data.
    pub fn cancel_and_drain(&mut self) -> io::Result<()> {
        self.closed = true;
        self.progress(false)?;
        for index in 0..self.slots.len() {
            if self.slots[index].is_none() {
                continue;
            }
            let entry = opcode::AsyncCancel::new(index as u64)
                .build()
                .user_data(u64::MAX - index as u64);
            // SAFETY: cancel has no user-memory payload; original slots remain owned.
            if unsafe { self.ring.submission().push(&entry) }.is_err() {
                self.ring.submit()?;
                unsafe { self.ring.submission().push(&entry) }
                    .map_err(|_| io::ErrorKind::WouldBlock)?;
            }
            self.cancellations += 1;
        }
        self.flush_written()
    }
    /// Drain physical writes, establish logical EOF, then sync data and metadata.
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
    /// Replace a finalized file while retaining this ring and its registrations.
    pub fn replace_file(&mut self, file: File) -> io::Result<()> {
        if !self.closed || self.outstanding + self.cancellations != 0 || self.error.is_some() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // SAFETY: only reads flags on a live owned descriptor.
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
        Ok(())
    }
    pub fn close(mut self) -> io::Result<()> {
        self.closed = true;
        self.sync()
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
    fn completion_contract() {
        let pool = BufferPool::new(2, 16, 512).unwrap();
        let mut p = Pending {
            lease: pool.reserve(16).unwrap(),
            range: FileRange { offset: 0, len: 16 },
            done: 0,
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
            };
            assert!(p.advance(result).is_err());
            assert_eq!(p.done, 0);
        }
    }
}
