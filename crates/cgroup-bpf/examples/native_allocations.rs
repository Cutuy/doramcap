use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct CountingAllocator;
fn count() {
    let _ = COUNTING.try_with(|enabled| {
        if enabled.get() {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        }
    });
}
unsafe impl GlobalAlloc for CountingAllocator {
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
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = cgroup_bpf::Backend::new(16384)?;
    let mut rows = Vec::with_capacity(16384);
    for _ in 0..3 {
        backend.collect_into(&mut rows)?;
    }
    ALLOCATIONS.with(|count| count.set(0));
    let mut elapsed = [0u128; 40];
    for duration in &mut elapsed {
        COUNTING.with(|enabled| enabled.set(true));
        let start = std::time::Instant::now();
        let result = backend.collect_into(&mut rows);
        *duration = start.elapsed().as_micros();
        COUNTING.with(|enabled| enabled.set(false));
        let stats = result?;
        assert_eq!(stats.overflow, 0);
        assert!(!rows.is_empty());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let allocations = ALLOCATIONS.with(Cell::get);
    elapsed.sort();
    println!(
        "native iterations=40 rows={} allocations={allocations} median_us={} max_us={}",
        rows.len(),
        elapsed[20],
        elapsed[39]
    );
    assert_eq!(allocations, 0, "steady-state native collection allocated");
    Ok(())
}
