use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct Allocator;
fn count() {
    let _ = ENABLED.try_with(|e| {
        if e.get() {
            let _ = ALLOCATIONS.try_with(|a| a.set(a.get() + 1));
        }
    });
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
#[test]
fn fixed_records_allocate_nothing_on_capture_including_overload() {
    use fault_reporter::*;
    let (reporter, mut publisher) = Reporter::new(
        1,
        Config {
            failure_capacity: 2,
            success_capacity: 2,
            ..Default::default()
        },
    )
    .unwrap();
    let mut definition = Definition::new("writer", "disk", "io").unwrap();
    definition.policy.success_interval = Some(std::time::Duration::from_secs(1));
    let check = reporter.register(definition).unwrap();
    let detail = Text::new("disk offline").unwrap();
    check.success().unwrap();
    ALLOCATIONS.with(|c| c.set(0));
    ENABLED.with(|c| c.set(true));
    for _ in 0..10_000 {
        check.failure(detail).unwrap();
        check.success().unwrap();
        check.success().unwrap();
    }
    while publisher.try_recv().is_some() {}
    ENABLED.with(|c| c.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert!(reporter.losses().failures > 0);
    assert!(reporter.losses().successes > 0);
    assert!(reporter.losses().coalesced_successes > 0);
}
