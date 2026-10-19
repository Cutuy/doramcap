use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use storage_persistence::{
    file_offsets::{OffsetAllocator, OffsetError},
    managed_buffers::{BufferPool, PoolError},
};
struct CountAlloc;
static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for CountAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if COUNTING.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static GLOBAL: CountAlloc = CountAlloc;

// One test keeps the allocation measurement isolated from parallel test cases.
#[test]
fn managed_resources_gate() {
    for alignment in [512, 4096] {
        let pool = BufferPool::new(2, 8192, alignment).unwrap();
        let mut first = pool.reserve(8192).unwrap();
        let mut second = pool.reserve(1).unwrap();
        assert_eq!(first.as_slice().as_ptr() as usize % alignment, 0);
        assert_ne!(first.as_slice().as_ptr(), second.as_slice().as_ptr());
        first.as_mut_slice().fill(0x55);
        second.as_mut_slice()[0] = 0xaa;
        assert!(first.as_slice().iter().all(|b| *b == 0x55));
        assert!(matches!(pool.reserve(1), Err(PoolError::Exhausted)));
        assert!(matches!(pool.reserve(8193), Err(PoolError::TooLarge)));
        drop(first);
        assert_eq!(pool.available(), 1);
        drop(second);
        assert_eq!(pool.available(), pool.capacity());
        let mut offsets = OffsetAllocator::new(0, alignment as u64).unwrap();
        COUNTING.store(true, Relaxed);
        for _ in 0..10000 {
            let mut lease = pool.reserve(alignment).unwrap();
            lease.as_mut_slice().fill(1);
            let reservation = offsets.reserve(alignment as u64).unwrap();
            reservation.commit();
        }
        COUNTING.store(false, Relaxed);
        assert_eq!(ALLOCS.load(Relaxed), 0);
        assert_eq!(offsets.next_offset(), 10000 * alignment as u64);
        let lease = pool.reserve(1).unwrap();
        drop(pool);
        assert_eq!(lease.as_slice().len(), 1); // owner survives pool handle
    }
    assert!(matches!(
        BufferPool::new(0, 1, 512),
        Err(PoolError::InvalidLayout)
    ));
    assert!(matches!(
        BufferPool::new(1, 0, 512),
        Err(PoolError::InvalidLayout)
    ));
    assert!(matches!(
        BufferPool::new(1, 1, 3),
        Err(PoolError::InvalidLayout)
    ));
    let mut offsets = OffsetAllocator::new(0, 512).unwrap();
    {
        let reservation = offsets.reserve(512).unwrap();
        assert_eq!(reservation.range().offset, 0);
    }
    assert_eq!(offsets.next_offset(), 0);
    let first = offsets.reserve(512).unwrap().commit();
    let second = offsets.reserve(1024).unwrap().commit();
    assert_eq!(first.offset + first.len, second.offset);
    assert!(matches!(offsets.reserve(1), Err(OffsetError::Unaligned)));
    assert!(matches!(offsets.reserve(0), Err(OffsetError::Empty)));
    let mut end = OffsetAllocator::new(u64::MAX - 511, 512).unwrap();
    assert!(matches!(end.reserve(512), Err(OffsetError::Overflow)));
    assert_eq!(end.next_offset(), u64::MAX - 511);
}
