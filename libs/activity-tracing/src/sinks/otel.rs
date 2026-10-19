use crate::{
    BatchView, Result, Sink,
    wire::{self, proto},
};
use opentelemetry::{
    InstrumentationScope, KeyValue,
    trace::{Event, Link, SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState},
};
use opentelemetry_sdk::trace::{SpanData, SpanExporter};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, UNIX_EPOCH},
};

#[derive(Default, Debug)]
pub struct OtelStats {
    pub incomplete: AtomicU64,
    pub orphan_records: AtomicU64,
    pub sequence_gaps: AtomicU64,
}
/// Bounded span assembly with the official SDK exporter interface.
/// The exporter must run on this synchronous worker (e.g. OTLP blocking HTTP).
pub struct OtelSink<E: SpanExporter> {
    exporter: E,
    active: BTreeMap<(u64, u64), SpanData>,
    capacity: usize,
    stats: Arc<OtelStats>,
    last_sequence: Option<u64>,
}
fn context(session: u64, root: u64, id: u64) -> SpanContext {
    let mut trace = [0; 16];
    trace[..8].copy_from_slice(&session.to_be_bytes());
    trace[8..].copy_from_slice(&root.to_be_bytes());
    SpanContext::new(
        TraceId::from_bytes(trace),
        SpanId::from_bytes(id.to_be_bytes()),
        TraceFlags::SAMPLED,
        false,
        TraceState::default(),
    )
}
impl<E: SpanExporter> OtelSink<E> {
    pub fn new(mut exporter: E, capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err("OTel span capacity must be nonzero".into());
        }
        exporter.set_resource(
            &opentelemetry_sdk::Resource::builder_empty()
                .with_service_name("doramcap")
                .build(),
        );
        Ok(Self {
            exporter,
            active: BTreeMap::new(),
            capacity,
            stats: Arc::default(),
            last_sequence: None,
        })
    }
    pub fn stats(&self) -> Arc<OtelStats> {
        self.stats.clone()
    }
    fn export(&self, s: SpanData) -> Result<()> {
        futures_executor::block_on(self.exporter.export(vec![s]))?;
        Ok(())
    }
    fn incomplete(&self, mut s: SpanData) -> Result<()> {
        self.stats.incomplete.fetch_add(1, Ordering::Relaxed);
        s.attributes
            .push(KeyValue::new("activity.incomplete", true));
        self.export(s)
    }
    fn attributes(batch: &proto::Batch, r: &proto::Record) -> Vec<KeyValue> {
        r.fields
            .iter()
            .map(|f| {
                let name = batch.dictionary[f.name as usize].clone();
                match f.kind {
                    0 => KeyValue::new(
                        name,
                        if f.bits <= i64::MAX as u64 {
                            opentelemetry::Value::I64(f.bits as i64)
                        } else {
                            opentelemetry::Value::String(f.bits.to_string().into())
                        },
                    ),
                    1 => KeyValue::new(name, f.bits as i64),
                    2 => KeyValue::new(name, f64::from_bits(f.bits)),
                    3 => KeyValue::new(name, f.bits != 0),
                    _ => KeyValue::new(
                        name,
                        std::str::from_utf8(
                            &batch.strings
                                [f.offset as usize..f.offset as usize + f.length as usize],
                        )
                        .unwrap()
                        .to_owned(),
                    ),
                }
            })
            .collect()
    }
}
impl OtelSink<opentelemetry_otlp::SpanExporter> {
    pub fn http(endpoint: impl Into<String>, timeout: Duration, capacity: usize) -> Result<Self> {
        use opentelemetry_otlp::WithExportConfig;
        if timeout.is_zero() {
            return Err("OTLP timeout must be nonzero".into());
        }
        Self::new(
            opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .with_endpoint(endpoint)
                .with_timeout(timeout)
                .build()?,
            capacity,
        )
    }
}
impl<E: SpanExporter + 'static> Sink for OtelSink<E> {
    fn name(&self) -> &str {
        "otel"
    }
    fn write(&mut self, b: BatchView<'_>) -> Result<()> {
        let batch = wire::decode(b.bytes)?;
        if self.last_sequence.is_some_and(|n| batch.sequence != n + 1) {
            self.stats.sequence_gaps.fetch_add(1, Ordering::Relaxed);
            for span in self.active.values_mut() {
                if span.attributes.len() < 64 {
                    span.attributes
                        .push(KeyValue::new("activity.sequence_gap", true));
                }
            }
        }
        self.last_sequence = Some(batch.sequence);
        for r in &batch.records {
            let key = (batch.session, r.span_id);
            let time = UNIX_EPOCH
                + Duration::from_nanos(batch.unix_origin_ns.saturating_add(r.monotonic_ns));
            if r.kind == 0 {
                if self.active.len() >= self.capacity {
                    let (_, s) = self.active.pop_first().unwrap();
                    self.incomplete(s)?;
                }
                let mut attributes = Self::attributes(&batch, r);
                attributes.extend([
                    KeyValue::new("process.pid", batch.pid as i64),
                    KeyValue::new("thread.id", r.tid as i64),
                    KeyValue::new(
                        "thread.name",
                        batch.dictionary[r.thread_name as usize].clone(),
                    ),
                ]);
                let s = SpanData {
                    span_context: context(batch.session, r.trace_root, r.span_id),
                    parent_span_id: SpanId::from_bytes(r.parent_id.to_be_bytes()),
                    parent_span_is_remote: false,
                    span_kind: SpanKind::Internal,
                    name: batch.dictionary[r.name as usize].clone().into(),
                    start_time: time,
                    end_time: time,
                    attributes,
                    dropped_attributes_count: 0,
                    events: Default::default(),
                    links: Default::default(),
                    status: Status::Unset,
                    instrumentation_scope: InstrumentationScope::builder("activity-tracing")
                        .build(),
                };
                self.active.insert(key, s);
                continue;
            }
            let Some(span) = self.active.get_mut(&key) else {
                self.stats.orphan_records.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            span.end_time = span.end_time.max(time);
            match r.kind {
                1 => {
                    if span.events.events.len() < 128 {
                        span.events.events.push(Event::new(
                            batch.dictionary[r.name as usize].clone(),
                            time,
                            Self::attributes(&batch, r),
                            0,
                        ));
                    } else {
                        span.events.dropped_count += 1;
                    }
                }
                2 => {
                    for attribute in Self::attributes(&batch, r) {
                        if let Some(existing) =
                            span.attributes.iter_mut().find(|a| a.key == attribute.key)
                        {
                            *existing = attribute;
                        } else if span.attributes.len() < 64 {
                            span.attributes.push(attribute);
                        } else {
                            span.dropped_attributes_count += 1;
                        }
                    }
                }
                3 => {
                    if span.links.links.len() < 64 {
                        span.links.links.push(Link::new(
                            context(batch.session, r.linked_root, r.linked_id),
                            Vec::new(),
                            0,
                        ));
                    } else {
                        span.links.dropped_count += 1;
                    }
                }
                4 => {
                    let s = self.active.remove(&key).unwrap();
                    self.export(s)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        while let Some((_, s)) = self.active.pop_first() {
            self.incomplete(s)?;
        }
        self.exporter.shutdown()?;
        Ok(())
    }
}
