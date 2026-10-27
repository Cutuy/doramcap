//! Protobuf workflow emission through the shared telemetry contract.
//!
//! Supply a short nonblocking sink (typically bounded `try_send`). Sequence numbers
//! advance even when encoding admission or delivery fails, making loss visible.
//! Clone the observer to read counters after moving it into a LocalExecutor. No transport,
//! threads or retained history are created. Payload capture follows executor payload-capture setting.
use crate::{Event, Observer, State};
use std::{cell::Cell, rc::Rc};
use telemetry_protocol::workflow as wire;

type Sink = dyn Fn(&[u8]) -> std::io::Result<()>;
struct Inner {
    sink: Box<Sink>,
    limit: usize,
    next: Cell<u64>,
    dropped: Cell<u64>,
}
/// Shared local observer. Each instance owns one sequence space across contexts.
#[derive(Clone)]
pub struct TelemetryObserver(Rc<Inner>);
impl TelemetryObserver {
    /// Create an observer with a maximum encoded event size. Zero rejects all events.
    pub fn new(max_bytes: usize, sink: impl Fn(&[u8]) -> std::io::Result<()> + 'static) -> Self {
        Self(Rc::new(Inner {
            sink: Box::new(sink),
            limit: max_bytes,
            next: Cell::new(1),
            dropped: Cell::new(0),
        }))
    }
    /// Encoding-admission and sink failures. Sink panics count here and propagate
    /// to LocalExecutor's observer-panic isolation; neither path retries side effects.
    pub fn dropped_events(&self) -> u64 {
        self.0.dropped.get()
    }
}
impl Observer for TelemetryObserver {
    fn observe(&self, event: &Event) {
        // Count pessimistically so a panicking sink is visible as loss.
        self.0.dropped.set(self.0.dropped.get().saturating_add(1));
        let sequence = self.0.next.get();
        let Some(next) = sequence.checked_add(1) else {
            return;
        };
        self.0.next.set(next);
        let state = match event.state {
            State::Started => wire::event::State::Started,
            State::Succeeded => wire::event::State::Succeeded,
            State::Failed => wire::event::State::Failed,
            State::Cancelled => wire::event::State::Cancelled,
            State::Panicked => wire::event::State::Panicked,
        };
        let payload_json = match event
            .payload
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
        {
            Ok(value) => value,
            Err(_) => return,
        };
        let packet = wire::Event {
            version: wire::VERSION,
            sequence,
            session: event.invocation.session.clone(),
            invocation: event.invocation.id,
            step: event.invocation.step.clone(),
            step_version: event.invocation.version,
            executor: event.invocation.executor.clone(),
            parent: event.invocation.parent,
            dependencies: event.invocation.dependencies.clone(),
            state: state.into(),
            elapsed_ns: event.elapsed_ns,
            duration_ns: event.duration_ns,
            payload_json,
            error: event.error.clone(),
        };
        if wire::encoded_len(&packet) > self.0.limit {
            return;
        }
        if (self.0.sink)(&wire::encode(&packet)).is_ok() {
            self.0.dropped.set(self.0.dropped.get().saturating_sub(1));
        }
    }
}
