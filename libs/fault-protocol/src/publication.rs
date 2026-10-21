use crate::{Error, Result, VERSION, Wire, ensure, id, ns, parse_id, pb, required};
use fault_aggregation as a;
use fault_reporter as r;
use prost::Message;
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Checked before Protobuf decoding. Decoding still allocates temporary messages.
    pub max_frame_bytes: usize,
    /// Total top-level sources + conditions for snapshots, or records for batches.
    pub max_records: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 8 * 1024 * 1024,
            max_records: 32_768,
        }
    }
}
impl Limits {
    fn records(self, count: usize) -> Result<()> {
        if count > self.max_records {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn bytes(self, count: usize) -> Result<()> {
        if count > self.max_frame_bytes {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug)]
pub enum ReporterPayload {
    Batch(r::Batch),
    Snapshot(r::Snapshot),
}
#[derive(Clone, Debug)]
pub struct ReporterFrame {
    pub published_unix_ns: Option<u64>,
    pub payload: ReporterPayload,
}
#[derive(Clone, Debug)]
pub struct Updates {
    pub epoch: u128,
    pub updates: Vec<a::Update>,
    pub stats: a::Stats,
}
impl Wire for Updates {
    type Proto = pb::UpdateBatch;
    fn to_proto(&self) -> Result<Self::Proto> {
        self.validate()?;
        Ok(pb::UpdateBatch {
            epoch: id(self.epoch)?,
            updates: self
                .updates
                .iter()
                .map(Wire::to_proto)
                .collect::<Result<_>>()?,
            stats: Some(self.stats.to_proto()?),
        })
    }
    fn from_proto(p: Self::Proto) -> Result<Self> {
        let value = Self {
            epoch: parse_id(p.epoch)?,
            updates: p
                .updates
                .into_iter()
                .map(a::Update::from_proto)
                .collect::<Result<_>>()?,
            stats: a::Stats::from_proto(required(p.stats, "stats")?)?,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        ensure(self.epoch != 0, "epoch")?;
        let mut revision = 0;
        for update in &self.updates {
            update.validate()?;
            ensure(
                update.epoch == self.epoch && update.revision > revision,
                "update ordering/epoch",
            )?;
            revision = update.revision;
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub enum AggregationPayload {
    Updates(Updates),
    Snapshot(a::View),
}
#[derive(Clone, Debug)]
pub struct AggregationFrame {
    pub published_unix_ns: Option<u64>,
    /// Aggregator-local monotonic publication time, scoped by aggregation epoch.
    pub emitted_at: Duration,
    pub payload: AggregationPayload,
}
fn encode<M: Message>(p: M, limits: Limits) -> Result<Vec<u8>> {
    limits.bytes(p.encoded_len())?;
    Ok(p.encode_to_vec())
}
/// No length prefix: one publication occupies one IPC payload or MCAP message.
pub fn encode_reporter(frame: &ReporterFrame, limits: Limits) -> Result<Vec<u8>> {
    use pb::reporter_publication::Payload;
    let payload = match &frame.payload {
        ReporterPayload::Batch(b) => {
            limits.records(b.observations.len())?;
            Payload::Batch(b.to_proto()?)
        }
        ReporterPayload::Snapshot(s) => {
            limits.records(s.conditions.len())?;
            Payload::Snapshot(s.to_proto()?)
        }
    };
    encode(
        pb::ReporterPublication {
            version: VERSION,
            published_unix_ns: frame.published_unix_ns,
            payload: Some(payload),
        },
        limits,
    )
}
pub fn decode_reporter(bytes: &[u8], limits: Limits) -> Result<ReporterFrame> {
    use pb::reporter_publication::Payload;
    limits.bytes(bytes.len())?;
    let p = pb::ReporterPublication::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
    if p.version != VERSION {
        return Err(Error::UnsupportedVersion(p.version));
    }
    let payload = match required(p.payload, "payload")? {
        Payload::Batch(b) => {
            limits.records(b.observations.len())?;
            ReporterPayload::Batch(r::Batch::from_proto(b)?)
        }
        Payload::Snapshot(s) => {
            limits.records(s.conditions.len())?;
            ReporterPayload::Snapshot(r::Snapshot::from_proto(s)?)
        }
    };
    Ok(ReporterFrame {
        published_unix_ns: p.published_unix_ns,
        payload,
    })
}
pub fn encode_aggregation(frame: &AggregationFrame, limits: Limits) -> Result<Vec<u8>> {
    use pb::aggregation_publication::Payload;
    let payload = match &frame.payload {
        AggregationPayload::Updates(b) => {
            limits.records(b.updates.len())?;
            Payload::Updates(b.to_proto()?)
        }
        AggregationPayload::Snapshot(s) => {
            limits.records(
                s.sources
                    .iter()
                    .fold(s.sources.len(), |n, s| n.saturating_add(s.conditions.len())),
            )?;
            Payload::Snapshot(s.to_proto()?)
        }
    };
    encode(
        pb::AggregationPublication {
            version: VERSION,
            published_unix_ns: frame.published_unix_ns,
            emitted_at_ns: ns(frame.emitted_at)?,
            payload: Some(payload),
        },
        limits,
    )
}
pub fn decode_aggregation(bytes: &[u8], limits: Limits) -> Result<AggregationFrame> {
    use pb::aggregation_publication::Payload;
    limits.bytes(bytes.len())?;
    let p = pb::AggregationPublication::decode(bytes).map_err(|e| Error::Decode(e.to_string()))?;
    if p.version != VERSION {
        return Err(Error::UnsupportedVersion(p.version));
    }
    let payload = match required(p.payload, "payload")? {
        Payload::Updates(b) => {
            limits.records(b.updates.len())?;
            AggregationPayload::Updates(Updates::from_proto(b)?)
        }
        Payload::Snapshot(s) => {
            limits.records(
                s.sources
                    .iter()
                    .fold(s.sources.len(), |n, s| n.saturating_add(s.conditions.len())),
            )?;
            AggregationPayload::Snapshot(a::View::from_proto(s)?)
        }
    };
    Ok(AggregationFrame {
        published_unix_ns: p.published_unix_ns,
        emitted_at: Duration::from_nanos(p.emitted_at_ns),
        payload,
    })
}
