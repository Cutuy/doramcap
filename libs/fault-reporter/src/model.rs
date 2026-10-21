//! Transport-independent records. Durations use a source-local monotonic epoch.
pub use fault_schema::pb::{Mode, Outcome, Severity};
use std::time::Duration;

/// Fixed-size UTF-8 text; oversized values are rejected, never silently truncated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Text {
    bytes: [u8; 192],
    len: u8,
}
impl Text {
    /// Empty UTF-8 text, usable without construction or allocation.
    pub const EMPTY: Self = Self {
        bytes: [0; 192],
        len: 0,
    };
    /// Copy a complete UTF-8 string into fixed-capacity storage.
    ///
    /// # Errors
    /// Returns [`Error::TextTooLong`] when `value` exceeds 192 bytes. No allocation
    /// or truncation occurs.
    pub fn new(value: &str) -> Result<Self, Error> {
        if value.len() > 192 {
            return Err(Error::TextTooLong);
        }
        let mut result = Self::EMPTY;
        result.bytes[..value.len()].copy_from_slice(value.as_bytes());
        result.len = value.len() as u8;
        Ok(result)
    }
    /// Borrow the stored UTF-8 text without allocation.
    pub fn as_str(&self) -> &str {
        // Construction always copies complete UTF-8 strings.
        std::str::from_utf8(&self.bytes[..self.len as usize]).expect("valid Text")
    }
}
/// Configuration, registration or capture failure. Queue saturation is
/// reported separately as [`Admission::Dropped`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    TextTooLong,
    Capacity,
    Duplicate,
    ClockWentBackwards,
    SequenceExhausted,
    Closed,
    WrongMode,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

/// Source incarnation and registration-assigned condition ID.
///
/// `source` is nonzero and changes on process restart; condition zero is valid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    pub source: u128,
    pub condition: u32,
}
/// Optional caller-supplied trace/span correlation, copied without validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceContext {
    pub trace_id: [u8; 16],
    pub span_id: [u8; 8],
}
/// Condition semantics and optional publication limiting.
///
/// Default: stateful mode, one-second recovery, disabled deadline and success
/// limiting. Failure activates immediately. Recovery requires success evidence;
/// silence cannot clear health. Deadline expiry affects evidence freshness.
/// Event-only mode records independent failures and accepts no success reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy {
    pub mode: Mode,
    /// None disables deadlines. A gap greater than this resets success-run evidence.
    pub deadline: Option<Duration>,
    /// Default one second. Zero clears on the first success.
    pub recovery: Duration,
    /// None attempts to queue every success. Initial, recovery-confirming and
    /// deadline-gap successes bypass limiting, as does first success after failure.
    pub success_interval: Option<Duration>,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            mode: Mode::Stateful,
            deadline: None,
            recovery: Duration::from_secs(1),
            success_interval: None,
        }
    }
}
impl Policy {
    /// Check policy combinations without changing state.
    ///
    /// # Errors
    /// Returns `InvalidConfig` for unspecified mode, zero enabled deadline/interval,
    /// a success interval at or above the deadline, or deadline/rate limiting in
    /// event-only mode. Zero recovery is valid and clears on the first success.
    pub fn validate(self) -> Result<(), Error> {
        if self.mode == Mode::Unspecified
            || self.deadline.is_some_and(|v| v.is_zero())
            || self.success_interval.is_some_and(|v| v.is_zero())
            || matches!((self.success_interval, self.deadline), (Some(i), Some(d)) if i >= d)
            || (self.mode == Mode::EventOnly
                && (self.deadline.is_some() || self.success_interval.is_some()))
        {
            return Err(Error::InvalidConfig);
        }
        Ok(())
    }
}
/// Bounded condition identity, severity and immutable policy.
///
/// The semantic identity is `(component, resource, code)`. Change the source
/// incarnation to change a registered definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Definition {
    pub component: Text,
    pub resource: Text,
    pub code: Text,
    pub severity: Severity,
    pub policy: Policy,
}
impl Definition {
    /// Construct a definition with error severity and default stateful policy.
    ///
    /// `component` and `code` must be nonempty; `resource` may be empty.
    /// Configure public policy/severity fields before registration; registered
    /// definitions are immutable within a source incarnation.
    ///
    /// # Errors
    /// Returns `InvalidConfig` for empty component/code, or `TextTooLong`
    /// for any string exceeding 192 UTF-8 bytes. Performs no allocation.
    pub fn new(component: &str, resource: &str, code: &str) -> Result<Self, Error> {
        if component.is_empty() || code.is_empty() {
            return Err(Error::InvalidConfig);
        }
        Ok(Self {
            component: Text::new(component)?,
            resource: Text::new(resource)?,
            code: Text::new(code)?,
            severity: Severity::Error,
            policy: Policy::default(),
        })
    }
}
/// One sequenced observation with cumulative condition evidence.
///
/// `sequence` starts at one and equals `successes + failures`. Timestamps use
/// the source monotonic epoch. Counters include coalesced/dropped calls; queue
/// ordering can differ from sequence order. These records carry latest-state
/// evidence across loss and cannot reconstruct every missing transient event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {
    pub key: Key,
    pub sequence: u64,
    pub observed_at: Duration,
    pub outcome: Outcome,
    pub successes: u64,
    pub failures: u64,
    pub first_failure_at: Option<Duration>,
    pub last_failure_at: Option<Duration>,
    pub success_since: Option<Duration>,
    pub consecutive_successes: u64,
    /// Longest observed success-run span since the last failure, retained across deadline gaps.
    pub max_success_span: Duration,
    pub detail: Text,
    pub trace: Option<TraceContext>,
}
/// Source-lifetime counters, saturating at `u64::MAX`.
///
/// `failures`/`successes` count queue drops. `coalesced_successes` counts calls
/// folded into latest evidence by rate limiting; these are distinct from drops.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Losses {
    pub failures: u64,
    pub successes: u64,
    pub coalesced_successes: u64,
}
/// Disposition after evidence capture; none of these outcomes promises transport delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// Copied into a bounded queue.
    Queued,
    /// Retained as latest evidence for a later success summary.
    Coalesced,
    /// Queue full; latest evidence retained and reconciliation requested.
    Dropped,
}
/// Immutable definition plus optional latest evidence for one condition.
///
/// `latest` is `None` until the condition has accepted an observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConditionSnapshot {
    pub key: Key,
    pub definition: Definition,
    pub latest: Option<Observation>,
}
/// Source definitions/latest evidence and cumulative loss metadata.
///
/// Per-condition consistent; conditions can advance independently during copying.
/// `emitted_at` is source-clock time after capture. Missing entries never imply
/// recovery or deletion at the receiver.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub source: u128,
    pub emitted_at: Duration,
    pub conditions: Vec<ConditionSnapshot>,
    pub losses: Losses,
}
/// Ready observations and cumulative loss metadata for one source.
///
/// May be empty. `emitted_at` shares the observations' source-clock epoch;
/// observations may be out of sequence because of prioritized queue draining.
#[derive(Clone, Debug)]
pub struct Batch {
    pub source: u128,
    pub emitted_at: Duration,
    pub observations: Vec<Observation>,
    pub losses: Losses,
}

/// Return whether validated success evidence establishes healthy state.
///
/// True for success with no historical failures or a success span meeting
/// `policy.recovery`. Zero recovery permits the first success. This pure
/// predicate validates neither records nor policy; callers validate them first.
pub fn recovery_confirmed(observation: Observation, policy: Policy) -> bool {
    observation.outcome == Outcome::Success
        && (observation.failures == 0 || observation.max_success_span >= policy.recovery)
}
