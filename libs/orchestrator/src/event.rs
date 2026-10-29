use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Identity of one call. IDs are monotonic within one executor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invocation {
    /// Caller-supplied stream namespace, distinct for each contributing executor.
    pub session: String,
    /// Local invocation number, starting at one.
    pub id: u64,
    /// Derived from the step's Rust type name; renames change this name.
    pub step: String,
    /// Step input/output contract revision.
    pub version: u32,
    /// Executor identity. The implementation supports only `local`.
    pub executor: String,
    /// Containing step, when this call is nested.
    pub parent: Option<u64>,
    /// Producer invocation IDs explicitly carried into this call.
    pub dependencies: Vec<u64>,
}

/// Observable state of a call. These events are not a recovery protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Input preparation returned and released the original input, before instance construction.
    Started,
    /// Body and output serialization completed successfully.
    Succeeded,
    /// Input validation, step body, or output serialization returned an error.
    Failed,
    /// Caller dropped a polled invocation before it completed.
    Cancelled,
    /// Invocation was dropped during stack unwinding. Input preparation panics
    /// can emit this event without a preceding `Started`.
    Panicked,
}

/// Event delivered synchronously to an observer, with no runner-owned history.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    /// Identity and dependencies.
    pub invocation: Invocation,
    /// Transition represented by this event.
    pub state: State,
    /// Nanoseconds since this executor was created (one monotonic clock origin).
    pub elapsed_ns: u64,
    /// Nanoseconds spent in this invocation so far.
    pub duration_ns: u64,
    /// Input on start or output on success, only when capture is explicitly enabled.
    pub payload: Option<Value>,
    /// Error text on failure. Step authors should exclude secrets.
    pub error: Option<String>,
}

/// Synchronous event consumer. Keep it short and nonblocking.
///
/// Stream or bound retained history in the consumer. The runner catches observer
/// panics and increments `LocalExecutor::observer_failures`; observation failures never
/// trigger a retry of side effects. Events may contain sensitive data if opted in.
pub trait Observer {
    /// Consume one transition without retaining references to it.
    fn observe(&self, event: &Event);
}

impl<F: Fn(&Event)> Observer for F {
    fn observe(&self, event: &Event) {
        self(event);
    }
}
