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

fn emit() {
    let span = tracing::info_span!(target:"activity","work",task.name="camera",frame=42u64);
    let entered = span.enter();
    tracing::info!(target:"activity",text="frame ready",value=42u64,formatted=?42u64);
    let clone = span.clone();
    drop(clone);
    drop(entered);
    drop(span);
}
struct Count(std::sync::Arc<std::sync::atomic::AtomicU64>);
impl activity_tracing::Sink for Count {
    fn name(&self) -> &str {
        "count"
    }
    fn write(&mut self, _: activity_tracing::BatchView<'_>) -> activity_tracing::Result<()> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Release);
        Ok(())
    }
}
#[test]
fn registered_producer_allocates_nothing_including_overflow() {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    let completed = Arc::new(AtomicU64::new(0));
    let mut runtime = activity_tracing::Runtime::start(
        activity_tracing::Config {
            blocks: 16,
            max_spans: 2,
            ..Default::default()
        },
        vec![Box::new(Count(completed.clone()))],
    )
    .unwrap();
    let handle = runtime.handle();
    let registration = handle.register_thread("allocation-test").unwrap();
    let dispatch = tracing::Dispatch::new(handle.subscriber());
    let _default = tracing::dispatcher::set_default(&dispatch);
    emit();
    let wait = |n| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while completed.load(Ordering::Acquire) < n {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    };
    wait(3);
    ALLOCATIONS.with(|c| c.set(0));
    for n in 1..=1000 {
        COUNTING.with(|c| c.set(true));
        emit();
        COUNTING.with(|c| c.set(false));
        wait((n + 1) * 3);
    }
    assert_eq!(handle.losses().records, 0);
    COUNTING.with(|c| c.set(true));
    for _ in 0..10_000 {
        emit();
    }
    for _ in 0..1_000 {
        let publish = tracing::info_span!(target:"activity","publish");
        let context = handle.context(&publish).unwrap();
        drop(publish);
        handle
            .with_parent(context, || {
                drop(tracing::info_span!(target:"activity","receive"));
            })
            .unwrap();
    }
    registration.flush();
    COUNTING.with(|c| c.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    drop(registration);
    runtime.shutdown().unwrap();
}
