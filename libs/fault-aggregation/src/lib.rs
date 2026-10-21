//! Bounded, deterministic fault aggregation. Applications supply monotonic time,
//! source lifecycle, event-loop scheduling and transport. No background threads or I/O.
//!
//! # Synopsis
//! ```
//! use fault_aggregation::{Aggregator, Config, FaultObservability};
//! use std::sync::Arc;
//! # fn main() -> Result<(), fault_aggregation::Error> {
//! let diagnostics = Arc::new(FaultObservability::new());
//! let mut agg = Aggregator::new(1, Config::default(), diagnostics.clone())?;
//! let view = agg.snapshot();
//! let counters = diagnostics.stats();
//! # Ok(())
//! # }
//! ```
//! Supply a fresh epoch and diagnostics instance after restart. Register producer
//! snapshots via [`Aggregator::reconcile`], ingest observations, schedule deadlines,
//! and drain domain updates from the host event loop.
use fault_reporter::{
    Batch, ConditionSnapshot, Definition, Key, Losses, Mode, Observation, Outcome, Snapshot,
};
pub use fault_schema::pb::{Freshness, Health, UpdateKind};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
pub mod observability;
pub use observability::{FaultObservability, Stats};

/// Registry, retention and freshness bounds fixed at construction.
///
/// Defaults: 64 sources, 256 conditions/source, 512 observations/batch, 1,024
/// retained updates, sequence-distance deduplication window 128, and zero
/// transport allowance. All capacities/windows must be nonzero.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub max_sources: usize,
    pub max_conditions_per_source: usize,
    pub max_batch_records: usize,
    pub output_capacity: usize,
    /// Sequence distance retained for exact duplicate rejection of late records.
    pub dedup_window: usize,
    /// Upper bound assumed for publication-to-receive delay. Counts against deadlines.
    /// Zero assumes negligible delay; choose a conservative bound for the transport.
    pub transport_allowance: Duration,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            max_sources: 64,
            max_conditions_per_source: 256,
            max_batch_records: 512,
            output_capacity: 1024,
            dedup_window: 128,
            transport_allowance: Duration::ZERO,
        }
    }
}
/// Registration, evidence validation or host-clock failure.
///
/// Batch/snapshot operations can apply an earlier prefix before returning an error;
/// see [`Aggregator::ingest_batch`] and [`Aggregator::reconcile`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    Capacity,
    UnknownCondition,
    UnknownSource,
    DefinitionConflict,
    InvalidObservation,
    ClockWentBackwards,
    RevisionExhausted,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
/// Outcome of one validated observation submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ingest {
    /// Newest evidence applied to current state.
    Applied,
    /// Sequence already retained; no domain update emitted.
    Duplicate,
    /// Older evidence emitted as history without regressing state.
    Late,
    /// Outside the deduplication window; counted without emitting history.
    TooOld,
}
/// Current condition evidence with independent health and freshness.
///
/// `revision` identifies the latest emitted condition update; late history can
/// advance it while preserving `latest`. Event-only health remains unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub key: Key,
    pub definition: Definition,
    pub health: Health,
    pub freshness: Freshness,
    pub latest: Option<Observation>,
    pub revision: u64,
}
/// Revisioned domain output for live consumers and recording.
///
/// `epoch`/`revision` identify output ordering. Source-wide updates omit key/state;
/// condition updates include both. `observation` preserves the triggering record,
/// including late evidence that did not replace current state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Update {
    pub epoch: u128,
    pub revision: u64,
    pub kind: UpdateKind,
    pub key: Option<Key>,
    pub state: Option<State>,
    /// Late records remain historical evidence without replacing latest state.
    pub observation: Option<Observation>,
    pub source: u128,
    pub connected: bool,
    pub losses: Losses,
}
/// Current source connectivity, cumulative producer losses and conditions.
#[derive(Clone, Debug)]
pub struct SourceView {
    pub source: u128,
    pub connected: bool,
    pub losses: Losses,
    pub conditions: Vec<State>,
}
/// Domain snapshot with epoch/revision watermark and captured diagnostics.
///
/// Retains current state after output loss. Excludes deduplication windows and
/// expiry bookkeeping; use producer snapshots to rebuild runtime state.
#[derive(Clone, Debug)]
pub struct View {
    pub epoch: u128,
    pub revision: u64,
    /// Captured diagnostics metadata retained in stored snapshots.
    pub stats: Stats,
    pub sources: Vec<SourceView>,
}
struct Entry {
    state: State,
    expires_at: Option<Duration>,
    seen: VecDeque<u64>,
}
struct Source {
    connected: bool,
    losses: Losses,
    conditions: BTreeMap<u32, Entry>,
}
/// Deterministic fault-state owner with host-injected diagnostics.
///
/// # Clock contract
/// All `now` arguments share a host monotonic epoch and must be nondecreasing.
/// Time-taking mutators return `ClockWentBackwards` for a regression and
/// `RevisionExhausted` at the output revision limit. Calls can advance the clock
/// before another validation fails. Source emission times use independent epochs.
///
/// # Ownership and output
/// Mutators require exclusive access; the host owns synchronization and scheduling.
/// No background thread or I/O is created. Output overflow drops oldest history,
/// increments injected diagnostics, and requests reconciliation. Read diagnostics
/// through the injected [`FaultObservability`]. Condition policy governs health
/// and freshness independently of those operational counters.
pub struct Aggregator {
    config: Config,
    epoch: u128,
    revision: u64,
    now: Duration,
    sources: BTreeMap<u128, Source>,
    outputs: VecDeque<Update>,
    notification: fault_reporter::Notification,
    reconciliation_pending: bool,
    observability: Arc<FaultObservability>,
}
impl Aggregator {
    /// Clone the wake handle signalled whenever a domain update is queued.
    ///
    /// Capture its generation before checking/draining output. One logical consumer
    /// owns publication. If the host protects this aggregator with a lock, release
    /// that lock before waiting; schedule [`Self::advance`] at [`Self::next_deadline`].
    pub fn notification(&self) -> fault_reporter::Notification {
        self.notification.clone()
    }
    /// Return whether retained domain updates are ready for draining.
    ///
    /// This does not include a pending reconciliation snapshot.
    pub fn has_updates(&self) -> bool {
        !self.outputs.is_empty()
    }
    /// Return a full view after output overflow, clearing that request.
    ///
    /// Returns `None` when no request is pending. Allocation follows [`Self::snapshot`].
    /// The host retains/retries the returned view until publication succeeds.
    /// A view restores current state; dropped historical updates remain missing.
    pub fn take_reconciliation(&mut self) -> Option<View> {
        if !self.reconciliation_pending {
            return None;
        }
        self.reconciliation_pending = false;
        Some(self.snapshot())
    }
    /// Return the next representable freshness-expiry time on the host clock.
    ///
    /// Expiry is strictly after the evidence deadline; the result includes one
    /// nanosecond to cross that boundary. Returns `None` when no condition needs
    /// expiry. Call [`Self::advance`] at this time even when ingress is idle.
    pub fn next_deadline(&self) -> Option<Duration> {
        self.sources
            .values()
            .flat_map(|s| s.conditions.values())
            .filter(|e| e.state.freshness != Freshness::Stale)
            .filter_map(|e| {
                e.expires_at
                    .and_then(|t| t.checked_add(Duration::from_nanos(1)))
            })
            .min()
    }
    /// Create an aggregator with bounded state and injected diagnostics.
    ///
    /// `epoch` is a nonzero ID that changes on restart. Supply a fresh
    /// [`FaultObservability`] per epoch and retain a clone to read diagnostics.
    /// Construction allocates output storage. The host owns scheduling and I/O.
    ///
    /// # Errors
    /// Returns `InvalidConfig` for a zero epoch or any zero capacity/deduplication window.
    pub fn new(
        epoch: u128,
        config: Config,
        observability: Arc<FaultObservability>,
    ) -> Result<Self, Error> {
        if epoch == 0
            || config.max_sources == 0
            || config.max_conditions_per_source == 0
            || config.max_batch_records == 0
            || config.output_capacity == 0
            || config.dedup_window == 0
        {
            return Err(Error::InvalidConfig);
        }
        Ok(Self {
            config,
            epoch,
            revision: 0,
            now: Duration::ZERO,
            sources: BTreeMap::new(),
            outputs: VecDeque::with_capacity(config.output_capacity),
            notification: fault_reporter::Notification::default(),
            reconciliation_pending: false,
            observability,
        })
    }
    fn clock(&mut self, now: Duration) -> Result<(), Error> {
        if now < self.now {
            return Err(Error::ClockWentBackwards);
        }
        if self.revision == u64::MAX {
            return Err(Error::RevisionExhausted);
        }
        self.now = now;
        Ok(())
    }
    fn emit(
        &mut self,
        source_id: u128,
        key: Option<Key>,
        kind: UpdateKind,
        observation: Option<Observation>,
    ) {
        // Mutators reserve enough revision space before changing state.
        self.revision += 1;
        let source = self.sources.get_mut(&source_id).unwrap();
        let state = key.map(|k| {
            let entry = source.conditions.get_mut(&k.condition).unwrap();
            entry.state.revision = self.revision;
            entry.state
        });
        let update = Update {
            epoch: self.epoch,
            revision: self.revision,
            kind,
            key,
            state,
            observation,
            source: source_id,
            connected: source.connected,
            losses: source.losses,
        };
        if self.outputs.len() == self.config.output_capacity {
            self.outputs.pop_front();
            self.reconciliation_pending = true;
            self.observability.dropped_updates();
        }
        self.outputs.push_back(update);
        self.notification.notify();
    }
    fn reserve_revisions(&self, count: usize) -> Result<(), Error> {
        if self.revision.checked_add(count as u64).is_none() {
            Err(Error::RevisionExhausted)
        } else {
            Ok(())
        }
    }
    /// Register a condition definition under its source/condition key.
    ///
    /// An identical existing definition is idempotent. New conditions begin with
    /// unknown health/freshness and emit `Registered`. A configured deadline starts
    /// at `now`, including before the first observation. New sources begin connected.
    /// Registration allocates bounded registry and deduplication storage.
    ///
    /// # Errors
    /// Returns `InvalidConfig` for invalid identity/definition/policy;
    /// `DefinitionConflict` for a changed definition or duplicate semantic identity;
    /// `Capacity` for registry limits; and the clock/revision errors described on
    /// [`Aggregator`].
    pub fn register(
        &mut self,
        key: Key,
        definition: Definition,
        now: Duration,
    ) -> Result<(), Error> {
        self.clock(now)?;
        definition
            .policy
            .validate()
            .map_err(|_| Error::InvalidConfig)?;
        if key.source == 0
            || definition.severity == fault_reporter::Severity::Unspecified
            || definition.component.as_str().is_empty()
            || definition.code.as_str().is_empty()
        {
            return Err(Error::InvalidConfig);
        }
        if let Some(source) = self.sources.get(&key.source) {
            if let Some(entry) = source.conditions.get(&key.condition) {
                return if entry.state.definition == definition {
                    Ok(())
                } else {
                    Err(Error::DefinitionConflict)
                };
            }
            if source.conditions.len() >= self.config.max_conditions_per_source {
                self.observability.rejected_inputs();
                return Err(Error::Capacity);
            }
            if source.conditions.values().any(|e| {
                e.state.definition.component == definition.component
                    && e.state.definition.resource == definition.resource
                    && e.state.definition.code == definition.code
            }) {
                return Err(Error::DefinitionConflict);
            }
        } else if self.sources.len() >= self.config.max_sources {
            self.observability.rejected_inputs();
            return Err(Error::Capacity);
        }
        let source = self.sources.entry(key.source).or_insert_with(|| Source {
            connected: true,
            losses: Losses::default(),
            conditions: BTreeMap::new(),
        });
        source.conditions.insert(
            key.condition,
            Entry {
                state: State {
                    key,
                    definition,
                    health: Health::Unknown,
                    freshness: Freshness::Unknown,
                    latest: None,
                    revision: 0,
                },
                expires_at: definition.policy.deadline.map(|d| now.saturating_add(d)),
                seen: VecDeque::with_capacity(self.config.dedup_window),
            },
        );
        self.emit(key.source, Some(key), UpdateKind::Registered, None);
        Ok(())
    }
    fn valid(o: Observation, emitted_at: Duration, mode: Mode) -> bool {
        o.sequence > 0
            && o.observed_at <= emitted_at
            && o.successes.checked_add(o.failures) == Some(o.sequence)
            && (mode != Mode::EventOnly || o.successes == 0)
            && match (o.first_failure_at, o.last_failure_at) {
                (None, None) => o.failures == 0,
                (Some(first), Some(last)) => {
                    o.failures > 0 && first <= last && last <= o.observed_at
                }
                _ => false,
            }
            && match o.outcome {
                Outcome::Unspecified => false,
                Outcome::Failure => {
                    o.failures > 0
                        && o.success_since.is_none()
                        && o.consecutive_successes == 0
                        && o.max_success_span.is_zero()
                        && o.last_failure_at == Some(o.observed_at)
                }
                Outcome::Success => {
                    mode == Mode::Stateful
                        && o.successes > 0
                        && o.consecutive_successes > 0
                        && o.consecutive_successes <= o.successes
                        && o.max_success_span <= o.observed_at
                        && o.success_since.is_some_and(|s| {
                            s <= o.observed_at
                                && o.max_success_span >= o.observed_at - s
                                && (o.consecutive_successes != 1 || s == o.observed_at)
                        })
                }
            }
    }
    /// Apply one observation using producer emission time and host receive time.
    ///
    /// `emitted_at` shares the observation's source-clock epoch. `now` uses the
    /// aggregator's host-clock epoch. Evidence age is emission minus observation
    /// time plus configured transport allowance. These clock epochs are independent.
    ///
    /// Newest evidence updates state. Duplicate sequences have no domain effect.
    /// Late observations within the deduplication window emit history without
    /// regressing state; older observations increment the `too_old` diagnostic.
    /// Sequence gaps emit reconciliation updates. Silence never confirms recovery.
    ///
    /// # Returns
    /// Returns [`Ingest`] describing application, duplication, late history or rejection
    /// outside the retained sequence window.
    ///
    /// # Errors
    /// Returns `UnknownSource`, `UnknownCondition`, or `InvalidObservation` for
    /// invalid/inconsistent evidence, plus the common clock/revision errors.
    /// Overflow of output retention drops the oldest update and requests a snapshot;
    /// it does not reject otherwise valid input.
    pub fn ingest(
        &mut self,
        observation: Observation,
        emitted_at: Duration,
        now: Duration,
    ) -> Result<Ingest, Error> {
        self.apply(observation, emitted_at, now, false)
    }
    fn apply(
        &mut self,
        o: Observation,
        emitted_at: Duration,
        now: Duration,
        snapshot: bool,
    ) -> Result<Ingest, Error> {
        self.clock(now)?;
        let source = self
            .sources
            .get_mut(&o.key.source)
            .ok_or(Error::UnknownSource)?;
        let entry = source
            .conditions
            .get_mut(&o.key.condition)
            .ok_or(Error::UnknownCondition)?;
        let policy = entry.state.definition.policy;
        if !Self::valid(o, emitted_at, policy.mode) {
            self.observability.rejected_inputs();
            return Err(Error::InvalidObservation);
        }
        let previous = entry.state.latest;
        if let Some(p) = previous {
            if o.sequence == p.sequence || entry.seen.contains(&o.sequence) {
                return Ok(Ingest::Duplicate);
            }
            if o.sequence < p.sequence {
                if p.sequence - o.sequence >= self.config.dedup_window as u64 {
                    self.observability.too_old();
                    return Ok(Ingest::TooOld);
                }
                if o.observed_at > p.observed_at
                    || o.failures > p.failures
                    || o.successes > p.successes
                {
                    self.observability.rejected_inputs();
                    return Err(Error::InvalidObservation);
                }
                entry.seen.push_back(o.sequence);
                self.emit(o.key.source, Some(o.key), UpdateKind::Late, Some(o));
                return Ok(Ingest::Late);
            }
            if o.observed_at < p.observed_at || o.failures < p.failures || o.successes < p.successes
            {
                self.observability.rejected_inputs();
                return Err(Error::InvalidObservation);
            }
        }
        entry
            .seen
            .retain(|s| o.sequence - *s < self.config.dedup_window as u64);
        entry.seen.push_back(o.sequence);
        let before = entry.state.health;
        if policy.mode == Mode::Stateful {
            entry.state.health = match o.outcome {
                Outcome::Unspecified => unreachable!("validated observation"),
                Outcome::Failure => Health::Active,
                Outcome::Success => {
                    if fault_reporter::recovery_confirmed(o, policy) {
                        Health::Healthy
                    } else {
                        Health::Recovering
                    }
                }
            };
        }
        let age = emitted_at
            .saturating_sub(o.observed_at)
            .saturating_add(self.config.transport_allowance);
        entry.expires_at = policy
            .deadline
            .map(|d| now.saturating_add(d.saturating_sub(age)));
        entry.state.freshness = if !source.connected || policy.deadline.is_some_and(|d| age > d) {
            Freshness::Stale
        } else {
            Freshness::Fresh
        };
        entry.state.latest = Some(o);
        let gap = o.sequence > previous.map_or(0, |p| p.sequence).saturating_add(1);
        let kind = if snapshot || gap {
            UpdateKind::Reconciled
        } else if policy.mode == Mode::EventOnly {
            UpdateKind::Event
        } else if before != entry.state.health {
            UpdateKind::Transition
        } else {
            UpdateKind::Observation
        };
        self.emit(o.key.source, Some(o.key), kind, Some(o));
        Ok(Ingest::Applied)
    }
    /// Apply observations in batch order, then merge cumulative source losses.
    ///
    /// `now` is host monotonic time. The batch supplies source emission time.
    /// An empty batch still requires a registered source. Accepted entries are
    /// idempotent on retry; a later error leaves earlier entries applied. Source
    /// losses are merged only after every observation succeeds.
    ///
    /// # Errors
    /// Returns `InvalidObservation` for excessive batch length or mismatched source
    /// IDs, errors from [`Self::ingest`], or `UnknownSource` for loss metadata.
    /// Also checks common clock/revision limits before processing.
    pub fn ingest_batch(&mut self, batch: &Batch, now: Duration) -> Result<(), Error> {
        self.clock(now)?;
        if batch.observations.len() > self.config.max_batch_records
            || batch
                .observations
                .iter()
                .any(|o| o.key.source != batch.source)
        {
            return Err(Error::InvalidObservation);
        }
        self.reserve_revisions(batch.observations.len().saturating_add(1))?;
        for observation in &batch.observations {
            self.ingest(*observation, batch.emitted_at, now)?;
        }
        self.update_losses(batch.source, batch.losses)?;
        Ok(())
    }
    fn update_losses(&mut self, id: u128, losses: Losses) -> Result<(), Error> {
        let source = self.sources.get_mut(&id).ok_or(Error::UnknownSource)?;
        let old = source.losses;
        source.losses.failures = old.failures.max(losses.failures);
        source.losses.successes = old.successes.max(losses.successes);
        source.losses.coalesced_successes = old.coalesced_successes.max(losses.coalesced_successes);
        if old != source.losses {
            self.emit(id, None, UpdateKind::Source, None);
        }
        Ok(())
    }
    /// Merge a producer snapshot by condition identity and sequence.
    ///
    /// Registers definitions, applies latest evidence, and merges cumulative losses.
    /// Absent conditions remain registered. Duplicate evidence cannot extend freshness.
    /// Empty snapshots can register a source. New sources begin connected; an existing
    /// disconnected source requires an explicit [`Self::set_connected`] call.
    /// A later error can leave earlier registrations/observations applied; retry safely.
    ///
    /// # Errors
    /// Returns `InvalidObservation` for malformed identity/cardinality, `Capacity`
    /// for registry limits, definition/observation errors from registration and ingest,
    /// and common clock/revision errors.
    pub fn reconcile(&mut self, snapshot: &Snapshot, now: Duration) -> Result<(), Error> {
        self.clock(now)?;
        if snapshot.source == 0
            || snapshot.conditions.len() > self.config.max_conditions_per_source
            || snapshot.conditions.iter().any(|c| {
                c.key.source != snapshot.source || c.latest.is_some_and(|o| o.key != c.key)
            })
        {
            return Err(Error::InvalidObservation);
        }
        self.reserve_revisions(
            snapshot
                .conditions
                .len()
                .saturating_mul(2)
                .saturating_add(1),
        )?;
        // Empty snapshots are useful for sources with no checks yet.
        if !self.sources.contains_key(&snapshot.source) {
            if self.sources.len() >= self.config.max_sources {
                self.observability.rejected_inputs();
                return Err(Error::Capacity);
            }
            self.sources.insert(
                snapshot.source,
                Source {
                    connected: true,
                    losses: Losses::default(),
                    conditions: BTreeMap::new(),
                },
            );
        }
        for ConditionSnapshot {
            key,
            definition,
            latest,
        } in &snapshot.conditions
        {
            self.register(*key, *definition, now)?;
            if let Some(o) = latest {
                self.apply(*o, snapshot.emitted_at, now, true)?;
            }
        }
        self.update_losses(snapshot.source, snapshot.losses)?;
        Ok(())
    }
    /// Expire evidence deadlines using host monotonic time `now`.
    ///
    /// Marks overdue evidence stale and emits freshness updates. Health remains
    /// unchanged, including active/recovering faults. Allocates a temporary key list
    /// bounded by registered conditions. Schedule using [`Self::next_deadline`].
    ///
    /// # Errors
    /// Returns `ClockWentBackwards` or `RevisionExhausted`.
    pub fn advance(&mut self, now: Duration) -> Result<(), Error> {
        self.clock(now)?;
        let count: usize = self.sources.values().map(|s| s.conditions.len()).sum();
        self.reserve_revisions(count)?;
        // Collect at most the configured total number of conditions.
        let expired: Vec<Key> = self
            .sources
            .values()
            .flat_map(|s| s.conditions.values())
            .filter(|e| {
                e.state.freshness != Freshness::Stale && e.expires_at.is_some_and(|t| now > t)
            })
            .map(|e| e.state.key)
            .collect();
        for key in expired {
            self.sources
                .get_mut(&key.source)
                .unwrap()
                .conditions
                .get_mut(&key.condition)
                .unwrap()
                .state
                .freshness = Freshness::Stale;
            self.emit(key.source, Some(key), UpdateKind::Freshness, None);
        }
        Ok(())
    }
    /// Change source connectivity at host monotonic time `now`.
    ///
    /// Disconnect marks its conditions stale and emits freshness/source updates.
    /// Reconnect preserves health and existing staleness; new evidence restores
    /// freshness. Repeating the same connectivity value has no domain effect.
    /// Allocates a temporary key list bounded by the source's conditions.
    ///
    /// # Errors
    /// Returns `UnknownSource`, `ClockWentBackwards`, or `RevisionExhausted`.
    pub fn set_connected(
        &mut self,
        source: u128,
        connected: bool,
        now: Duration,
    ) -> Result<(), Error> {
        self.clock(now)?;
        let count = self
            .sources
            .get(&source)
            .ok_or(Error::UnknownSource)?
            .conditions
            .len();
        self.reserve_revisions(count.saturating_add(1))?;
        let s = self.sources.get_mut(&source).unwrap();
        if s.connected == connected {
            return Ok(());
        }
        s.connected = connected;
        let keys: Vec<Key> = s.conditions.values().map(|e| e.state.key).collect();
        if !connected {
            for key in keys {
                self.sources
                    .get_mut(&source)
                    .unwrap()
                    .conditions
                    .get_mut(&key.condition)
                    .unwrap()
                    .state
                    .freshness = Freshness::Stale;
                self.emit(source, Some(key), UpdateKind::Freshness, None);
            }
        }
        self.emit(source, None, UpdateKind::Source, None);
        Ok(())
    }
    /// Return a copy of current condition state, or `None` for an unknown key.
    ///
    /// Does not expire timers; call [`Self::advance`] according to host time.
    pub fn state(&self, key: Key) -> Option<State> {
        self.sources
            .get(&key.source)?
            .conditions
            .get(&key.condition)
            .map(|e| e.state)
    }
    /// Remove the oldest retained domain update, or return `None`.
    ///
    /// Updates are ordered by revision within this aggregator epoch. Retention loss
    /// creates revision gaps; consumers recover current state from a snapshot.
    /// This call performs no allocation or transport and does not clear reconciliation.
    pub fn pop_update(&mut self) -> Option<Update> {
        self.outputs.pop_front()
    }
    /// Copy the current domain view and captured diagnostic metadata.
    ///
    /// Allocates vectors bounded by configured source/condition limits. Includes
    /// source connectivity, cumulative losses, condition revisions and the current
    /// epoch/revision watermark. Diagnostics are read from the injected instance.
    ///
    /// The view omits deduplication and expiry bookkeeping; it is a display/replay
    /// snapshot rather than a complete runtime checkpoint. Does not drain updates,
    /// clear reconciliation, or expire deadlines.
    pub fn snapshot(&self) -> View {
        View {
            epoch: self.epoch,
            revision: self.revision,
            stats: self.observability.stats(),
            sources: self
                .sources
                .iter()
                .map(|(id, s)| SourceView {
                    source: *id,
                    connected: s.connected,
                    losses: s.losses,
                    conditions: s.conditions.values().map(|e| e.state).collect(),
                })
                .collect(),
        }
    }
    /// Remove a disconnected source and emit a `Retired` update.
    ///
    /// The host explicitly chooses retirement; active/stale conditions are otherwise
    /// retained. Reattach consumers with a new snapshot after retirement. A restarted
    /// producer must use a new source incarnation ID.
    ///
    /// # Errors
    /// Returns `UnknownSource` for an absent source, `InvalidConfig` while connected,
    /// or `RevisionExhausted` when another output revision cannot be assigned.
    pub fn retire_source(&mut self, source: u128) -> Result<(), Error> {
        self.reserve_revisions(1)?;
        let s = self.sources.get(&source).ok_or(Error::UnknownSource)?;
        if s.connected {
            return Err(Error::InvalidConfig);
        }
        self.emit(source, None, UpdateKind::Retired, None);
        self.sources.remove(&source);
        Ok(())
    }
}
