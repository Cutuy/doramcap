use crate::{Error, Result, Wire, ensure, id, ns, parse_id, pb, required};
use fault_aggregation as a;
use fault_reporter as r;
use std::collections::BTreeSet;
use std::time::Duration;
fn text(s: String) -> Result<r::Text> {
    r::Text::new(&s).map_err(|_| Error::Invalid("text exceeds 192 bytes"))
}
fn mode_to(v: r::Mode) -> i32 {
    v as i32
}
fn mode_from(v: i32) -> Result<r::Mode> {
    let value = r::Mode::try_from(v).map_err(|_| Error::UnknownEnum("mode", v))?;
    ensure(value != r::Mode::Unspecified, "mode unspecified")?;
    Ok(value)
}
fn severity_to(v: r::Severity) -> i32 {
    v as i32
}
fn severity_from(v: i32) -> Result<r::Severity> {
    let value = r::Severity::try_from(v).map_err(|_| Error::UnknownEnum("severity", v))?;
    ensure(value != r::Severity::Unspecified, "severity unspecified")?;
    Ok(value)
}
fn outcome_to(v: r::Outcome) -> i32 {
    v as i32
}
fn outcome_from(v: i32) -> Result<r::Outcome> {
    let value = r::Outcome::try_from(v).map_err(|_| Error::UnknownEnum("outcome", v))?;
    ensure(value != r::Outcome::Unspecified, "outcome unspecified")?;
    Ok(value)
}
fn health_to(v: a::Health) -> i32 {
    v as i32
}
fn health_from(v: i32) -> Result<a::Health> {
    let value = a::Health::try_from(v).map_err(|_| Error::UnknownEnum("health", v))?;
    ensure(value != a::Health::Unspecified, "health unspecified")?;
    Ok(value)
}
fn freshness_to(v: a::Freshness) -> i32 {
    v as i32
}
fn freshness_from(v: i32) -> Result<a::Freshness> {
    let value = a::Freshness::try_from(v).map_err(|_| Error::UnknownEnum("freshness", v))?;
    ensure(value != a::Freshness::Unspecified, "freshness unspecified")?;
    Ok(value)
}
fn updatekind_to(v: a::UpdateKind) -> i32 {
    v as i32
}
fn updatekind_from(v: i32) -> Result<a::UpdateKind> {
    let value = a::UpdateKind::try_from(v).map_err(|_| Error::UnknownEnum("updatekind", v))?;
    ensure(
        value != a::UpdateKind::Unspecified,
        "updatekind unspecified",
    )?;
    Ok(value)
}
impl Wire for r::Key {
    type Proto = pb::Key;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::Key {
            source: id(self.source)?,
            condition: self.condition,
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            source: parse_id(p.source)?,
            condition: p.condition,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        ensure(self.source != 0, "source")
    }
}
impl Wire for r::Policy {
    type Proto = pb::Policy;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::Policy {
            mode: mode_to(self.mode),
            deadline_ns: self.deadline.map(ns).transpose()?,
            recovery_ns: Some(ns(self.recovery)?),
            success_interval_ns: self.success_interval.map(ns).transpose()?,
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            mode: mode_from(p.mode)?,
            deadline: p.deadline_ns.map(Duration::from_nanos),
            recovery: Duration::from_nanos(required(p.recovery_ns, "recovery_ns")?),
            success_interval: p.success_interval_ns.map(Duration::from_nanos),
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        r::Policy::validate(*self).map_err(|_| Error::Invalid("policy"))
    }
}
impl Wire for r::Definition {
    type Proto = pb::Definition;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::Definition {
            component: self.component.as_str().to_owned(),
            resource: self.resource.as_str().to_owned(),
            code: self.code.as_str().to_owned(),
            severity: severity_to(self.severity),
            policy: Some(self.policy.to_proto()?),
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            component: text(p.component)?,
            resource: text(p.resource)?,
            code: text(p.code)?,
            severity: severity_from(p.severity)?,
            policy: r::Policy::from_proto(required(p.policy, "policy")?)?,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        ensure(
            !self.component.as_str().is_empty() && !self.code.as_str().is_empty(),
            "definition",
        )?;
        ensure(
            self.severity != r::Severity::Unspecified,
            "severity unspecified",
        )?;
        Wire::validate(&self.policy)
    }
}
impl Wire for r::TraceContext {
    type Proto = pb::TraceContext;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::TraceContext {
            trace_id: self.trace_id.to_vec(),
            span_id: self.span_id.to_vec(),
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            trace_id: p
                .trace_id
                .try_into()
                .map_err(|_| Error::Invalid("trace_id length"))?,
            span_id: p
                .span_id
                .try_into()
                .map_err(|_| Error::Invalid("span_id length"))?,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
impl Wire for r::Observation {
    type Proto = pb::Observation;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::Observation {
            key: Some(self.key.to_proto()?),
            sequence: self.sequence,
            observed_at_ns: ns(self.observed_at)?,
            outcome: outcome_to(self.outcome),
            successes: self.successes,
            failures: self.failures,
            first_failure_at_ns: self.first_failure_at.map(ns).transpose()?,
            last_failure_at_ns: self.last_failure_at.map(ns).transpose()?,
            success_since_ns: self.success_since.map(ns).transpose()?,
            consecutive_successes: self.consecutive_successes,
            max_success_span_ns: ns(self.max_success_span)?,
            detail: self.detail.as_str().to_owned(),
            trace: self.trace.as_ref().map(Wire::to_proto).transpose()?,
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            key: r::Key::from_proto(required(p.key, "key")?)?,
            sequence: p.sequence,
            observed_at: Duration::from_nanos(p.observed_at_ns),
            outcome: outcome_from(p.outcome)?,
            successes: p.successes,
            failures: p.failures,
            first_failure_at: p.first_failure_at_ns.map(Duration::from_nanos),
            last_failure_at: p.last_failure_at_ns.map(Duration::from_nanos),
            success_since: p.success_since_ns.map(Duration::from_nanos),
            consecutive_successes: p.consecutive_successes,
            max_success_span: Duration::from_nanos(p.max_success_span_ns),
            detail: text(p.detail)?,
            trace: p.trace.map(r::TraceContext::from_proto).transpose()?,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        self.key.validate()?;
        ensure(
            self.sequence > 0 && self.successes.checked_add(self.failures) == Some(self.sequence),
            "observation counts",
        )?;
        ensure(
            match (self.first_failure_at, self.last_failure_at) {
                (None, None) => self.failures == 0,
                (Some(f), Some(l)) => self.failures > 0 && f <= l && l <= self.observed_at,
                _ => false,
            },
            "failure timestamps",
        )?;
        ensure(
            match self.outcome {
                r::Outcome::Unspecified => false,
                r::Outcome::Failure => {
                    self.failures > 0
                        && self.last_failure_at == Some(self.observed_at)
                        && self.success_since.is_none()
                        && self.consecutive_successes == 0
                        && self.max_success_span.is_zero()
                }
                r::Outcome::Success => {
                    self.successes > 0
                        && self.consecutive_successes > 0
                        && self.consecutive_successes <= self.successes
                        && self.max_success_span <= self.observed_at
                        && self.success_since.is_some_and(|s| {
                            s <= self.observed_at
                                && self.max_success_span >= self.observed_at - s
                                && (self.consecutive_successes != 1 || s == self.observed_at)
                                && self.last_failure_at.is_none_or(|f| s >= f)
                        })
                }
            },
            "observation evidence",
        )
    }
}
impl Wire for r::Losses {
    type Proto = pb::Losses;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::Losses {
            failures: self.failures,
            successes: self.successes,
            coalesced_successes: self.coalesced_successes,
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            failures: p.failures,
            successes: p.successes,
            coalesced_successes: p.coalesced_successes,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
impl Wire for r::ConditionSnapshot {
    type Proto = pb::ConditionSnapshot;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::ConditionSnapshot {
            key: Some(self.key.to_proto()?),
            definition: Some(self.definition.to_proto()?),
            latest: self.latest.as_ref().map(Wire::to_proto).transpose()?,
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            key: r::Key::from_proto(required(p.key, "key")?)?,
            definition: r::Definition::from_proto(required(p.definition, "definition")?)?,
            latest: p.latest.map(r::Observation::from_proto).transpose()?,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        self.key.validate()?;
        self.definition.validate()?;
        if let Some(o) = self.latest {
            o.validate()?;
            ensure(o.key == self.key, "condition key")?;
            ensure(
                self.definition.policy.mode != r::Mode::EventOnly || o.successes == 0,
                "event-only evidence",
            )?;
        }
        Ok(())
    }
}
impl Wire for r::Batch {
    type Proto = pb::ReporterBatch;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::ReporterBatch {
            source: id(self.source)?,
            emitted_at_ns: ns(self.emitted_at)?,
            observations: self
                .observations
                .iter()
                .map(Wire::to_proto)
                .collect::<Result<_>>()?,
            losses: Some(self.losses.to_proto()?),
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            source: parse_id(p.source)?,
            emitted_at: Duration::from_nanos(p.emitted_at_ns),
            observations: p
                .observations
                .into_iter()
                .map(r::Observation::from_proto)
                .collect::<Result<_>>()?,
            losses: r::Losses::from_proto(required(p.losses, "losses")?)?,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        ensure(self.source != 0, "source")?;
        for o in &self.observations {
            o.validate()?;
            ensure(
                o.key.source == self.source && o.observed_at <= self.emitted_at,
                "batch source/time",
            )?;
        }
        Ok(())
    }
}
impl Wire for r::Snapshot {
    type Proto = pb::ReporterSnapshot;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::ReporterSnapshot {
            source: id(self.source)?,
            emitted_at_ns: ns(self.emitted_at)?,
            conditions: self
                .conditions
                .iter()
                .map(Wire::to_proto)
                .collect::<Result<_>>()?,
            losses: Some(self.losses.to_proto()?),
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            source: parse_id(p.source)?,
            emitted_at: Duration::from_nanos(p.emitted_at_ns),
            conditions: p
                .conditions
                .into_iter()
                .map(r::ConditionSnapshot::from_proto)
                .collect::<Result<_>>()?,
            losses: r::Losses::from_proto(required(p.losses, "losses")?)?,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        ensure(self.source != 0, "source")?;
        let mut ids = BTreeSet::new();
        for c in &self.conditions {
            c.validate()?;
            ensure(
                c.key.source == self.source && ids.insert(c.key.condition),
                "snapshot key",
            )?;
            ensure(
                c.latest.is_none_or(|o| o.observed_at <= self.emitted_at),
                "snapshot time",
            )?;
        }
        Ok(())
    }
}
impl Wire for a::Stats {
    type Proto = pb::Stats;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::Stats {
            dropped_updates: self.dropped_updates,
            rejected_inputs: self.rejected_inputs,
            too_old: self.too_old,
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            dropped_updates: p.dropped_updates,
            rejected_inputs: p.rejected_inputs,
            too_old: p.too_old,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
impl Wire for a::State {
    type Proto = pb::State;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::State {
            key: Some(self.key.to_proto()?),
            definition: Some(self.definition.to_proto()?),
            health: health_to(self.health),
            freshness: freshness_to(self.freshness),
            latest: self.latest.as_ref().map(Wire::to_proto).transpose()?,
            revision: self.revision,
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            key: r::Key::from_proto(required(p.key, "key")?)?,
            definition: r::Definition::from_proto(required(p.definition, "definition")?)?,
            health: health_from(p.health)?,
            freshness: freshness_from(p.freshness)?,
            latest: p.latest.map(r::Observation::from_proto).transpose()?,
            revision: p.revision,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        ensure(
            self.health != a::Health::Unspecified && self.freshness != a::Freshness::Unspecified,
            "state enum unspecified",
        )?;
        ensure(self.revision > 0, "state revision")?;
        r::ConditionSnapshot {
            key: self.key,
            definition: self.definition,
            latest: self.latest,
        }
        .validate()
    }
}
impl Wire for a::Update {
    type Proto = pb::Update;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::Update {
            epoch: id(self.epoch)?,
            revision: self.revision,
            kind: updatekind_to(self.kind),
            key: self.key.as_ref().map(Wire::to_proto).transpose()?,
            state: self.state.as_ref().map(Wire::to_proto).transpose()?,
            observation: self.observation.as_ref().map(Wire::to_proto).transpose()?,
            source: id(self.source)?,
            connected: self.connected,
            losses: Some(self.losses.to_proto()?),
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            epoch: parse_id(p.epoch)?,
            revision: p.revision,
            kind: updatekind_from(p.kind)?,
            key: p.key.map(r::Key::from_proto).transpose()?,
            state: p.state.map(a::State::from_proto).transpose()?,
            observation: p.observation.map(r::Observation::from_proto).transpose()?,
            source: parse_id(p.source)?,
            connected: p.connected,
            losses: r::Losses::from_proto(required(p.losses, "losses")?)?,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        ensure(
            self.epoch != 0 && self.source != 0 && self.revision > 0,
            "update identity",
        )?;
        ensure(
            self.kind != a::UpdateKind::Unspecified,
            "update kind unspecified",
        )?;
        let source_wide = matches!(self.kind, a::UpdateKind::Source | a::UpdateKind::Retired);
        ensure(
            self.key.is_none() == source_wide && self.state.is_none() == source_wide,
            "update state presence",
        )?;
        if let Some(s) = self.state {
            s.validate()?;
            ensure(
                Some(s.key) == self.key
                    && s.key.source == self.source
                    && s.revision == self.revision,
                "update state identity",
            )?;
        }
        if let Some(o) = self.observation {
            o.validate()?;
            ensure(Some(o.key) == self.key, "update observation key")?;
        }
        ensure(
            self.observation.is_some()
                == matches!(
                    self.kind,
                    a::UpdateKind::Observation
                        | a::UpdateKind::Transition
                        | a::UpdateKind::Reconciled
                        | a::UpdateKind::Late
                        | a::UpdateKind::Event
                ),
            "update observation presence",
        )
    }
}
impl Wire for a::SourceView {
    type Proto = pb::SourceView;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::SourceView {
            source: id(self.source)?,
            connected: self.connected,
            losses: Some(self.losses.to_proto()?),
            conditions: self
                .conditions
                .iter()
                .map(Wire::to_proto)
                .collect::<Result<_>>()?,
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            source: parse_id(p.source)?,
            connected: p.connected,
            losses: r::Losses::from_proto(required(p.losses, "losses")?)?,
            conditions: p
                .conditions
                .into_iter()
                .map(a::State::from_proto)
                .collect::<Result<_>>()?,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        ensure(self.source != 0, "source")?;
        let mut ids = BTreeSet::new();
        for s in &self.conditions {
            s.validate()?;
            ensure(
                s.key.source == self.source && ids.insert(s.key.condition),
                "source condition key",
            )?;
        }
        Ok(())
    }
}
impl Wire for a::View {
    type Proto = pb::AggregationSnapshot;
    fn to_proto(&self) -> Result<Self::Proto> {
        Wire::validate(self)?;
        Ok(pb::AggregationSnapshot {
            epoch: id(self.epoch)?,
            revision: self.revision,
            stats: Some(self.stats.to_proto()?),
            sources: self
                .sources
                .iter()
                .map(Wire::to_proto)
                .collect::<Result<_>>()?,
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            epoch: parse_id(p.epoch)?,
            revision: p.revision,
            stats: a::Stats::from_proto(required(p.stats, "stats")?)?,
            sources: p
                .sources
                .into_iter()
                .map(a::SourceView::from_proto)
                .collect::<Result<_>>()?,
        };
        Wire::validate(&value)?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        ensure(self.epoch != 0, "epoch")?;
        let mut ids = BTreeSet::new();
        for s in &self.sources {
            s.validate()?;
            ensure(ids.insert(s.source), "duplicate source")?;
            for c in &s.conditions {
                ensure(c.revision <= self.revision, "snapshot revision")?;
            }
        }
        Ok(())
    }
}
