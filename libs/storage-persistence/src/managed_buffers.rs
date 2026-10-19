//! Single-threaded, fixed-capacity aligned memory with exclusive RAII leases.
//! Kernel registration is added by the I/O layer; alignment alone does not pin memory.
use std::{
    alloc::{Layout, alloc_zeroed, dealloc},
    cell::RefCell,
    ptr::NonNull,
    rc::Rc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolError {
    InvalidLayout,
    AllocationFailed,
    Exhausted,
    TooLarge,
}

struct Buffer {
    ptr: NonNull<u8>,
    layout: Layout,
    index: usize,
}
impl Buffer {
    fn new(layout: Layout, index: usize) -> Result<Self, PoolError> {
        // SAFETY: the validated layout has nonzero size; null is handled below.
        let ptr =
            NonNull::new(unsafe { alloc_zeroed(layout) }).ok_or(PoolError::AllocationFailed)?;
        Ok(Self { ptr, layout, index })
    }
}
impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: this owns the allocation, with its original layout.
        unsafe { dealloc(self.ptr.as_ptr(), self.layout) }
    }
}
struct Inner {
    free: RefCell<Vec<Buffer>>,
    capacity: usize,
    size: usize,
}
/// Each lease owns its buffer; clones of the pool share a bounded free list.
/// Rc deliberately confines the pool and leases to their owner thread.
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
        capacity.checked_mul(size).ok_or(PoolError::InvalidLayout)?;
        let mut free = Vec::new();
        free.try_reserve_exact(capacity)
            .map_err(|_| PoolError::AllocationFailed)?;
        for index in 0..capacity {
            free.push(Buffer::new(layout, index)?);
        }
        Ok(Self {
            inner: Rc::new(Inner {
                free: RefCell::new(free),
                capacity,
                size,
            }),
        })
    }
    pub fn reserve(&self, len: usize) -> Result<BufferLease, PoolError> {
        if len > self.inner.size {
            return Err(PoolError::TooLarge);
        }
        let buffer = self
            .inner
            .free
            .borrow_mut()
            .pop()
            .ok_or(PoolError::Exhausted)?;
        Ok(BufferLease {
            buffer: Some(buffer),
            pool: self.inner.clone(),
            len,
        })
    }
    pub fn available(&self) -> usize {
        self.inner.free.borrow().len()
    }
    pub(crate) fn registration(&self) -> Result<Vec<libc::iovec>, PoolError> {
        let free = self.inner.free.borrow();
        if free.len() != self.inner.capacity {
            return Err(PoolError::Exhausted);
        }
        let mut buffers: Vec<_> = free.iter().collect();
        buffers.sort_by_key(|b| b.index);
        Ok(buffers
            .iter()
            .map(|b| libc::iovec {
                iov_base: b.ptr.as_ptr().cast(),
                iov_len: b.layout.size(),
            })
            .collect())
    }
    pub(crate) fn owns(&self, lease: &BufferLease) -> bool {
        Rc::ptr_eq(&self.inner, &lease.pool)
    }
    pub fn capacity(&self) -> usize {
        self.inner.capacity
    }
}
/// Dropping an abandoned lease returns its buffer. The I/O layer must retain
/// submitted leases until all kernel references retire.
pub struct BufferLease {
    buffer: Option<Buffer>,
    pool: Rc<Inner>,
    len: usize,
}
impl BufferLease {
    pub(crate) fn index(&self) -> usize {
        self.buffer.as_ref().unwrap().index
    }
    /// Shortens the initialized range submitted to storage.
    pub fn truncate(&mut self, len: usize) {
        assert!(len <= self.len);
        self.len = len;
    }

    pub fn as_slice(&self) -> &[u8] {
        let buffer = self.buffer.as_ref().unwrap();
        // SAFETY: allocation is initialized, retained by self, and len is bounded.
        unsafe { std::slice::from_raw_parts(buffer.ptr.as_ptr(), self.len) }
    }
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        let buffer = self.buffer.as_mut().unwrap();
        // SAFETY: the lease exclusively owns this buffer and self is mutably borrowed.
        unsafe { std::slice::from_raw_parts_mut(buffer.ptr.as_ptr(), self.len) }
    }
}
impl Drop for BufferLease {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            self.pool.free.borrow_mut().push(buffer);
        }
    }
}
