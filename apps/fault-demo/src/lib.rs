//! Demo projection of revisioned domain output for a browser.
use fault_aggregation::{State, UpdateKind};
use fault_protocol::{AggregationFrame, AggregationPayload};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    time::Instant,
};
/// Bounded display model. Gaps mark the view incomplete until an authoritative
/// snapshot arrives. IDs/counters are JSON strings to preserve integer precision.
#[derive(Default)]
pub struct Display {
    epoch: u128,
    revision: u64,
    complete: bool,
    conditions: BTreeMap<(u128, u32), State>,
    history: VecDeque<Value>,
    traces: BTreeMap<(u128, u32), Trace>,
    at_ms: f64,
    received: Option<Instant>,
    commands: VecDeque<Value>,
}
impl Display {
    /// Apply ordered publications; a new epoch clears the prior view. A snapshot
    /// replaces all displayed state, including retired sources and filtered keys.
    pub fn apply(&mut self, frame: AggregationFrame) {
        let at_ms = frame.emitted_at.as_secs_f64() * 1000.0;
        let epoch = match &frame.payload {
            AggregationPayload::Snapshot(v) => v.epoch,
            AggregationPayload::Updates(b) => b.epoch,
        };
        if epoch != self.epoch {
            self.epoch = epoch;
            self.revision = 0;
            self.complete = false;
            self.conditions.clear();
            self.history.clear();
            self.traces.clear();
            self.commands.clear();
        }
        if at_ms >= self.at_ms || self.received.is_none() || self.revision == 0 {
            self.at_ms = at_ms;
            self.received = Some(Instant::now());
        }
        match frame.payload {
            AggregationPayload::Snapshot(v) => {
                if v.revision < self.revision {
                    return;
                }
                self.conditions.clear();
                for s in v.sources {
                    for c in s.conditions {
                        if self.conditions.len() < 16384 {
                            self.track(c, at_ms, true);
                            self.conditions.insert((s.source, c.key.condition), c);
                        }
                    }
                }
                self.revision = v.revision;
                self.complete = true;
            }
            AggregationPayload::Updates(b) => {
                for u in b.updates {
                    if u.revision <= self.revision {
                        continue;
                    }
                    if u.revision != self.revision + 1 {
                        self.complete = false;
                    }
                    self.revision = u.revision;
                    if u.kind == UpdateKind::Retired {
                        self.conditions.retain(|(source, _), _| *source != u.source);
                    }
                    if let Some(s) = u.state
                        && (self.conditions.len() < 16384
                            || self
                                .conditions
                                .contains_key(&(s.key.source, s.key.condition)))
                    {
                        self.track(s, at_ms, u.kind == UpdateKind::Reconciled);
                        self.conditions.insert((s.key.source, s.key.condition), s);
                    }
                    self.history.push_back(json!({"revision":u.revision.to_string(),"kind":u.kind.as_str_name(),"source":format!("{:032x}",u.source)}));
                    if self.history.len() > 128 {
                        self.history.pop_front();
                    }
                }
            }
        }
    }
    fn track(&mut self, state: State, at_ms: f64, snapshot: bool) {
        // Reserve timeline history for the demonstration, independently of noisy
        // host diagnostics. Bounds are fixed even for malformed/unexpected sources.
        if state.definition.component.as_str() != "demo" {
            return;
        }
        let key = (state.key.source, state.key.condition);
        if self.traces.len() >= 16 && !self.traces.contains_key(&key) {
            return;
        }
        self.traces
            .entry(key)
            .or_default()
            .push(state, at_ms, snapshot);
    }
    /// Record admission of a demo command. This marks a request, not confirmation
    /// that a producer has applied it. Commands share the publication time axis.
    pub fn command(&mut self, source: &str, mode: &str) {
        self.commands
            .push_back(json!({"at_ms":self.now_ms(),"source":source,"mode":mode}));
        if self.commands.len() > 64 {
            self.commands.pop_front();
        }
    }
    fn now_ms(&self) -> f64 {
        self.at_ms
            + self
                .received
                .map_or(0.0, |t| t.elapsed().as_secs_f64() * 1000.0)
    }
    /// Current state and bounded timeline on the aggregator publication clock.
    /// Report points contain cumulative-count deltas, including coalesced evidence;
    /// snapshot reconciliation is identified explicitly. Recovery progress advances
    /// only from producer evidence, never from the browser's wall clock.
    pub fn json(&self) -> Value {
        json!({"epoch":format!("{:032x}",self.epoch),"revision":self.revision.to_string(),"complete":self.complete,
            "now_ms":self.now_ms(),"commands":self.commands,
            "conditions":self.conditions.values().map(|s|{
                let latest=s.latest;
                let span=latest.and_then(|o|o.success_since.map(|start|o.observed_at.saturating_sub(start).as_secs_f64()*1000.0));
                let trace=self.traces.get(&(s.key.source,s.key.condition));
                json!({"source":format!("{:032x}",s.key.source),"condition":s.key.condition,
                    "component":s.definition.component.as_str(),"resource":s.definition.resource.as_str(),"code":s.definition.code.as_str(),
                    "mode":s.definition.policy.mode.as_str_name(),"recovery_ms":s.definition.policy.recovery.as_secs_f64()*1000.0,
                    "deadline_ms":s.definition.policy.deadline.map(|d|d.as_secs_f64()*1000.0),"recovery_evidence_ms":span,
                    "health":s.health.as_str_name(),"freshness":s.freshness.as_str_name(),
                    "detail":latest.map(|o|o.detail.as_str().to_owned()),"failures":latest.map(|o|o.failures.to_string()),"successes":latest.map(|o|o.successes.to_string()),
                    "states":trace.map(|t|t.states.iter().map(|p|json!({"at_ms":p.at_ms,"health":p.health,"freshness":p.freshness,"snapshot":p.snapshot})).collect::<Vec<_>>()).unwrap_or_default(),
                    "reports":trace.map(|t|t.reports.iter().map(|p|json!({"at_ms":p.at_ms,"successes":p.successes.to_string(),"failures":p.failures.to_string(),"snapshot":p.snapshot})).collect::<Vec<_>>()).unwrap_or_default()
                })
            }).collect::<Vec<_>>(),"history":self.history})
    }
}
#[derive(Default)]
struct Trace {
    states: VecDeque<StatePoint>,
    reports: VecDeque<ReportPoint>,
    sequence: u64,
    successes: u64,
    failures: u64,
}
struct StatePoint {
    at_ms: f64,
    health: &'static str,
    freshness: &'static str,
    snapshot: bool,
}
struct ReportPoint {
    at_ms: f64,
    successes: u64,
    failures: u64,
    snapshot: bool,
}
impl Trace {
    fn push(&mut self, s: State, at_ms: f64, snapshot: bool) {
        let health = s.health.as_str_name();
        let freshness = s.freshness.as_str_name();
        if self
            .states
            .back()
            .is_none_or(|p| p.health != health || p.freshness != freshness)
        {
            self.states.push_back(StatePoint {
                at_ms,
                health,
                freshness,
                snapshot,
            });
        }
        // Preserve the state that spans the left edge of the retained window.
        while self.states.len() > 512
            || (self.states.len() > 1 && self.states[1].at_ms < at_ms - 60_000.0)
        {
            self.states.pop_front();
        }
        if let Some(o) = s.latest
            && o.sequence > self.sequence
        {
            let successes = o.successes.saturating_sub(self.successes);
            let failures = o.failures.saturating_sub(self.failures);
            self.sequence = o.sequence;
            self.successes = o.successes;
            self.failures = o.failures;
            if let Some(p) = self.reports.back_mut()
                && (p.at_ms / 100.0).floor() == (at_ms / 100.0).floor()
                && p.snapshot == snapshot
            {
                p.successes = p.successes.saturating_add(successes);
                p.failures = p.failures.saturating_add(failures);
            } else {
                self.reports.push_back(ReportPoint {
                    at_ms,
                    successes,
                    failures,
                    snapshot,
                });
            }
        }
        while self.reports.len() > 1200
            || self
                .reports
                .front()
                .is_some_and(|p| p.at_ms < at_ms - 60_000.0)
        {
            self.reports.pop_front();
        }
    }
}
