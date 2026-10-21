use storage_persistence::{
    file_offsets::{OffsetAllocator, OffsetError},
    managed_buffers::{BufferPool, PoolError},
};
#[path = "support/allocations.rs"]
mod allocations;

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
        let third = pool.reserve(1).unwrap();
        let large = pool.reserve(8193).unwrap();
        assert_eq!(large.capacity(), 8193_usize.next_multiple_of(alignment));
        assert_eq!(pool.capacity(), 4);
        drop(third);
        drop(large);
        drop(first);
        assert_eq!(pool.available(), 3);
        drop(second);
        assert_eq!(pool.available(), pool.capacity());
        let mut offsets = OffsetAllocator::new(0, alignment as u64).unwrap();
        allocations::start();
        for _ in 0..10000 {
            let mut lease = pool.reserve(alignment).unwrap();
            lease.as_mut_slice().fill(1);
            let reservation = offsets.reserve(alignment as u64).unwrap();
            reservation.commit();
        }
        let allocations = allocations::finish();
        assert_eq!(allocations, 0);
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

#[test]
fn allocation_counter_is_thread_local() {
    // Arrange
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other_barrier = barrier.clone();
    let other = std::thread::spawn(move || {
        other_barrier.wait();
        let allocation = std::hint::black_box(Box::new(42_u64));
        drop(std::hint::black_box(allocation));
        other_barrier.wait();
    });

    // Act
    allocations::start();
    barrier.wait();
    barrier.wait();
    let other_thread_allocations = allocations::finish();
    other.join().unwrap();

    // Assert
    assert_eq!(other_thread_allocations, 0);

    // Act
    allocations::start();
    let allocation = std::hint::black_box(Box::new(42_u64));
    let local_allocations = allocations::finish();
    drop(allocation);

    // Assert
    assert_eq!(local_allocations, 1);
}
