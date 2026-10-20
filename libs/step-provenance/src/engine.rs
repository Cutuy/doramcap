use crate::{
    Result, VERSION, invalid,
    pb::{self, Boundary, Reason, Selector},
    store::{Limits, Store},
};
use prost::Message;
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub records: usize,
    pub streams: usize,
    pub instances: usize,
    pub pending: usize,
    pub wait_keys: usize,
    pub wait_links: usize,
    /// Number of instance evaluations; useful for verifying targeted dispatch.
    pub evaluations: u64,
}
/// Declared source generations plus gaps at the retention/reporting boundary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lineage {
    pub roots: Vec<pb::GenerationRef>,
    pub missing: Vec<pb::GenerationRef>,
    pub expired: Vec<pb::GenerationRef>,
}
impl Lineage {
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty() && self.expired.is_empty()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InstanceKey {
    definition: usize,
    anchor: pb::GenerationRef,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Wait {
    Record(pb::GenerationRef),
    Time(pb::GenerationRef, Boundary),
    Step(pb::Step),
    History(String, String, String),
}
#[derive(Default)]
struct ProbeState {
    selected: Option<pb::SelectedProbe>,
    failure: Option<Reason>,
}
struct Instance {
    probes: [ProbeState; 2],
    waits: Vec<Wait>,
    done: bool,
}

/// Synchronous evaluator. Call `ingest` in arrival order and drain returned results.
/// No internal worker, timer, result queue, or activity-tracing dependency.
pub struct Engine {
    store: Store,
    definitions: Vec<pb::MeasurementDefinition>,
    anchors: HashMap<pb::Step, Vec<usize>>,
    affected: HashMap<(pb::Step, Boundary), Vec<usize>>,
    instances: HashMap<InstanceKey, Instance>,
    by_anchor: HashMap<pb::GenerationRef, Vec<InstanceKey>>,
    waiters: HashMap<Wait, BTreeSet<InstanceKey>>,
    evaluations: u64,
}
impl Engine {
    pub fn new(config: pb::Definitions, limits: Limits) -> Result<Self> {
        limits.validate()?;
        if config.version != VERSION
            || config.measurements.len() > limits.definitions
            || config.encoded_len() > limits.frame_bytes
        {
            return Err(invalid(
                "unsupported definitions version or configuration limit",
            ));
        }
        let mut names = BTreeSet::new();
        let mut anchors: HashMap<_, Vec<_>> = HashMap::new();
        let mut affected: HashMap<_, Vec<_>> = HashMap::new();
        for (id, d) in config.measurements.iter().enumerate() {
            limits.string(&d.name, true)?;
            if !names.insert(&d.name) {
                return Err(invalid("duplicate measurement name"));
            }
            if d.missing_data != pb::MissingDataPolicy::WaitUntilEviction as i32 {
                return Err(invalid("unsupported missing-data policy"));
            }
            let anchor = d.anchor.as_ref().ok_or_else(|| invalid("missing anchor"))?;
            validate_step(anchor, &limits)?;
            anchors.entry(anchor.clone()).or_default().push(id);
            for p in [&d.start, &d.end] {
                let p = p.as_ref().ok_or_else(|| invalid("missing probe"))?;
                let step = p
                    .step
                    .as_ref()
                    .ok_or_else(|| invalid("missing probe step"))?;
                validate_step(step, &limits)?;
                let boundary =
                    Boundary::try_from(p.boundary).map_err(|_| invalid("unknown boundary"))?;
                if boundary == Boundary::Unspecified {
                    return Err(invalid("unspecified boundary"));
                }
                let selector =
                    Selector::try_from(p.selector).map_err(|_| invalid("unknown selector"))?;
                if selector == Selector::Unspecified {
                    return Err(invalid("unspecified selector"));
                }
                if selector == Selector::Previous && step != anchor {
                    return Err(invalid("previous selects the anchor's own step"));
                }
                let ids = affected.entry((step.clone(), boundary)).or_default();
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
        Ok(Self {
            store: Store::new(limits),
            definitions: config.measurements,
            anchors,
            affected,
            instances: HashMap::new(),
            by_anchor: HashMap::new(),
            waiters: HashMap::new(),
            evaluations: 0,
        })
    }
    pub fn decode_update(&mut self, bytes: &[u8]) -> Result<Vec<pb::MeasurementResult>> {
        if bytes.len() > self.store.limits.frame_bytes {
            return Err(invalid("oversized frame"));
        }
        let patch =
            pb::StepUpdate::decode(bytes).map_err(|e| invalid(format!("invalid protobuf: {e}")))?;
        self.ingest(patch)
    }
    pub fn record(&self, r: &pb::GenerationRef) -> Option<&crate::Record> {
        self.store.records.get(r)
    }
    /// Follow direct input edges to declared roots, deduplicating shared ancestors.
    pub fn lineage(&self, output: &pb::GenerationRef) -> Result<Lineage> {
        let mut seen = BTreeSet::new();
        let mut stack = vec![output.clone()];
        let mut result = Lineage::default();
        while let Some(r) = stack.pop() {
            if !seen.insert(r.clone()) {
                continue;
            }
            if seen.len() > self.store.limits.traversal_records {
                return Err(invalid("lineage traversal limit"));
            }
            if self.store.expired(&r) {
                result.expired.push(r);
                continue;
            }
            match self
                .store
                .records
                .get(&r)
                .and_then(|record| record.inputs.as_ref())
            {
                Some(inputs) if inputs.is_empty() => result.roots.push(r),
                Some(inputs) => stack.extend(inputs.iter().cloned()),
                None => result.missing.push(r),
            }
        }
        result.roots.sort();
        result.missing.sort();
        result.expired.sort();
        Ok(result)
    }
    pub fn affected_definitions(&self, step: &pb::Step, boundary: Boundary) -> Vec<&str> {
        self.affected
            .get(&(step.clone(), boundary))
            .into_iter()
            .flatten()
            .map(|&id| self.definitions[id].name.as_str())
            .collect()
    }
    pub fn stats(&self) -> Stats {
        Stats {
            records: self.store.records.len(),
            streams: self.store.stream_count(),
            instances: self.instances.len(),
            pending: self.instances.values().filter(|i| !i.done).count(),
            wait_keys: self.waiters.len(),
            wait_links: self.waiters.values().map(BTreeSet::len).sum(),
            evaluations: self.evaluations,
        }
    }
    pub fn ingest(&mut self, patch: pb::StepUpdate) -> Result<Vec<pb::MeasurementResult>> {
        let start_changed = patch.start_ns.is_some();
        let end_changed = patch.end_ns.is_some();
        let change = self.store.ingest(patch)?;
        if !change.changed {
            return Ok(Vec::new());
        }
        let mut results = Vec::new();
        let mut ready = BTreeSet::new();
        for evicted in &change.evicted {
            if let Some(keys) = self.by_anchor.remove(evicted) {
                for key in keys {
                    if let Some(instance) = self.instances.remove(&key) {
                        self.unwatch(&key, &instance.waits);
                        if !instance.done {
                            results.push(self.result(&key, &instance, Some(Reason::AnchorEvicted)));
                        }
                    }
                }
            }
            self.wake(&Wait::Record(evicted.clone()), &mut ready);
            self.wake(&Wait::Time(evicted.clone(), Boundary::Start), &mut ready);
            self.wake(&Wait::Time(evicted.clone(), Boundary::End), &mut ready);
            self.wake(&Wait::Step(evicted.step_key()), &mut ready);
        }
        self.wake(&Wait::Record(change.generation.clone()), &mut ready);
        if start_changed {
            self.wake(
                &Wait::Time(change.generation.clone(), Boundary::Start),
                &mut ready,
            );
        }
        if end_changed {
            self.wake(
                &Wait::Time(change.generation.clone(), Boundary::End),
                &mut ready,
            );
        }
        self.wake(&Wait::Step(change.generation.step_key()), &mut ready);
        if change.created {
            self.wake(&history(&change.generation), &mut ready);
            for &definition in self
                .anchors
                .get(&change.generation.step_key())
                .into_iter()
                .flatten()
            {
                let key = InstanceKey {
                    definition,
                    anchor: change.generation.clone(),
                };
                let d = &self.definitions[definition];
                let probes = [d.start.as_ref().unwrap(), d.end.as_ref().unwrap()].map(|p| {
                    if p.selector() == Selector::LatestKnownAtAnchor {
                        let latest = self
                            .store
                            .by_step
                            .get(p.step.as_ref().unwrap())
                            .into_iter()
                            .flatten()
                            .filter_map(|r| self.store.records.get(r))
                            .max_by_key(|r| r.arrival);
                        match latest {
                            Some(r) => ProbeState {
                                selected: Some(selected(&r.generation, p.boundary())),
                                failure: None,
                            },
                            None => ProbeState {
                                selected: None,
                                failure: Some(Reason::NoMatch),
                            },
                        }
                    } else {
                        ProbeState::default()
                    }
                });
                self.instances.insert(
                    key.clone(),
                    Instance {
                        probes,
                        waits: Vec::new(),
                        done: false,
                    },
                );
                self.by_anchor
                    .entry(change.generation.clone())
                    .or_default()
                    .push(key.clone());
                ready.insert(key);
            }
        }
        for key in ready {
            let Some(mut instance) = self.instances.remove(&key) else {
                continue;
            };
            if instance.done {
                self.instances.insert(key, instance);
                continue;
            }
            self.unwatch(&key, &instance.waits);
            instance.waits.clear();
            self.evaluations = self.evaluations.saturating_add(1);
            let d = &self.definitions[key.definition];
            for (state, probe) in instance
                .probes
                .iter_mut()
                .zip([d.start.as_ref().unwrap(), d.end.as_ref().unwrap()])
            {
                self.resolve(&key.anchor, probe, state, &mut instance.waits);
            }
            let failure = instance.probes.iter().find_map(|p| p.failure);
            let complete = instance.probes.iter().all(|p| {
                p.selected
                    .as_ref()
                    .is_some_and(|s| s.timestamp_ns.is_some())
            });
            if failure.is_some() || complete {
                results.push(self.result(&key, &instance, failure));
                instance.done = true;
                instance.waits.clear();
            } else {
                for wait in &instance.waits {
                    self.waiters
                        .entry(wait.clone())
                        .or_default()
                        .insert(key.clone());
                }
            }
            self.instances.insert(key, instance);
        }
        Ok(results)
    }
    fn wake(&self, wait: &Wait, ready: &mut BTreeSet<InstanceKey>) {
        if let Some(keys) = self.waiters.get(wait) {
            ready.extend(keys.iter().cloned());
        }
    }
    fn unwatch(&mut self, key: &InstanceKey, waits: &[Wait]) {
        for wait in waits {
            if let Some(keys) = self.waiters.get_mut(wait) {
                keys.remove(key);
                if keys.is_empty() {
                    self.waiters.remove(wait);
                }
            }
        }
    }
    fn resolve(
        &self,
        anchor: &pb::GenerationRef,
        probe: &pb::Probe,
        state: &mut ProbeState,
        waits: &mut Vec<Wait>,
    ) {
        if state.failure.is_some() {
            return;
        }
        if state.selected.is_none() {
            let target = probe.step.as_ref().unwrap();
            let mut matches = BTreeSet::new();
            match probe.selector() {
                Selector::ConsumedAncestor => {
                    let mut stack = vec![anchor.clone()];
                    let mut seen = BTreeSet::new();
                    let before = waits.len();
                    while let Some(r) = stack.pop() {
                        if !seen.insert(r.clone()) {
                            continue;
                        }
                        if seen.len() > self.store.limits.traversal_records {
                            state.failure = Some(Reason::TraversalLimit);
                            return;
                        }
                        if &r.step_key() == target {
                            matches.insert(r);
                            continue;
                        }
                        if self.store.expired(&r) {
                            state.failure = Some(Reason::HistoryEvicted);
                            return;
                        }
                        match self
                            .store
                            .records
                            .get(&r)
                            .and_then(|record| record.inputs.as_ref())
                        {
                            Some(inputs) => stack.extend(inputs.iter().cloned()),
                            None => {
                                if !self.store.records.contains_key(&r) {
                                    waits.push(history(&r));
                                }
                                waits.push(Wait::Record(r));
                            }
                        }
                    }
                    if waits.len() > before {
                        return;
                    }
                    if matches.is_empty() {
                        state.failure = Some(Reason::NoMatch);
                        return;
                    }
                }
                Selector::SameInvocation => {
                    let invocation = &self.store.records[anchor].invocation;
                    if invocation.is_empty() {
                        state.failure = Some(Reason::NoMatch);
                        return;
                    }
                    for r in self.store.by_step.get(target).into_iter().flatten() {
                        if self.store.records[r].invocation == *invocation {
                            matches.insert(r.clone());
                        }
                    }
                }
                Selector::Previous => {
                    let Some(sequence) = anchor.sequence.checked_sub(1) else {
                        state.failure = Some(Reason::NoMatch);
                        return;
                    };
                    let mut prior = anchor.clone();
                    prior.sequence = sequence;
                    if self.store.expired(&prior) {
                        state.failure = Some(Reason::HistoryEvicted);
                        return;
                    }
                    for r in self.store.by_step.get(target).into_iter().flatten() {
                        if r.epoch == anchor.epoch && r.sequence == sequence {
                            matches.insert(r.clone());
                        }
                    }
                }
                _ => unreachable!("selectors validated; latest is frozen at anchor creation"),
            }
            match matches.len() {
                0 => {
                    waits.push(Wait::Step(target.clone()));
                    return;
                }
                1 => state.selected = Some(selected(matches.first().unwrap(), probe.boundary())),
                _ => {
                    state.failure = Some(Reason::Ambiguous);
                    return;
                }
            }
        }
        let chosen = state.selected.as_mut().unwrap();
        if chosen.timestamp_ns.is_some() {
            return;
        }
        let r = chosen.generation.as_ref().unwrap();
        if self.store.expired(r) {
            state.failure = Some(Reason::HistoryEvicted);
            return;
        }
        if let Some(record) = self.store.records.get(r) {
            chosen.clock_domain = record.clock_domain.clone();
            chosen.timestamp_ns = match probe.boundary() {
                Boundary::Start => record.start_ns,
                _ => record.end_ns,
            };
        }
        if chosen.timestamp_ns.is_none() {
            if !self.store.records.contains_key(r) {
                waits.push(history(r));
            }
            waits.push(Wait::Time(r.clone(), probe.boundary()));
        }
    }
    fn result(
        &self,
        key: &InstanceKey,
        instance: &Instance,
        mut failure: Option<Reason>,
    ) -> pb::MeasurementResult {
        let [start, end] = [&instance.probes[0].selected, &instance.probes[1].selected];
        let mut duration = None;
        if failure.is_none() {
            let (s, e) = (start.as_ref().unwrap(), end.as_ref().unwrap());
            if s.clock_domain != e.clock_domain {
                failure = Some(Reason::ClockMismatch);
            } else {
                duration = e.timestamp_ns.unwrap().checked_sub(s.timestamp_ns.unwrap());
                if duration.is_none() {
                    failure = Some(Reason::InvalidDuration);
                }
            }
        }
        pb::MeasurementResult {
            version: VERSION,
            measurement: self.definitions[key.definition].name.clone(),
            anchor: Some(key.anchor.clone()),
            start: start.clone(),
            end: end.clone(),
            status: if failure.is_none() {
                pb::Status::Complete as i32
            } else {
                pb::Status::Incomplete as i32
            },
            reason: failure.unwrap_or(Reason::Unspecified) as i32,
            duration_ns: duration,
        }
    }
}
fn validate_step(step: &pb::Step, limits: &Limits) -> Result<()> {
    limits.string(&step.producer, true)?;
    limits.string(&step.name, true)
}
fn selected(r: &pb::GenerationRef, boundary: Boundary) -> pb::SelectedProbe {
    pb::SelectedProbe {
        generation: Some(r.clone()),
        boundary: boundary as i32,
        timestamp_ns: None,
        clock_domain: String::new(),
    }
}

fn history(r: &pb::GenerationRef) -> Wait {
    Wait::History(r.producer.clone(), r.epoch.clone(), r.step.clone())
}
