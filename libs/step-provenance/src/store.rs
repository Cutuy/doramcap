use crate::{Result, VERSION, invalid, pb};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Hard bounds include stream tombstones; new streams are rejected at the limit.
#[derive(Clone, Debug)]
pub struct Limits {
    pub generations_per_stream: usize,
    pub streams: usize,
    pub inputs_per_generation: usize,
    pub definitions: usize,
    pub string_bytes: usize,
    pub frame_bytes: usize,
    pub traversal_records: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            generations_per_stream: 128,
            streams: 64,
            inputs_per_generation: 32,
            definitions: 64,
            string_bytes: 256,
            frame_bytes: 1 << 20,
            traversal_records: 4096,
        }
    }
}
impl Limits {
    pub(crate) fn validate(&self) -> Result<()> {
        if [
            self.generations_per_stream,
            self.streams,
            self.inputs_per_generation,
            self.definitions,
            self.string_bytes,
            self.frame_bytes,
            self.traversal_records,
        ]
        .contains(&0)
        {
            return Err(invalid("limits must be nonzero"));
        }
        self.generations_per_stream
            .checked_mul(self.streams)
            .and_then(|n| n.checked_mul(self.definitions))
            .ok_or_else(|| invalid("instance bound overflows usize"))?;
        Ok(())
    }
    pub(crate) fn string(&self, s: &str, required: bool) -> Result<()> {
        if s.len() > self.string_bytes || (required && s.is_empty()) {
            return Err(invalid("empty or oversized identity/configuration string"));
        }
        Ok(())
    }
    pub(crate) fn reference(&self, r: &pb::GenerationRef) -> Result<()> {
        for s in [&r.producer, &r.epoch, &r.step, &r.generation] {
            self.string(s, true)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Record {
    pub generation: pb::GenerationRef,
    pub clock_domain: String,
    pub invocation: String,
    pub start_ns: Option<u64>,
    pub end_ns: Option<u64>,
    pub inputs: Option<Vec<pb::GenerationRef>>,
    pub(crate) arrival: u64,
}
type StreamKey = (String, String, String);
fn stream(r: &pb::GenerationRef) -> StreamKey {
    (r.producer.clone(), r.epoch.clone(), r.step.clone())
}
#[derive(Default)]
struct Stream {
    retained: BTreeMap<u64, pb::GenerationRef>,
    floor: Option<u64>,
}
pub(crate) struct Store {
    pub records: HashMap<pb::GenerationRef, Record>,
    streams: HashMap<StreamKey, Stream>,
    pub by_step: HashMap<pb::Step, BTreeSet<pb::GenerationRef>>,
    pub limits: Limits,
    arrival: u64,
}
pub(crate) struct Change {
    pub generation: pb::GenerationRef,
    pub created: bool,
    pub changed: bool,
    pub evicted: Vec<pb::GenerationRef>,
}
impl Store {
    pub fn new(limits: Limits) -> Self {
        Self {
            records: HashMap::new(),
            streams: HashMap::new(),
            by_step: HashMap::new(),
            limits,
            arrival: 0,
        }
    }
    pub fn expired(&self, r: &pb::GenerationRef) -> bool {
        self.streams.get(&stream(r)).is_some_and(|s| {
            s.floor.is_some_and(|f| r.sequence <= f)
                || (s.retained.len() == self.limits.generations_per_stream
                    && s.retained
                        .first_key_value()
                        .is_some_and(|(&seq, _)| r.sequence < seq))
        })
    }
    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }
    pub fn ingest(&mut self, patch: pb::StepUpdate) -> Result<Change> {
        use prost::Message;
        if patch.version != VERSION || patch.encoded_len() > self.limits.frame_bytes {
            return Err(invalid("unsupported update version or oversized frame"));
        }
        let r = patch
            .generation
            .ok_or_else(|| invalid("missing generation"))?;
        self.limits.reference(&r)?;
        self.limits.string(&patch.clock_domain, true)?;
        self.limits.string(&patch.invocation, false)?;
        if self.expired(&r) {
            return Err(invalid("generation is outside retained history"));
        }
        let key = stream(&r);
        if let Some(s) = self.streams.get(&key) {
            if s.retained.get(&r.sequence).is_some_and(|old| old != &r)
                || s.retained
                    .values()
                    .any(|old| old.generation == r.generation && old != &r)
            {
                return Err(invalid("generation ID/sequence conflict"));
            }
        } else if self.streams.len() >= self.limits.streams {
            return Err(invalid(
                "stream limit reached; restart collector to reset epochs",
            ));
        }
        let old = self.records.get(&r);
        let mut next = old.cloned().unwrap_or_else(|| Record {
            generation: r.clone(),
            clock_domain: patch.clock_domain.clone(),
            invocation: patch.invocation.clone(),
            start_ns: None,
            end_ns: None,
            inputs: None,
            arrival: 0,
        });
        if next.clock_domain != patch.clock_domain || next.invocation != patch.invocation {
            return Err(invalid("immutable generation metadata conflict"));
        }
        let mut changed = old.is_none();
        for (slot, value) in [
            (&mut next.start_ns, patch.start_ns),
            (&mut next.end_ns, patch.end_ns),
        ] {
            if let Some(value) = value {
                if slot.is_some_and(|v| v != value) {
                    return Err(invalid("timestamp conflict"));
                }
                changed |= slot.is_none();
                *slot = Some(value);
            }
        }
        if let (Some(start), Some(end)) = (next.start_ns, next.end_ns)
            && end < start
        {
            return Err(invalid("end precedes start"));
        }
        let new_edges = next.inputs.is_none() && patch.inputs.is_some();
        if let Some(inputs) = patch.inputs {
            if inputs.refs.len() > self.limits.inputs_per_generation {
                return Err(invalid("input limit reached"));
            }
            let mut unique = BTreeSet::new();
            for input in inputs.refs {
                self.limits.reference(&input)?;
                if !unique.insert(input) {
                    return Err(invalid("duplicate input"));
                }
            }
            let inputs: Vec<_> = unique.into_iter().collect();
            if next.inputs.as_ref().is_some_and(|old| old != &inputs) {
                return Err(invalid("input set conflict"));
            }
            changed |= next.inputs.is_none();
            next.inputs = Some(inputs);
        }
        if new_edges {
            // A late parent can close a cycle; validate before changing any state.
            let mut seen = BTreeSet::new();
            let mut stack = next.inputs.clone().unwrap_or_default();
            while let Some(input) = stack.pop() {
                if input == r {
                    return Err(invalid("dependency cycle"));
                }
                if !seen.insert(input.clone()) {
                    continue;
                }
                if seen.len() > self.limits.traversal_records {
                    return Err(invalid("cycle validation traversal limit"));
                }
                if let Some(record) = self.records.get(&input)
                    && let Some(inputs) = &record.inputs
                {
                    stack.extend(inputs.iter().cloned());
                }
            }
        }
        let created = old.is_none();
        if created {
            self.arrival = self
                .arrival
                .checked_add(1)
                .ok_or_else(|| invalid("arrival counter exhausted"))?;
            next.arrival = self.arrival;
        }
        self.records.insert(r.clone(), next);
        self.by_step
            .entry(r.step_key())
            .or_default()
            .insert(r.clone());
        let s = self.streams.entry(key).or_default();
        s.retained.insert(r.sequence, r.clone());
        let mut evicted = Vec::new();
        if s.retained.len() > self.limits.generations_per_stream {
            let (seq, removed) = s.retained.pop_first().expect("nonempty stream");
            s.floor = Some(seq);
            self.records.remove(&removed);
            if let Some(refs) = self.by_step.get_mut(&removed.step_key()) {
                refs.remove(&removed);
            }
            evicted.push(removed);
        }
        Ok(Change {
            generation: r,
            created,
            changed,
            evicted,
        })
    }
}
