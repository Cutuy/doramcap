//! Bounded success/failure reporting with explicit loss and transport-independent records.
//!
//! Callbacks serialize on a short per-condition mutex. They allocate no record storage
//! and perform no I/O. There is no lock-free or hard-real-time guarantee. One publisher
//! drains both outcomes fairly; applications own event-loop scheduling, transport and shutdown.
//!
//! # Synopsis
//! ```
//! use fault_reporter::{Config, Definition, Reporter, Text};
//! # fn main() -> Result<(), fault_reporter::Error> {
//! let (reporter, mut publisher) = Reporter::new(1, Config::default())?;
//! let check = reporter.register(Definition::new("recorder", "disk", "write_failed")?)?;
//! check.failure(Text::new("disk unavailable")?)?;
//! let registration = publisher.take_reconciliation().unwrap();
//! let ready = publisher.batch(64)?;
//! # Ok(())
//! # }
//! ```
//! The example source ID is illustrative; hosts supply a fresh ID per incarnation.
//! See [`Policy`] for hysteresis/deadlines, [`Check::observe`] for admission,
//! and [`Publisher`] for scheduling and shutdown.
mod model;
mod notification;
use crossbeam_queue::ArrayQueue;
pub use model::*;
pub use notification::Notification;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

/// Source-local monotonic time provider, shared across reporting handles.
///
/// Implementations must be fast, thread-safe and non-reentrant. Returned durations
/// are offsets from one stable epoch; wall-clock timestamps belong in transport metadata.
pub trait Clock: Send + Sync + 'static {
    /// Return a nondecreasing offset from this source clock's epoch.
    fn now(&self) -> Duration;
}
/// Elapsed time since construction, backed by [`Instant`].
pub struct MonotonicClock(Instant);
impl Default for MonotonicClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}
impl Clock for MonotonicClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
}
/// Reporter storage and drain bounds, fixed at construction.
///
/// Defaults: 256 conditions, 256 urgent slots per outcome, 256 repeat slots per
/// outcome, and eight failures before servicing a waiting success. All limits
/// must be nonzero; `max_conditions` must fit in `u32`.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Maximum registered conditions; registration storage is retained until drop.
    pub max_conditions: usize,
    /// Reserved urgent failure slots.
    pub failure_capacity: usize,
    /// Repeat slots per outcome, separate from urgent capacity.
    pub repeat_capacity: usize,
    /// Reserved urgent success slots, isolated from failure and repeat traffic.
    pub success_capacity: usize,
    /// At most this many failure records are drained before a waiting success.
    pub failure_burst: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            max_conditions: 256,
            failure_capacity: 256,
            repeat_capacity: 256,
            success_capacity: 256,
            failure_burst: 8,
        }
    }
}
struct Shared {
    source: u128,
    config: Config,
    clock: Arc<dyn Clock>,
    closed: AtomicBool,
    failures: ArrayQueue<Observation>,
    repeat_failures: ArrayQueue<Observation>,
    repeat_successes: ArrayQueue<Observation>,
    notification: Notification,
    dirty: AtomicBool,
    successes: ArrayQueue<Observation>,
    failure_loss: AtomicU64,
    success_loss: AtomicU64,
    coalesced: AtomicU64,
    conditions: Mutex<Vec<Arc<Condition>>>,
}
struct Condition {
    key: Key,
    definition: Definition,
    state: Mutex<Local>,
}
#[derive(Default)]
struct Local {
    latest: Option<Observation>,
    published_success_at: Option<Duration>,
    pending: bool,
}
/// Cloneable source owner used to register conditions and close reporting.
///
/// Clones share registration, queues, counters and closure state. Registration
/// allocates; report calls use fixed-capacity records. Methods using internal
/// mutexes can block and panic on poisoning. The host owns transport and scheduling.
#[derive(Clone)]
pub struct Reporter(Arc<Shared>);
/// Cloneable handle for one immutable registered condition.
///
/// Clones serialize reporting through the same condition lock. The owning
/// application decides how to recover from its original operation and reports
/// the outcome; capture does not change that operation's result.
#[derive(Clone)]
pub struct Check {
    shared: Arc<Shared>,
    condition: Arc<Condition>,
}
/// Single-consumer drain handle for the host event loop.
///
/// Owns no thread or I/O. Use notification hints and summary deadlines to schedule
/// draining. Publish definitions before observations, retain failed publications
/// for bounded host-managed retries, and snapshot on reconnect. Keep the handle
/// until closure, final draining and snapshot publication are complete.
pub struct Publisher {
    shared: Arc<Shared>,
    failure_run: usize,
}
fn increment(value: &AtomicU64) {
    let _ = value.try_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
        Some(v.saturating_add(1))
    });
}
impl Reporter {
    /// Create a source and its single-consumer drain handle.
    ///
    /// `source` is a nonzero ID unique to this process incarnation; choose a new ID
    /// on restart. `config` bounds registration and queue storage. Construction
    /// allocates queues and starts a source-local monotonic clock. No thread or I/O is created.
    ///
    /// # Errors
    /// Returns [`Error::InvalidConfig`] for a zero source, zero capacity/burst,
    /// or `max_conditions` greater than `u32::MAX`.
    pub fn new(source: u128, config: Config) -> Result<(Self, Publisher), Error> {
        Self::with_clock(source, config, Arc::new(MonotonicClock::default()))
    }
    /// Create a reporter using a caller-supplied monotonic clock.
    ///
    /// The reporter retains `clock` through an `Arc`. [`Clock::now`] runs under
    /// condition locks and must return promptly without reentering the reporter.
    /// For manually advanced clocks, notify the publisher after advancing time.
    ///
    /// # Errors
    /// Uses the same configuration validation as [`Self::new`].
    pub fn with_clock(
        source: u128,
        config: Config,
        clock: Arc<dyn Clock>,
    ) -> Result<(Self, Publisher), Error> {
        if source == 0
            || config.max_conditions == 0
            || config.max_conditions > u32::MAX as usize
            || config.repeat_capacity == 0
            || config.failure_capacity == 0
            || config.success_capacity == 0
            || config.failure_burst == 0
        {
            return Err(Error::InvalidConfig);
        }
        let shared = Arc::new(Shared {
            source,
            config,
            clock,
            closed: AtomicBool::new(false),
            failures: ArrayQueue::new(config.failure_capacity),
            repeat_failures: ArrayQueue::new(config.repeat_capacity),
            repeat_successes: ArrayQueue::new(config.repeat_capacity),
            notification: Notification::default(),
            dirty: AtomicBool::new(false),
            successes: ArrayQueue::new(config.success_capacity),
            failure_loss: AtomicU64::new(0),
            success_loss: AtomicU64::new(0),
            coalesced: AtomicU64::new(0),
            conditions: Mutex::new(Vec::with_capacity(config.max_conditions)),
        });
        Ok((
            Self(shared.clone()),
            Publisher {
                shared,
                failure_run: 0,
            },
        ))
    }
    /// Register an immutable condition and return a cloneable reporting handle.
    ///
    /// Uniqueness is `(component, resource, code)` within this source. Registration
    /// allocates condition storage, serializes on the registry lock, and requests
    /// reconciliation. Publish its definition before delivering its observations.
    ///
    /// # Errors
    /// Returns `InvalidConfig` for invalid policy, unspecified severity or empty
    /// component/code; `Closed` after closure; `Duplicate` for an existing identity;
    /// and `Capacity` when the registration limit is reached.
    pub fn register(&self, definition: Definition) -> Result<Check, Error> {
        definition.policy.validate()?;
        if definition.severity == Severity::Unspecified
            || definition.component.as_str().is_empty()
            || definition.code.as_str().is_empty()
        {
            return Err(Error::InvalidConfig);
        }
        let mut conditions = self.0.conditions.lock().unwrap();
        if self.0.closed.load(Ordering::Acquire) {
            return Err(Error::Closed);
        }
        if conditions.iter().any(|c| {
            c.definition.component == definition.component
                && c.definition.resource == definition.resource
                && c.definition.code == definition.code
        }) {
            return Err(Error::Duplicate);
        }
        if conditions.len() == self.0.config.max_conditions {
            return Err(Error::Capacity);
        }
        let condition = Arc::new(Condition {
            key: Key {
                source: self.0.source,
                condition: conditions.len() as u32,
            },
            definition,
            state: Mutex::new(Local::default()),
        });
        conditions.push(condition.clone());
        self.0.dirty.store(true, Ordering::Release);
        self.0.notification.notify();
        Ok(Check {
            shared: self.0.clone(),
            condition,
        })
    }
    /// Close this source, wake the publisher, and wait for in-flight reports.
    ///
    /// Closure affects every cloned reporter/check and is idempotent. This call
    /// may block on condition locks. The host still drains queues, captures
    /// [`Publisher::final_snapshot`], and applies its own transport flush deadline.
    pub fn close(&self) {
        self.0.closed.store(true, Ordering::Release);
        self.0.notification.notify();
        let conditions = self.0.conditions.lock().unwrap();
        for condition in conditions.iter() {
            drop(condition.state.lock().unwrap());
        }
    }
    /// Read cumulative queue-drop and success-coalescing counters.
    ///
    /// Counters saturate at `u64::MAX`. Reads are individually atomic; concurrent
    /// reports may fall between reads. No condition lock or allocation is required.
    pub fn losses(&self) -> Losses {
        self.0.losses()
    }
}
impl Shared {
    fn losses(&self) -> Losses {
        Losses {
            failures: self.failure_loss.load(Ordering::Relaxed),
            successes: self.success_loss.load(Ordering::Relaxed),
            coalesced_successes: self.coalesced.load(Ordering::Relaxed),
        }
    }
    fn enqueue(&self, observation: Observation, urgent: bool) -> Admission {
        let (queue, counter) = match observation.outcome {
            Outcome::Failure => (
                if urgent {
                    &self.failures
                } else {
                    &self.repeat_failures
                },
                &self.failure_loss,
            ),
            Outcome::Success => (
                if urgent {
                    &self.successes
                } else {
                    &self.repeat_successes
                },
                &self.success_loss,
            ),
            Outcome::Unspecified => unreachable!("validated outcome"),
        };
        let admission = if queue.push(observation).is_ok() {
            Admission::Queued
        } else {
            increment(counter);
            self.dirty.store(true, Ordering::Release);
            Admission::Dropped
        };
        self.notification.notify();
        admission
    }
}
impl Check {
    /// Return the stable source/condition identity assigned at registration.
    pub fn key(&self) -> Key {
        self.condition.key
    }
    /// Report success with empty diagnostic text and no trace context.
    ///
    /// # Returns
    /// Returns the admission disposition described by [`Self::observe`].
    ///
    /// # Errors
    /// Uses [`Self::observe`] validation; event-only conditions return `WrongMode`.
    pub fn success(&self) -> Result<Admission, Error> {
        self.observe(Outcome::Success, Text::EMPTY, None)
    }
    /// Report failure with bounded diagnostic text and no trace context.
    ///
    /// `detail` is copied into the observation. Stateful failures reset recovery;
    /// event-only failures represent individual occurrences.
    ///
    /// # Returns
    /// Returns the admission disposition described by [`Self::observe`].
    ///
    /// # Errors
    /// Uses [`Self::observe`] validation.
    pub fn failure(&self, detail: Text) -> Result<Admission, Error> {
        self.observe(Outcome::Failure, detail, None)
    }
    /// Record an outcome, diagnostic text, and optional trace correlation.
    ///
    /// Each accepted call advances the condition sequence and cumulative evidence
    /// before queue admission. Initial observations, outcome changes, recovery
    /// confirmation, deadline-gap successes and event-only failures use reserved
    /// urgent capacity. Repeated successes may be coalesced by policy.
    ///
    /// # Returns
    /// Returns [`Admission::Queued`], [`Admission::Coalesced`], or
    /// [`Admission::Dropped`]. Dropped records retain latest evidence, increment
    /// loss counters, and request reconciliation. Admission does not imply delivery.
    ///
    /// # Errors
    /// Returns `InvalidConfig` for an unspecified outcome; `Closed` after closure;
    /// `WrongMode` for event-only success; `ClockWentBackwards` for a regressing
    /// condition clock; or `SequenceExhausted` at the sequence limit.
    /// These errors leave that condition's observation evidence unchanged.
    ///
    /// # Concurrency
    /// Cloned checks serialize on a per-condition mutex. Capture allocates no
    /// record storage and performs no I/O; it can wait for another report or snapshot.
    /// Lock-based methods panic if an internal mutex has been poisoned.
    pub fn observe(
        &self,
        outcome: Outcome,
        detail: Text,
        trace: Option<TraceContext>,
    ) -> Result<Admission, Error> {
        if outcome == Outcome::Unspecified {
            return Err(Error::InvalidConfig);
        }
        let mut local = self.condition.state.lock().unwrap();
        if self.shared.closed.load(Ordering::Acquire) {
            return Err(Error::Closed);
        }
        let policy = self.condition.definition.policy;
        if policy.mode == Mode::EventOnly && outcome == Outcome::Success {
            return Err(Error::WrongMode);
        }
        let now = self.shared.clock.now();
        let previous = local.latest;
        if previous.is_some_and(|p| p.observed_at > now) {
            return Err(Error::ClockWentBackwards);
        }
        let sequence = previous
            .map_or(Some(1), |p| p.sequence.checked_add(1))
            .ok_or(Error::SequenceExhausted)?;
        let continuous_success = previous.is_some_and(|p| {
            p.outcome == Outcome::Success
                && !policy.deadline.is_some_and(|d| now - p.observed_at > d)
        });
        let successes =
            previous.map_or(0, |p| p.successes) + u64::from(outcome == Outcome::Success);
        let failures = previous.map_or(0, |p| p.failures) + u64::from(outcome == Outcome::Failure);
        let success_since = if outcome == Outcome::Failure {
            None
        } else if continuous_success {
            previous.and_then(|p| p.success_since)
        } else {
            Some(now)
        };
        let max_success_span = if outcome == Outcome::Failure {
            Duration::ZERO
        } else {
            previous
                .map_or(Duration::ZERO, |p| p.max_success_span)
                .max(now - success_since.unwrap())
        };
        let observation = Observation {
            key: self.condition.key,
            sequence,
            observed_at: now,
            outcome,
            successes,
            failures,
            first_failure_at: previous.and_then(|p| p.first_failure_at).or(
                if outcome == Outcome::Failure {
                    Some(now)
                } else {
                    None
                },
            ),
            last_failure_at: if outcome == Outcome::Failure {
                Some(now)
            } else {
                previous.and_then(|p| p.last_failure_at)
            },
            success_since,
            max_success_span,
            consecutive_successes: if outcome == Outcome::Failure {
                0
            } else if continuous_success {
                previous.unwrap().consecutive_successes + 1
            } else {
                1
            },
            detail,
            trace,
        };
        let urgent = previous.is_none_or(|p| p.outcome != outcome)
            || policy.mode == Mode::EventOnly
            || (outcome == Outcome::Success && !continuous_success)
            || (recovery_confirmed(observation, policy)
                && previous.is_some_and(|p| !recovery_confirmed(p, policy)));
        local.latest = Some(observation);
        if !urgent
            && outcome == Outcome::Success
            && continuous_success
            && policy.success_interval.is_some_and(|i| {
                local
                    .published_success_at
                    .is_some_and(|last| now.saturating_sub(last) < i)
            })
        {
            if !local.pending {
                self.shared.notification.notify();
            }
            local.pending = true;
            increment(&self.shared.coalesced);
            return Ok(Admission::Coalesced);
        }
        local.pending = false;
        if outcome == Outcome::Success {
            local.published_success_at = Some(now);
        }
        Ok(self.shared.enqueue(observation, urgent))
    }
}
impl Publisher {
    /// Clone the publisher wake handle for host event-loop integration.
    ///
    /// Capture its generation before checking work. One logical consumer owns
    /// queue draining; wake hints may be coalesced or stale. See [`Notification`].
    pub fn notification(&self) -> Notification {
        self.shared.notification.clone()
    }
    /// Return whether registration or queue overflow requests a snapshot.
    ///
    /// This concurrent hint does not clear the request. Use [`Self::take_reconciliation`].
    pub fn reconciliation_pending(&self) -> bool {
        self.shared.dirty.load(Ordering::Acquire)
    }
    /// Take a requested definition/latest-evidence snapshot, or return `None`.
    ///
    /// Clears the request before copying so concurrent overflow can request another
    /// snapshot. Allocates and locks as [`Self::snapshot`] does. The host must retain
    /// and retry the returned value until publication succeeds; taking it is not an acknowledgment.
    pub fn take_reconciliation(&self) -> Option<Snapshot> {
        self.shared
            .dirty
            .swap(false, Ordering::AcqRel)
            .then(|| self.snapshot())
    }
    /// Return whether closure has started.
    ///
    /// Use [`Reporter::close`] completion to establish that all in-flight calls finished.
    pub fn is_closed(&self) -> bool {
        self.shared.closed.load(Ordering::Acquire)
    }
    /// Return whether any urgent or repeat lane currently contains records.
    ///
    /// Excludes pending summaries and reconciliation. Producers can change this hint
    /// immediately; combine it with notification generations when deciding to wait.
    pub fn has_records(&self) -> bool {
        !self.shared.failures.is_empty()
            || !self.shared.successes.is_empty()
            || !self.shared.repeat_failures.is_empty()
            || !self.shared.repeat_successes.is_empty()
    }
    /// Return the earliest pending summary deadline in source-clock time.
    ///
    /// Returns `None` when no summary is pending. Scans registered conditions under
    /// locks. Schedule [`Self::tick`] at this time; a timer flush preserves the
    /// original observation timestamp and cannot establish new recovery evidence.
    pub fn next_summary_at(&self) -> Option<Duration> {
        self.shared
            .conditions
            .lock()
            .unwrap()
            .iter()
            .filter_map(|c| {
                let local = c.state.lock().unwrap();
                if !local.pending {
                    return None;
                }
                Some(
                    local
                        .published_success_at?
                        .saturating_add(c.definition.policy.success_interval?),
                )
            })
            .min()
    }
    /// Block until a wake hint or timeout, scheduling pending summary flushes.
    ///
    /// Returns immediately for ready records, reconciliation, or closure. Otherwise
    /// waits for at most `timeout` or the nearest summary deadline, then calls
    /// [`Self::tick`]. Wake hints may return early; recheck work after returning.
    ///
    /// # Scheduling
    /// This optional blocking helper creates no thread. Hosts using an existing
    /// event loop can use [`Self::notification`], [`Self::next_summary_at`] and
    /// nonblocking queue drains instead. Custom clocks require host advancement
    /// and notification. Timeouts are subject to OS scheduling and lock contention.
    pub fn wait(&mut self, timeout: Duration) {
        let seen = self.shared.notification.generation();
        self.tick();
        if self.has_records() || self.reconciliation_pending() || self.is_closed() {
            return;
        }
        let timeout = self.next_summary_at().map_or(timeout, |at| {
            timeout.min(at.saturating_sub(self.shared.clock.now()))
        });
        self.shared.notification.wait_since(seen, timeout);
        self.tick();
    }
    /// Attempt to enqueue success summaries whose source-clock deadline is due.
    ///
    /// Scans and locks registered conditions. Queue saturation increments losses
    /// and requests reconciliation; latest evidence remains available in snapshots.
    /// Creates no observations and does not refresh observation timestamps.
    pub fn tick(&mut self) {
        let now = self.shared.clock.now();
        let conditions = self.shared.conditions.lock().unwrap();
        for condition in conditions.iter() {
            let mut local = condition.state.lock().unwrap();
            if local.pending
                && condition
                    .definition
                    .policy
                    .success_interval
                    .is_some_and(|i| {
                        local
                            .published_success_at
                            .is_some_and(|last| now.saturating_sub(last) >= i)
                    })
            {
                self.shared.enqueue(local.latest.unwrap(), false);
                local.pending = false;
                local.published_success_at = Some(now);
            }
        }
    }
    /// Remove one ready observation, or return `None` when queues are empty.
    ///
    /// Urgent records overtake repeats within each outcome. After `failure_burst`
    /// failures, a waiting success gets a turn. Delivery can reorder sequences;
    /// receivers must reconcile by identity and sequence. This call does not wait,
    /// allocate, flush pending summaries, or clear reconciliation requests.
    pub fn try_recv(&mut self) -> Option<Observation> {
        if self.failure_run >= self.shared.config.failure_burst
            && let Some(record) = self
                .shared
                .successes
                .pop()
                .or_else(|| self.shared.repeat_successes.pop())
        {
            self.failure_run = 0;
            return Some(record);
        }
        if let Some(record) = self
            .shared
            .failures
            .pop()
            .or_else(|| self.shared.repeat_failures.pop())
        {
            self.failure_run = self.failure_run.saturating_add(1);
            return Some(record);
        }
        self.failure_run = 0;
        self.shared
            .successes
            .pop()
            .or_else(|| self.shared.repeat_successes.pop())
    }
    /// Flush due summaries and drain at most `limit` ready observations.
    ///
    /// Allocates a vector bounded by `limit`; returns immediately without a
    /// batch-fill delay. Empty and single-record batches are valid. The returned
    /// batch includes source-clock emission time and cumulative loss counters.
    ///
    /// # Errors
    /// Returns `InvalidConfig` if `limit` is zero or exceeds total configured
    /// urgent and repeat queue capacity.
    pub fn batch(&mut self, limit: usize) -> Result<Batch, Error> {
        if limit == 0
            || limit
                > self
                    .shared
                    .config
                    .failure_capacity
                    .saturating_add(self.shared.config.success_capacity)
                    .saturating_add(self.shared.config.repeat_capacity.saturating_mul(2))
        {
            return Err(Error::InvalidConfig);
        }
        self.tick();
        let mut observations = Vec::with_capacity(limit);
        while observations.len() < limit {
            let Some(record) = self.try_recv() else { break };
            observations.push(record);
        }
        Ok(Batch {
            source: self.shared.source,
            emitted_at: self.shared.clock.now(),
            observations,
            losses: self.shared.losses(),
        })
    }
    /// Copy registered definitions, latest evidence and cumulative losses.
    ///
    /// Allocates storage bounded by `max_conditions` and locks the registry and each
    /// condition. Each condition is consistent; different conditions can represent
    /// different instants. Original observation timestamps are preserved. This call
    /// neither drains queues nor clears reconciliation requests. Missing transient
    /// history remains visible through sequences and losses.
    pub fn snapshot(&self) -> Snapshot {
        let conditions = self
            .shared
            .conditions
            .lock()
            .unwrap()
            .iter()
            .map(|c| ConditionSnapshot {
                key: c.key,
                definition: c.definition,
                latest: c.state.lock().unwrap().latest,
            })
            .collect();
        Snapshot {
            source: self.shared.source,
            emitted_at: self.shared.clock.now(),
            conditions,
            losses: self.shared.losses(),
        }
    }
    /// Copy final evidence after source closure, including pending summaries.
    ///
    /// Call after [`Reporter::close`] returns. Allocation and locking follow
    /// [`Self::snapshot`]; queue records remain available for draining.
    ///
    /// # Errors
    /// Returns `InvalidConfig` if closure has not started.
    pub fn final_snapshot(&self) -> Result<Snapshot, Error> {
        if !self.shared.closed.load(Ordering::Acquire) {
            return Err(Error::InvalidConfig);
        }
        Ok(self.snapshot())
    }
}
