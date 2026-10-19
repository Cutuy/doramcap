//! Same-process aligned slabs with exclusive filling and shared immutable reads.
//!
//! Registration retains an allocation independently of the lease that permits reuse.
//! Sealing allocates one Arc control block; this API makes no allocation-free claim.
use crate::observability::{self, EventData, EventKind, Ids, Observer};
use std::{
    alloc::{Layout, alloc_zeroed, dealloc},
    io,
    ptr::NonNull,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering},
    },
    time::Instant,
};

/// An admission/validation error that preserves the submitted value.
#[derive(Debug)]
pub struct Rejected<T> {
    pub error: io::Error,
    pub value: T,
}
impl<T> std::fmt::Display for Rejected<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl<T: std::fmt::Debug> std::error::Error for Rejected<T> {}

#[derive(Debug)]
pub(crate) struct SlabAllocation {
    pub(crate) id: u64,
    ptr: NonNull<u8>,
    layout: Layout,
    use_id: AtomicU64,
    phase: AtomicU8,
    readers: AtomicUsize,
    io_leases: AtomicUsize,
    had_io: AtomicBool,
    since_ns: AtomicU64,
}
// SAFETY: allocation handles keep raw storage alive but expose no public byte access.
// The unique WritableSlab or shared SealedSlab mediates all byte access. The pool
// mutex publishes the final reader's release before handing out a new writer.
unsafe impl Send for SlabAllocation {}
unsafe impl Sync for SlabAllocation {}
impl SlabAllocation {
    fn new(layout: Layout) -> io::Result<Arc<Self>> {
        let id = observability::next_id();
        let ptr =
            NonNull::new(unsafe { alloc_zeroed(layout) }).ok_or(io::ErrorKind::OutOfMemory)?;
        Ok(Arc::new(Self {
            id,
            ptr,
            layout,
            use_id: AtomicU64::new(0),
            phase: AtomicU8::new(0),
            readers: AtomicUsize::new(0),
            io_leases: AtomicUsize::new(0),
            had_io: AtomicBool::new(false),
            since_ns: AtomicU64::new(0),
        }))
    }
    pub(crate) fn ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }
    pub(crate) fn capacity(&self) -> usize {
        self.layout.size()
    }
}
impl Drop for SlabAllocation {
    fn drop(&mut self) {
        // SAFETY: final owner, using the original allocation layout.
        unsafe { dealloc(self.ptr.as_ptr(), self.layout) }
    }
}
#[derive(Debug)]
struct PoolInner {
    id: u64,
    observer: Observer,
    epoch: Instant,
    allocations: Vec<Arc<SlabAllocation>>,
    free: Mutex<Vec<Arc<SlabAllocation>>>,
}
/// Bounded producer-owned pool. Clones share the same free list.
#[derive(Clone, Debug)]
pub struct SlabPool {
    inner: Arc<PoolInner>,
}
impl SlabPool {
    pub fn new(count: usize, size: usize, alignment: usize) -> io::Result<Self> {
        Self::with_observer(count, size, alignment, Observer::default())
    }
    pub fn with_observer(
        count: usize,
        size: usize,
        alignment: usize,
        observer: Observer,
    ) -> io::Result<Self> {
        if count == 0
            || size == 0
            || !alignment.is_power_of_two()
            || !size.is_multiple_of(alignment)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let layout =
            Layout::from_size_align(size, alignment).map_err(|_| io::ErrorKind::InvalidInput)?;
        let mut allocations = Vec::new();
        allocations
            .try_reserve_exact(count)
            .map_err(|_| io::ErrorKind::OutOfMemory)?;
        for _ in 0..count {
            allocations.push(SlabAllocation::new(layout)?);
        }
        Ok(Self {
            inner: Arc::new(PoolInner {
                id: observability::next_id(),
                observer,
                epoch: Instant::now(),
                free: Mutex::new(allocations.clone()),
                allocations,
            }),
        })
    }
    pub fn available(&self) -> usize {
        self.inner.free.lock().unwrap().len()
    }
    pub fn capacity(&self) -> usize {
        self.inner.allocations.len()
    }
    pub fn acquire(&self) -> io::Result<WritableSlab> {
        let mut free = self.inner.free.lock().unwrap();
        let allocation = free.pop().ok_or(io::ErrorKind::WouldBlock)?;
        let use_id = observability::next_id();
        allocation.use_id.store(use_id, Ordering::Relaxed);
        allocation.phase.store(1, Ordering::Relaxed);
        allocation.had_io.store(false, Ordering::Relaxed);
        allocation.since_ns.store(
            self.inner
                .observer
                .now()
                .map_or(0, |t| nanos(t.duration_since(self.inner.epoch)).max(1)),
            Ordering::Relaxed,
        );
        let inner = SlabLeaseInner {
            allocation,
            pool: self.inner.clone(),
            use_id,
        };
        inner.record(EventKind::SlabAcquired);
        Ok(WritableSlab { inner })
    }
    /// Bounded metadata copy. Cross-thread counters are sampled, not transactional.
    pub fn snapshot(&self) -> PoolSnapshot {
        let sampled = self.inner.epoch.elapsed();
        let slabs: Vec<_> = self
            .inner
            .allocations
            .iter()
            .map(|a| {
                let since = a.since_ns.load(Ordering::Relaxed);
                SlabSnapshot {
                    region_id: a.id,
                    slab_use_id: a.use_id.load(Ordering::Relaxed),
                    phase: match a.phase.load(Ordering::Relaxed) {
                        0 => SlabPhase::Free,
                        1 => SlabPhase::Writable,
                        _ => SlabPhase::Sealed,
                    },
                    readers: a.readers.load(Ordering::Relaxed),
                    io_leases: a.io_leases.load(Ordering::Relaxed),
                    had_io: a.had_io.load(Ordering::Relaxed),
                    age_us: (since != 0).then(|| nanos(sampled).saturating_sub(since) / 1000),
                }
            })
            .collect();
        PoolSnapshot {
            pool_id: self.inner.id,
            sampled_ns: nanos(sampled),
            available: self.available(),
            slabs,
        }
    }
    pub(crate) fn allocations(&self) -> &[Arc<SlabAllocation>] {
        &self.inner.allocations
    }
}
#[derive(Debug)]
struct SlabLeaseInner {
    use_id: u64,
    allocation: Arc<SlabAllocation>,
    pool: Arc<PoolInner>,
}
impl SlabLeaseInner {
    fn ids(&self) -> Ids {
        Ids {
            region: self.allocation.id,
            slab_use: self.use_id,
            pool: self.pool.id,
            ..Default::default()
        }
    }
    fn record(&self, kind: EventKind) {
        self.pool.observer.record(kind, || EventData {
            ids: self.ids(),
            len: self.allocation.capacity() as u64,
            ..Default::default()
        });
    }
}
fn nanos(duration: std::time::Duration) -> u64 {
    duration.as_nanos().min(u64::MAX as u128) as u64
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlabPhase {
    Free,
    Writable,
    Sealed,
}
#[derive(Clone, Copy, Debug)]
pub struct SlabSnapshot {
    pub region_id: u64,
    pub slab_use_id: u64,
    pub phase: SlabPhase,
    pub readers: usize,
    pub io_leases: usize,
    pub had_io: bool,
    pub age_us: Option<u64>,
}
impl SlabSnapshot {
    pub fn retained_after_io(&self) -> bool {
        self.phase == SlabPhase::Sealed && self.had_io && self.io_leases == 0 && self.readers > 0
    }
}
#[derive(Debug)]
pub struct PoolSnapshot {
    pub pool_id: u64,
    pub sampled_ns: u64,
    pub available: usize,
    pub slabs: Vec<SlabSnapshot>,
}
impl Drop for SlabLeaseInner {
    fn drop(&mut self) {
        // At most capacity() exclusive tokens exist; returning a token never grows Vec.
        let mut free = self.pool.free.lock().unwrap();
        self.allocation.phase.store(0, Ordering::Relaxed);
        self.allocation.since_ns.store(0, Ordering::Relaxed);
        free.push(self.allocation.clone());
        self.record(EventKind::SlabRecycled);
    }
}
/// Exclusive, movable write permission. Sealing consumes this handle.
///
/// ```compile_fail
/// use storage_persistence::slabs::SlabPool;
/// let pool = SlabPool::new(1, 4096, 4096).unwrap();
/// let mut slab = pool.acquire().unwrap();
/// let bytes = slab.bytes_mut();
/// let sealed = slab.seal(4096).unwrap();
/// bytes[0] = 1; // cannot seal while a mutable borrow is still in use
/// ```
#[derive(Debug)]
pub struct WritableSlab {
    inner: SlabLeaseInner,
}
impl WritableSlab {
    pub fn capacity(&self) -> usize {
        self.inner.allocation.capacity()
    }
    /// All bytes are initialized. Reused slabs retain their previous contents.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: unique lease; no sealed readers exist until this handle is consumed.
        unsafe { std::slice::from_raw_parts_mut(self.inner.allocation.ptr(), self.capacity()) }
    }
    pub fn seal(self, len: usize) -> Result<SealedSlab, Rejected<Self>> {
        if len == 0 || len > self.capacity() {
            return Err(Rejected {
                error: io::ErrorKind::InvalidInput.into(),
                value: self,
            });
        }
        self.inner.allocation.readers.store(1, Ordering::Relaxed);
        self.inner.allocation.phase.store(2, Ordering::Relaxed);
        self.inner.allocation.since_ns.store(
            self.inner
                .pool
                .observer
                .now()
                .map_or(0, |t| nanos(t.duration_since(self.inner.pool.epoch)).max(1)),
            Ordering::Relaxed,
        );
        self.inner.record(EventKind::SlabSealed);
        Ok(SealedSlab {
            io_lease: false,
            inner: Arc::new(self.inner),
            len,
        })
    }
}
/// Shared immutable bytes. The last reader and I/O lease return the entire slab.
#[derive(Debug)]
pub struct SealedSlab {
    io_lease: bool,
    inner: Arc<SlabLeaseInner>,
    len: usize,
}
impl Clone for SealedSlab {
    fn clone(&self) -> Self {
        self.inner
            .allocation
            .readers
            .fetch_add(1, Ordering::Relaxed);
        self.inner.record(EventKind::ReaderCloned);
        Self {
            inner: self.inner.clone(),
            len: self.len,
            io_lease: false,
        }
    }
}
impl Drop for SealedSlab {
    fn drop(&mut self) {
        if self.io_lease {
            self.inner
                .allocation
                .io_leases
                .fetch_sub(1, Ordering::Relaxed);
            self.inner.record(EventKind::IoLeaseReleased);
        } else {
            self.inner
                .allocation
                .readers
                .fetch_sub(1, Ordering::Relaxed);
            self.inner.record(EventKind::ReaderReleased);
        }
    }
}
impl SealedSlab {
    pub(crate) fn observation_ids(&self) -> Ids {
        self.inner.ids()
    }
    pub(crate) fn into_io(mut self) -> Self {
        self.inner
            .allocation
            .readers
            .fetch_sub(1, Ordering::Relaxed);
        self.inner
            .allocation
            .io_leases
            .fetch_add(1, Ordering::Relaxed);
        self.inner.allocation.had_io.store(true, Ordering::Relaxed);
        self.io_lease = true;
        self.inner.record(EventKind::IoLeaseAcquired);
        self
    }
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: initialized, bounded bytes; all handles are immutable until final drop.
        unsafe { std::slice::from_raw_parts(self.inner.allocation.ptr(), self.len) }
    }
    pub fn capacity(&self) -> usize {
        self.inner.allocation.capacity()
    }
    pub(crate) fn allocation(&self) -> &Arc<SlabAllocation> {
        &self.inner.allocation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seal_rejects_without_losing_token_and_readers_cross_threads() {
        let pool = SlabPool::new(1, 4096, 4096).unwrap();
        let mut slab = pool.acquire().unwrap().seal(4097).unwrap_err().value;
        slab.bytes_mut().fill(42);
        let sealed = slab.seal(4096).unwrap();
        let reader = sealed.clone();
        let address = sealed.as_slice().as_ptr() as usize;
        assert_eq!(
            pool.acquire().unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        drop(sealed);
        std::thread::spawn(move || assert!(reader.as_slice().iter().all(|&b| b == 42)))
            .join()
            .unwrap();
        assert_eq!(pool.available(), 1);
        assert_eq!(
            pool.acquire().unwrap().bytes_mut().as_ptr() as usize,
            address
        );
    }
}
