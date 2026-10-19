//! Owner-thread free list of reusable, independently aligned allocations.
use std::{
    alloc::{Layout, alloc_zeroed, dealloc},
    cell::{Cell, RefCell},
    ptr::NonNull,
    rc::Rc,
};

pub const DEFAULT_BUFFER_SIZE: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolError {
    InvalidLayout,
    AllocationFailed,
    Busy,
}
impl From<PoolError> for std::io::Error {
    fn from(error: PoolError) -> Self {
        match error {
            PoolError::InvalidLayout => std::io::ErrorKind::InvalidInput,
            PoolError::AllocationFailed => std::io::ErrorKind::OutOfMemory,
            PoolError::Busy => std::io::ErrorKind::WouldBlock,
        }
        .into()
    }
}

/// Registration retains this owner independently of whether a lease is free.
/// Mutable access is exclusively mediated by BufferLease on the owner thread.
pub(crate) struct Allocation {
    ptr: NonNull<u8>,
    layout: Layout,
}
impl Allocation {
    fn new(layout: Layout) -> Result<Rc<Self>, PoolError> {
        let ptr =
            NonNull::new(unsafe { alloc_zeroed(layout) }).ok_or(PoolError::AllocationFailed)?;
        Ok(Rc::new(Self { ptr, layout }))
    }
    pub(crate) fn ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }
    pub(crate) fn capacity(&self) -> usize {
        self.layout.size()
    }
}
impl Drop for Allocation {
    fn drop(&mut self) {
        // SAFETY: last owner of this allocation, with its original layout.
        unsafe { dealloc(self.ptr.as_ptr(), self.layout) }
    }
}
struct Inner {
    free: RefCell<Vec<Rc<Allocation>>>,
    count: Cell<usize>,
    bytes: Cell<usize>,
    minimum: usize,
    alignment: usize,
    #[cfg(test)]
    fail_allocation: Cell<bool>,
}
/// Clones share a growable free list. Allocations never move or resize.
#[derive(Clone)]
pub struct BufferPool {
    inner: Rc<Inner>,
}
impl BufferPool {
    pub fn new(capacity: usize, size: usize, alignment: usize) -> Result<Self, PoolError> {
        if capacity == 0 || size == 0 {
            return Err(PoolError::InvalidLayout);
        }
        let layout =
            Layout::from_size_align(size, alignment).map_err(|_| PoolError::InvalidLayout)?;
        let bytes = capacity.checked_mul(size).ok_or(PoolError::InvalidLayout)?;
        let mut free = Vec::new();
        free.try_reserve_exact(capacity)
            .map_err(|_| PoolError::AllocationFailed)?;
        for _ in 0..capacity {
            free.push(Allocation::new(layout)?);
        }
        Ok(Self {
            inner: Rc::new(Inner {
                free: RefCell::new(free),
                count: Cell::new(capacity),
                bytes: Cell::new(bytes),
                minimum: size,
                alignment,
                #[cfg(test)]
                fail_allocation: Cell::new(false),
            }),
        })
    }
    /// Borrow a best-fit free allocation or grow immediately, without I/O waits.
    pub fn reserve(&self, len: usize) -> Result<BufferLease, PoolError> {
        let mut free = self.inner.free.borrow_mut();
        let allocation = if let Some(index) = free
            .iter()
            .enumerate()
            .filter(|(_, b)| b.capacity() >= len)
            .min_by_key(|(_, b)| b.capacity())
            .map(|(i, _)| i)
        {
            free.swap_remove(index)
        } else {
            let size = len
                .max(self.inner.minimum)
                .checked_next_multiple_of(self.inner.alignment)
                .ok_or(PoolError::InvalidLayout)?;
            let layout = Layout::from_size_align(size, self.inner.alignment)
                .map_err(|_| PoolError::InvalidLayout)?;
            let count = self
                .inner
                .count
                .get()
                .checked_add(1)
                .ok_or(PoolError::InvalidLayout)?;
            let bytes = self
                .inner
                .bytes
                .get()
                .checked_add(size)
                .ok_or(PoolError::InvalidLayout)?;
            // Reserve room for EVERY allocation to return, including outstanding leases.
            let additional = count - free.len();
            free.try_reserve(additional)
                .map_err(|_| PoolError::AllocationFailed)?;
            #[cfg(test)]
            if self.inner.fail_allocation.get() {
                return Err(PoolError::AllocationFailed);
            }
            let allocation = Allocation::new(layout)?;
            self.inner.count.set(count);
            self.inner.bytes.set(bytes);
            allocation
        };
        Ok(BufferLease {
            allocation: Some(allocation),
            pool: self.inner.clone(),
            len,
        })
    }
    pub fn available(&self) -> usize {
        self.inner.free.borrow().len()
    }
    pub fn capacity(&self) -> usize {
        self.inner.count.get()
    }
    pub fn allocated_bytes(&self) -> usize {
        self.inner.bytes.get()
    }
    pub fn free_bytes(&self) -> usize {
        self.inner.free.borrow().iter().map(|a| a.capacity()).sum()
    }
    pub(crate) fn allocations(&self) -> Result<Vec<Rc<Allocation>>, PoolError> {
        if self.available() != self.capacity() {
            return Err(PoolError::Busy);
        }
        Ok(self.inner.free.borrow().clone())
    }
    pub(crate) fn same_pool(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
    pub(crate) fn owns(&self, lease: &BufferLease) -> bool {
        Rc::ptr_eq(&self.inner, &lease.pool)
    }
    /// Explicit idle trim; registered allocations must first be unregistered.
    pub fn trim(&self) -> Result<usize, PoolError> {
        let mut free = self.inner.free.borrow_mut();
        if free.len() != self.capacity() || free.iter().any(|a| Rc::strong_count(a) != 1) {
            return Err(PoolError::Busy);
        }
        let bytes = self.inner.bytes.replace(0);
        free.clear();
        self.inner.count.set(0);
        Ok(bytes)
    }
}
/// Exclusive mutable lease. Drop returns the same allocation without allocating.
pub struct BufferLease {
    allocation: Option<Rc<Allocation>>,
    pool: Rc<Inner>,
    len: usize,
}
impl std::fmt::Debug for BufferLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferLease")
            .field("len", &self.len)
            .field("capacity", &self.capacity())
            .finish()
    }
}
impl BufferLease {
    pub(crate) fn allocation(&self) -> &Rc<Allocation> {
        self.allocation.as_ref().unwrap()
    }
    pub fn capacity(&self) -> usize {
        self.allocation().capacity()
    }
    pub(crate) fn restore_len(&mut self, len: usize) {
        assert!(len <= self.capacity());
        self.len = len;
    }
    pub fn truncate(&mut self, len: usize) {
        assert!(len <= self.len);
        self.len = len;
    }
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: initialized, retained allocation and bounded length.
        unsafe { std::slice::from_raw_parts(self.allocation().ptr(), self.len) }
    }
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: only this lease exposes mutation; registration owners never access bytes.
        unsafe { std::slice::from_raw_parts_mut(self.allocation().ptr(), self.len) }
    }
}
impl Drop for BufferLease {
    fn drop(&mut self) {
        if let Some(allocation) = self.allocation.take() {
            self.pool.free.borrow_mut().push(allocation);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allocation_failure_preserves_live_leases() {
        let pool = BufferPool::new(1, 4096, 4096).unwrap();
        let mut lease = pool.reserve(4096).unwrap();
        lease.as_mut_slice().fill(7);
        pool.inner.fail_allocation.set(true);
        assert!(matches!(
            pool.reserve(8192),
            Err(PoolError::AllocationFailed)
        ));
        assert_eq!(pool.capacity(), 1);
        assert_eq!(pool.allocated_bytes(), 4096);
        assert!(lease.as_slice().iter().all(|b| *b == 7));
        drop(lease);
        assert_eq!(pool.available(), 1);
    }
}
