//! Injectable bounded event collection and optional MCAP stage metrics.
mod mcap;
pub use mcap::{McapMetrics, McapMonitor, McapSnapshot};
use std::{
    collections::VecDeque,
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
pub(crate) fn next_id() -> u64 {
    NEXT_ID
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
        .expect("process observation identity exhausted")
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    Off,
    Summary,
    Inspect,
    Capture,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ids {
    pub writer: u64,
    pub file: u64,
    pub write: u64,
    pub operation: u64,
    pub region: u64,
    pub slab_use: u64,
    pub pool: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Accepted,
    Rejected(io::ErrorKind),
    SqeQueued,
    CqeObserved,
    Retry,
    Written,
    Failed(io::ErrorKind),
    Cancelled,
    SubmitFailed(io::ErrorKind),
    RegistrationHit,
    Registered,
    RegistrationFailed(io::ErrorKind),
    RegistrationRetired,
    SlabAcquired,
    SlabSealed,
    ReaderCloned,
    ReaderReleased,
    IoLeaseAcquired,
    IoLeaseReleased,
    SlabRecycled,
    SyncCompleted,
    SyncFailed(io::ErrorKind),
    FileRotated,
    CombiningFlushed,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct EventData {
    /// Zero identifies an inapplicable field. IDs are process-session scoped.
    pub ids: Ids,
    pub offset: u64,
    pub len: u64,
    pub result: Option<i32>,
    /// Registration, queue-to-observation, or sync duration, depending on kind.
    pub duration_us: Option<u64>,
}
#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub sequence: u64,
    pub elapsed_ns: u64,
    pub kind: EventKind,
    pub data: EventData,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct CaptureFilter {
    pub writer: Option<u64>,
    pub write: Option<u64>,
    pub region: Option<u64>,
}
impl CaptureFilter {
    fn matches(self, ids: Ids) -> bool {
        self.writer.is_none_or(|v| v == ids.writer)
            && self.write.is_none_or(|v| v == ids.write)
            && self.region.is_none_or(|v| v == ids.region)
    }
}
#[derive(Debug)]
struct History {
    events: VecDeque<Event>,
    capacity: usize,
    sequence: u64,
    overwritten: u64,
    until: Option<Instant>,
    filter: CaptureFilter,
}
#[derive(Debug)]
struct Shared {
    mode: AtomicU8,
    epoch: Instant,
    dropped: AtomicU64,
    history: Mutex<History>,
}
/// Cloneable producer handle. Contains no payload owner and invokes no exporter callback.
#[derive(Clone, Debug, Default)]
pub struct Observer {
    shared: Option<Arc<Shared>>,
}
/// Application-owned reader and runtime controls. Survives writer/pool destruction.
#[derive(Clone, Debug)]
pub struct Collector {
    shared: Arc<Shared>,
}
#[derive(Debug)]
pub struct EventBatch {
    pub events: Vec<Event>,
    pub next_cursor: u64,
    pub oldest_available: Option<u64>,
    pub missed_before_cursor: u64,
    pub overwritten_total: u64,
    pub contention_drops: u64,
}
impl Collector {
    pub fn new(capacity: usize, mode: Mode) -> io::Result<Self> {
        if capacity == 0 || mode == Mode::Capture {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut events = VecDeque::new();
        events
            .try_reserve_exact(capacity)
            .map_err(|_| io::ErrorKind::OutOfMemory)?;
        Ok(Self {
            shared: Arc::new(Shared {
                mode: AtomicU8::new(mode as u8),
                epoch: Instant::now(),
                dropped: AtomicU64::new(0),
                history: Mutex::new(History {
                    events,
                    capacity,
                    sequence: 0,
                    overwritten: 0,
                    until: None,
                    filter: CaptureFilter::default(),
                }),
            }),
        })
    }
    pub fn observer(&self) -> Observer {
        Observer {
            shared: Some(self.shared.clone()),
        }
    }
    /// Capture requires an explicit duration; use capture() for that mode.
    pub fn set_mode(&self, mode: Mode) -> io::Result<()> {
        if mode == Mode::Capture {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut h = self.shared.history.lock().unwrap();
        h.until = None;
        h.filter = CaptureFilter::default();
        self.shared.mode.store(mode as u8, Ordering::Release);
        Ok(())
    }
    /// Timed, optionally filtered detail. Expiry returns to Inspect.
    pub fn capture(&self, duration: Duration, filter: CaptureFilter) -> io::Result<()> {
        if duration.is_zero() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let until = Instant::now()
            .checked_add(duration)
            .ok_or(io::ErrorKind::InvalidInput)?;
        let mut h = self.shared.history.lock().unwrap();
        h.until = Some(until);
        h.filter = filter;
        self.shared
            .mode
            .store(Mode::Capture as u8, Ordering::Release);
        Ok(())
    }
    pub fn mode(&self) -> Mode {
        let mut h = self.shared.history.lock().unwrap();
        self.shared.expire(&mut h);
        decode_mode(self.shared.mode.load(Ordering::Acquire))
    }
    /// Cursor belongs to this collector. Reading never removes history.
    /// The exporter owns returned metadata; storage may overwrite older events.
    pub fn read_after(&self, cursor: u64, limit: usize) -> EventBatch {
        let mut h = self.shared.history.lock().unwrap();
        self.shared.expire(&mut h);
        let oldest = h.events.front().map(|e| e.sequence);
        let events: Vec<_> = h
            .events
            .iter()
            .filter(|e| e.sequence > cursor)
            .take(limit)
            .copied()
            .collect();
        EventBatch {
            next_cursor: events.last().map_or(cursor, |e| e.sequence),
            events,
            oldest_available: oldest,
            missed_before_cursor: oldest.map_or(0, |n| n.saturating_sub(cursor.saturating_add(1))),
            overwritten_total: h.overwritten,
            contention_drops: self.shared.dropped.load(Ordering::Relaxed),
        }
    }
}
fn decode_mode(mode: u8) -> Mode {
    match mode {
        0 => Mode::Off,
        1 => Mode::Summary,
        2 => Mode::Inspect,
        _ => Mode::Capture,
    }
}
impl Shared {
    fn expire(&self, h: &mut History) {
        if h.until.is_some_and(|until| Instant::now() >= until) {
            h.until = None;
            h.filter = CaptureFilter::default();
            self.mode.store(Mode::Inspect as u8, Ordering::Release);
        }
    }
}
impl Observer {
    pub(crate) fn now(&self) -> Option<Instant> {
        self.shared
            .as_ref()
            .filter(|s| s.mode.load(Ordering::Acquire) != Mode::Off as u8)
            .map(|_| Instant::now())
    }
    /// No event allocation, formatting, exporter callbacks or blocking on the producer.
    pub fn record(&self, kind: EventKind, data: impl FnOnce() -> EventData) {
        let Some(s) = &self.shared else { return };
        if s.mode.load(Ordering::Acquire) < Mode::Inspect as u8 {
            return;
        }
        let Ok(mut h) = s.history.try_lock() else {
            s.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        s.expire(&mut h);
        let mode = decode_mode(s.mode.load(Ordering::Acquire));
        if matches!(mode, Mode::Off | Mode::Summary) {
            return;
        }
        if mode != Mode::Capture
            && matches!(kind, EventKind::ReaderCloned | EventKind::ReaderReleased)
        {
            return;
        }
        let data = data();
        if mode == Mode::Capture && !h.filter.matches(data.ids) {
            return;
        }
        h.sequence += 1;
        let event = Event {
            sequence: h.sequence,
            elapsed_ns: s.epoch.elapsed().as_nanos().min(u64::MAX as u128) as u64,
            kind,
            data,
        };
        if h.events.len() == h.capacity {
            h.events.pop_front();
            h.overwritten += 1;
        }
        h.events.push_back(event);
    }
}
pub(crate) fn elapsed_us(start: Option<Instant>) -> Option<u64> {
    start.map(|t| t.elapsed().as_micros().min(u64::MAX as u128) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn noop_and_summary_skip_event_factory() {
        Observer::default().record(EventKind::Accepted, || panic!("disabled"));
        for mode in [Mode::Off, Mode::Summary] {
            Collector::new(2, mode)
                .unwrap()
                .observer()
                .record(EventKind::Accepted, || panic!("filtered"));
        }
        assert!(Observer::default().now().is_none());
    }
    #[test]
    fn concurrent_producers_account_for_every_attempt() {
        let c = Collector::new(8, Mode::Inspect).unwrap();
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let o = c.observer();
                std::thread::spawn(move || {
                    for _ in 0..1000 {
                        o.record(EventKind::Accepted, EventData::default);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let batch = c.read_after(0, 8);
        assert_eq!(batch.next_cursor + batch.contention_drops, 4000);
        assert_eq!(
            batch.overwritten_total + batch.events.len() as u64,
            batch.next_cursor
        );
        assert!(
            batch
                .events
                .windows(2)
                .all(|e| e[1].sequence == e[0].sequence + 1 && e[1].elapsed_ns >= e[0].elapsed_ns)
        );
    }
    #[test]
    fn bounded_history_contention_and_capture_expiry() {
        let c = Collector::new(2, Mode::Inspect).unwrap();
        let o = c.observer();
        for _ in 0..4 {
            o.record(EventKind::Accepted, EventData::default);
        }
        let b = c.read_after(0, 1);
        assert_eq!(
            (b.next_cursor, b.missed_before_cursor, b.overwritten_total),
            (3, 2, 2)
        );
        assert_eq!(c.read_after(3, 1).events[0].sequence, 4);
        {
            let _lock = c.shared.history.lock().unwrap();
            o.record(EventKind::Accepted, || panic!("contended"));
        }
        assert_eq!(c.read_after(0, 2).contention_drops, 1);
        c.capture(
            Duration::from_secs(10),
            CaptureFilter {
                region: Some(7),
                ..Default::default()
            },
        )
        .unwrap();
        o.record(EventKind::ReaderCloned, EventData::default);
        assert_eq!(c.read_after(4, 10).events.len(), 0);
        o.record(EventKind::ReaderCloned, || EventData {
            ids: Ids {
                region: 7,
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(c.read_after(4, 10).events.len(), 1);
        c.shared.history.lock().unwrap().until = Some(Instant::now());
        assert_eq!(c.mode(), Mode::Inspect);
        o.record(EventKind::ReaderCloned, || panic!("verbose filtered"));
        c.set_mode(Mode::Off).unwrap();
        o.record(EventKind::Accepted, || panic!("off"));
    }
}
