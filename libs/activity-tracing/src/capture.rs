use crate::{ActivitySubscriber, Result};
use crossbeam_queue::ArrayQueue;
use std::{
    cell::RefCell,
    marker::PhantomData,
    rc::Rc,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tracing_core::Metadata;

pub(crate) const FIELDS: usize = 8;
pub(crate) const TEXT: usize = 256;

#[derive(Clone, Debug)]
pub struct Config {
    pub blocks: usize,
    pub records_per_block: usize,
    pub string_bytes_per_block: usize,
    pub max_spans: usize,
    pub max_depth: usize,
    pub max_threads: usize,
    pub encoded_buffers: usize,
    pub sink_queue: usize,
    /// Flush every standalone event for prompt live delivery. Span boundaries always flush.
    pub flush_events: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            blocks: 64,
            records_per_block: 64,
            string_bytes_per_block: 8192,
            max_spans: 4096,
            max_depth: 64,
            max_threads: 64,
            encoded_buffers: 16,
            sink_queue: 4,
            flush_events: true,
        }
    }
}
impl Config {
    pub(crate) fn validate(&self) -> Result<()> {
        if [
            self.blocks,
            self.records_per_block,
            self.string_bytes_per_block,
            self.max_spans,
            self.max_depth,
            self.max_threads,
            self.encoded_buffers,
            self.sink_queue,
        ]
        .contains(&0)
        {
            return Err("capacities must be nonzero".into());
        }
        if self.string_bytes_per_block > u32::MAX as usize
            || self.records_per_block > 65536
            || self.max_spans > 1_000_000
            || self.max_depth > 65536
            || self.max_threads > 65536
        {
            return Err("capacity exceeds supported limit".into());
        }
        self.encoded_capacity().ok_or("encoded capacity overflow")?;
        Ok(())
    }
    pub(crate) fn encoded_capacity(&self) -> Option<usize> {
        // Worst-case dictionary bytes, per-record fields and varint framing.
        self.records_per_block
            .checked_mul(4096)?
            .checked_add(self.string_bytes_per_block)?
            .checked_add(1024)
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Losses {
    pub records: u64,
    pub fields: u64,
    pub spans: u64,
    pub unregistered: u64,
    pub encoded_batches: u64,
}
#[derive(Default)]
pub(crate) struct Counters {
    pub records: AtomicU64,
    pub fields: AtomicU64,
    pub spans: AtomicU64,
    pub unregistered: AtomicU64,
    pub encoded_batches: AtomicU64,
}
impl Counters {
    pub fn snapshot(&self) -> Losses {
        Losses {
            records: self.records.load(Ordering::Relaxed),
            fields: self.fields.load(Ordering::Relaxed),
            spans: self.spans.load(Ordering::Relaxed),
            unregistered: self.unregistered.load(Ordering::Relaxed),
            encoded_batches: self.encoded_batches.load(Ordering::Relaxed),
        }
    }
}
#[derive(Clone, Copy, Default)]
pub(crate) struct Field {
    pub name: &'static str,
    pub kind: u32,
    pub bits: u64,
    pub offset: u32,
    pub len: u32,
}
#[derive(Clone, Copy, Default)]
pub(crate) struct Record {
    pub kind: u32,
    pub time: u64,
    pub span: u64,
    pub root: u64,
    pub parent: u64,
    pub linked: u64,
    pub linked_root: u64,
    pub tid: u32,
    pub thread: &'static str,
    pub name: &'static str,
    pub fields: [Field; FIELDS],
    pub count: usize,
}
pub(crate) struct Block {
    pub records: Vec<Record>,
    pub strings: Vec<u8>,
}
impl Block {
    fn new(c: &Config) -> Self {
        Self {
            records: {
                let mut v = vec![Record::default(); c.records_per_block];
                v.clear();
                v
            },
            strings: {
                let mut v = vec![0; c.string_bytes_per_block];
                v.clear();
                v
            },
        }
    }
    pub fn reset(&mut self) {
        self.records.clear();
        self.strings.clear();
    }
}
#[derive(Clone, Copy)]
pub(crate) struct SpanState {
    pub id: u64,
    pub refs: usize,
    pub root: u64,
    pub parent: u64,
    pub meta: &'static Metadata<'static>,
}
pub(crate) struct Registry {
    pub slots: Vec<Option<SpanState>>,
    pub next: u64,
}
pub(crate) struct Core {
    pub config: Config,
    pub free: ArrayQueue<Block>,
    pub ready: ArrayQueue<Block>,
    pub losses: Counters,
    pub registry: Mutex<Registry>,
    pub session: u64,
    pub unix_origin: u64,
    pub origin: Instant,
    pub accepting: AtomicBool,
    pub registered: AtomicUsize,
    pub wake: OnceLock<std::thread::Thread>,
    pub encoded_pool: OnceLock<std::sync::Weak<crate::pipeline::Pool>>,
}
impl Core {
    pub fn new(config: Config) -> Result<Arc<Self>> {
        config.validate()?;
        let mut seed = [0u8; 8];
        use std::io::Read;
        std::fs::File::open("/dev/urandom")?.read_exact(&mut seed)?;
        let origin = Instant::now();
        let unix_origin = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .try_into()?;
        let core = Arc::new(Self {
            free: ArrayQueue::new(config.blocks),
            ready: ArrayQueue::new(config.blocks),
            registry: Mutex::new(Registry {
                slots: vec![None; config.max_spans],
                next: 1,
            }),
            session: u64::from_le_bytes(seed).max(1),
            unix_origin,
            origin,
            accepting: AtomicBool::new(true),
            registered: AtomicUsize::new(0),
            wake: OnceLock::new(),
            encoded_pool: OnceLock::new(),
            losses: Counters::default(),
            config,
        });
        for _ in 0..core.config.blocks {
            assert!(core.free.push(Block::new(&core.config)).is_ok());
        }
        Ok(core)
    }
    pub fn recycle(&self, mut b: Block) {
        b.reset();
        assert!(self.free.push(b).is_ok());
    }
    pub fn publish(&self, b: Block) {
        if b.records.is_empty() {
            self.recycle(b);
            return;
        }
        assert!(self.ready.push(b).is_ok());
        if let Some(t) = self.wake.get() {
            t.unpark();
        }
    }
    pub fn context(&self, id: u64) -> Option<SpanState> {
        let r = self.registry.lock().unwrap();
        r.slots[(id as usize) % r.slots.len()].filter(|s| s.id == id)
    }
    pub fn current(&self) -> Option<u64> {
        LOCAL.with(|l| {
            l.borrow()
                .as_ref()
                .filter(|s| std::ptr::eq(self, Arc::as_ptr(&s.core)))
                .and_then(|s| {
                    if s.overflow_depth == 0 {
                        s.stack.last().copied()
                    } else {
                        None
                    }
                })
        })
    }
    pub fn contextual_parent(&self) -> Option<(u64, u64)> {
        let scoped = LOCAL.with(|local| {
            local
                .borrow()
                .as_ref()
                .filter(|s| std::ptr::eq(self, Arc::as_ptr(&s.core)))
                .and_then(|s| {
                    s.parent_scope
                        .filter(|(_, depth)| s.overflow_depth == 0 && *depth == s.stack.len())
                        .map(|(c, _)| (c.root, c.span))
                })
        });
        scoped.or_else(|| {
            self.current()
                .and_then(|id| self.context(id))
                .map(|s| (s.root, s.id))
        })
    }
    pub fn emit(&self, mut record: Record, visit: impl FnOnce(&mut Record, &mut Vec<u8>)) {
        if !self.accepting.load(Ordering::Acquire) {
            return;
        }
        LOCAL.with(|l| {
            let Ok(mut local) = l.try_borrow_mut() else {
                self.losses.records.fetch_add(1, Ordering::Relaxed);
                return;
            };
            let Some(state) = local
                .as_mut()
                .filter(|s| std::ptr::eq(self, Arc::as_ptr(&s.core)))
            else {
                self.losses.unregistered.fetch_add(1, Ordering::Relaxed);
                return;
            };
            if state.block.is_none() {
                state.block = self.free.pop();
            }
            let Some(b) = state.block.as_mut() else {
                self.losses.records.fetch_add(1, Ordering::Relaxed);
                return;
            };
            record.time = self.origin.elapsed().as_nanos().min(u64::MAX as u128) as u64;
            record.tid = state.tid;
            record.thread = state.name;
            visit(&mut record, &mut b.strings);
            b.records.push(record);
            if b.records.len() == self.config.records_per_block
                || record.kind != 1
                || self.config.flush_events
            {
                self.publish(state.block.take().unwrap());
            }
        });
    }
}
pub(crate) struct Local {
    pub core: Arc<Core>,
    pub block: Option<Block>,
    pub tid: u32,
    pub name: &'static str,
    pub stack: Vec<u64>,
    pub overflow_depth: usize,
    pub parent_scope: Option<(ActivityContext, usize)>,
}
impl Drop for Local {
    fn drop(&mut self) {
        if let Some(b) = self.block.take() {
            if self.core.accepting.load(Ordering::Acquire) {
                self.core.publish(b)
            } else {
                self.core.recycle(b)
            }
        }
        self.core.registered.fetch_sub(1, Ordering::Release);
    }
}
thread_local! { pub(crate) static LOCAL:RefCell<Option<Local>>=const{RefCell::new(None)}; }
/// Copied causal identity for same-runtime message passing. Holding it keeps no span alive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivityContext {
    session: u64,
    root: u64,
    span: u64,
}
impl ActivityContext {
    pub fn span_id(self) -> u64 {
        self.span
    }
    pub fn trace_root(self) -> u64 {
        self.root
    }
}
struct ParentScope<'a> {
    core: &'a Arc<Core>,
    previous: Option<(ActivityContext, usize)>,
}
impl Drop for ParentScope<'_> {
    fn drop(&mut self) {
        LOCAL.with(|l| {
            if let Some(s) = l
                .borrow_mut()
                .as_mut()
                .filter(|s| Arc::ptr_eq(&s.core, self.core))
            {
                s.parent_scope = self.previous;
            }
        });
    }
}
/// Approximate concurrent occupancy and configured payload capacity. Queue lengths
/// are sampled independently. Reserved bytes exclude allocator/queue metadata,
/// thread stacks, span registry, encoder scratch space and sink-owned allocations.
#[derive(Clone, Copy, Debug)]
pub struct BufferSnapshot {
    pub capture_blocks: usize,
    pub capture_free: usize,
    pub capture_ready: usize,
    pub capture_reserved_bytes: usize,
    pub encoded_buffers: usize,
    pub encoded_free: usize,
    pub encoded_reserved_bytes: usize,
    pub registered_threads: usize,
}
#[derive(Clone)]
pub struct Handle(pub(crate) Arc<Core>);
impl Handle {
    pub fn subscriber(&self) -> ActivitySubscriber {
        ActivitySubscriber(self.0.clone())
    }
    /// Registration may allocate. Keep the guard on this OS thread until capture completes.
    pub fn register_thread(&self, name: &'static str) -> Result<Registration> {
        if name.len() > TEXT {
            return Err("thread name exceeds 256 bytes".into());
        }
        LOCAL.with(|l| {
            let mut l = l.borrow_mut();
            if l.is_some() {
                return Err("thread already registered".into());
            }
            // Serializes registration with shutdown's accepting transition.
            let _registry = self.0.registry.lock().unwrap();
            if !self.0.accepting.load(Ordering::Acquire) {
                return Err("capture is shut down".into());
            }
            if self.0.registered.load(Ordering::Acquire) >= self.0.config.max_threads {
                return Err("registered thread capacity exhausted".into());
            }
            let mut stack = vec![0; self.0.config.max_depth];
            stack.clear();
            #[cfg(target_os = "linux")]
            let tid = unsafe { libc::syscall(libc::SYS_gettid) as u32 };
            #[cfg(not(target_os = "linux"))]
            compile_error!("activity-tracing currently requires Linux OS thread IDs");
            self.0.registered.fetch_add(1, Ordering::Release);
            *l = Some(Local {
                core: self.0.clone(),
                block: None,
                tid,
                name,
                stack,
                overflow_depth: 0,
                parent_scope: None,
            });
            Ok(Registration {
                _thread: PhantomData,
            })
        })
    }
    /// Snapshot a span's identity without cloning or extending its lifetime.
    pub fn context(&self, span: &tracing::Span) -> Option<ActivityContext> {
        span.with_subscriber(|(id, dispatch)| {
            dispatch
                .downcast_ref::<ActivitySubscriber>()
                .filter(|s| Arc::ptr_eq(&s.0, &self.0))?;
            self.0.context(id.into_u64()).map(|s| ActivityContext {
                session: self.0.session,
                root: s.root,
                span: s.id,
            })
        })
        .flatten()
    }
    /// Establish the parent for new contextual spans in a synchronous callback.
    /// Call on a registered thread. The token must originate from this runtime.
    pub fn with_parent<T>(&self, context: ActivityContext, f: impl FnOnce() -> T) -> Result<T> {
        if context.session != self.0.session {
            return Err("context belongs to another runtime".into());
        }
        let previous = LOCAL.with(|l| {
            let mut local = l.borrow_mut();
            let s = local
                .as_mut()
                .filter(|s| Arc::ptr_eq(&s.core, &self.0))
                .ok_or("thread is not registered")?;
            Ok::<_, crate::Error>(s.parent_scope.replace((context, s.stack.len())))
        })?;
        let _scope = ParentScope {
            core: &self.0,
            previous,
        };
        Ok(f())
    }
    /// Read pool occupancy without adding work to producer capture callbacks.
    pub fn buffers(&self) -> BufferSnapshot {
        let c = &self.0.config;
        BufferSnapshot {
            capture_blocks: c.blocks,
            capture_free: self.0.free.len(),
            capture_ready: self.0.ready.len(),
            capture_reserved_bytes: c.blocks
                * (c.records_per_block * std::mem::size_of::<Record>() + c.string_bytes_per_block),
            encoded_buffers: c.encoded_buffers,
            encoded_free: self
                .0
                .encoded_pool
                .get()
                .and_then(std::sync::Weak::upgrade)
                .map_or(c.encoded_buffers, |p| p.available()),
            encoded_reserved_bytes: c.encoded_buffers * c.encoded_capacity().unwrap(),
            registered_threads: self.0.registered.load(Ordering::Relaxed),
        }
    }
    pub fn losses(&self) -> Losses {
        self.0.losses.snapshot()
    }
}
pub struct Registration {
    _thread: PhantomData<Rc<()>>,
}
impl Registration {
    /// Publish a partial block. Call before sleeping when batching events.
    pub fn flush(&self) {
        LOCAL.with(|l| {
            let mut l = l.borrow_mut();
            if let Some(s) = l.as_mut()
                && let Some(b) = s.block.take()
            {
                s.core.publish(b);
            }
        });
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        LOCAL.with(|l| {
            l.borrow_mut().take();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn buffer_snapshot_distinguishes_held_queued_and_free_blocks() {
        let core = Core::new(Config {
            blocks: 2,
            flush_events: false,
            ..Default::default()
        })
        .unwrap();
        let handle = Handle(core.clone());
        let initial = handle.buffers();
        assert_eq!(initial.capture_free, 2);
        assert_eq!(
            initial.capture_reserved_bytes,
            2 * (64 * std::mem::size_of::<Record>() + 8192)
        );
        let registration = handle.register_thread("snapshot").unwrap();
        core.emit(
            Record {
                kind: 1,
                ..Default::default()
            },
            |_, _| {},
        );
        let held = handle.buffers();
        assert_eq!(
            (
                held.capture_free,
                held.capture_ready,
                held.registered_threads
            ),
            (1, 0, 1)
        );
        registration.flush();
        assert_eq!(handle.buffers().capture_ready, 1);
        core.recycle(core.ready.pop().unwrap());
        assert_eq!(handle.buffers().capture_free, 2);
        drop(registration);
        assert_eq!(handle.buffers().registered_threads, 0);
    }
    #[test]
    fn exhaustion_recycles_and_counts_exactly() {
        let core = Core::new(Config {
            blocks: 1,
            ..Default::default()
        })
        .unwrap();
        let handle = Handle(core.clone());
        let reg = handle.register_thread("test").unwrap();
        core.emit(
            Record {
                kind: 1,
                ..Default::default()
            },
            |_, _| {},
        );
        core.emit(
            Record {
                kind: 1,
                ..Default::default()
            },
            |_, _| {},
        );
        assert_eq!(core.losses.snapshot().records, 1);
        assert!(core.free.is_empty());
        core.recycle(core.ready.pop().unwrap());
        core.emit(
            Record {
                kind: 1,
                ..Default::default()
            },
            |_, _| {},
        );
        assert_eq!(core.ready.len(), 1);
        drop(reg);
    }
    #[test]
    fn concurrent_producers_have_complete_accounting() {
        let core = Core::new(Config {
            blocks: 8,
            ..Default::default()
        })
        .unwrap();
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let c = core.clone();
                scope.spawn(move || {
                    let _r = Handle(c.clone()).register_thread("producer").unwrap();
                    for _ in 0..1000 {
                        c.emit(
                            Record {
                                kind: 1,
                                ..Default::default()
                            },
                            |_, _| {},
                        );
                    }
                });
            }
        });
        let mut received = 0;
        while let Some(b) = core.ready.pop() {
            received += b.records.len();
            core.recycle(b);
        }
        assert_eq!(received as u64 + core.losses.snapshot().records, 4000);
        assert_eq!(core.free.len(), 8);
    }
}
