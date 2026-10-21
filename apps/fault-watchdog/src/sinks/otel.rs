//! Structured OTLP/HTTP logs projected from the domain publication stream.
use super::{Packet, Sink};
use crate::config::pb::OtelConfig;
use eyre::Result;
use fault_protocol::AggregationPayload;
use opentelemetry::{
    InstrumentationScope, SpanId, TraceId,
    logs::{AnyValue, LogRecord, Logger, LoggerProvider, Severity},
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    logs::{LogBatch, LogExporter, SdkLoggerProvider},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
/// One synchronous HTTP exporter. The outer sink executor owns retries and queue
/// bounds; stable epoch/revision attributes permit downstream deduplication.
pub struct Logs {
    exporter: opentelemetry_otlp::LogExporter,
    provider: SdkLoggerProvider,
    scope: InstrumentationScope,
    timeout: Duration,
}
impl Logs {
    /// Construct an official OTLP logs exporter for the full `/v1/logs` endpoint.
    pub fn new(config: OtelConfig) -> Result<Self> {
        let timeout = Duration::from_millis(config.timeout_ms.into());
        let mut exporter = opentelemetry_otlp::LogExporter::builder()
            .with_http()
            .with_endpoint(config.endpoint)
            .with_timeout(timeout)
            .build()?;
        exporter.set_resource(
            &Resource::builder_empty()
                .with_service_name(config.service_name)
                .build(),
        );
        Ok(Self {
            exporter,
            provider: SdkLoggerProvider::builder().build(),
            scope: InstrumentationScope::builder("fault-watchdog").build(),
            timeout,
        })
    }
}
impl Sink for Logs {
    fn write(&mut self, packet: &Packet) -> Result<()> {
        let frame = fault_protocol::decode_aggregation(&packet.bytes, Default::default())?;
        let logger = self.provider.logger("fault-watchdog");
        let timestamp = frame
            .published_unix_ns
            .map(|n| UNIX_EPOCH + Duration::from_nanos(n))
            .unwrap_or_else(SystemTime::now);
        let (epoch, rows) = match frame.payload {
            AggregationPayload::Updates(b) => (
                b.epoch,
                b.updates
                    .into_iter()
                    .map(|u| {
                        (
                            u.revision,
                            u.source,
                            u.kind.as_str_name().to_owned(),
                            u.connected,
                            u.state,
                            u.observation,
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
            AggregationPayload::Snapshot(v) => (
                v.epoch,
                v.sources
                    .into_iter()
                    .flat_map(|s| {
                        let source = s.source;
                        let connected = s.connected;
                        s.conditions.into_iter().map(move |c| {
                            (
                                v.revision,
                                source,
                                "SNAPSHOT".into(),
                                connected,
                                Some(c),
                                c.latest,
                            )
                        })
                    })
                    .collect(),
            ),
        };
        let mut records = Vec::with_capacity(rows.len());
        for (revision, source, kind, connected, state, observation) in rows {
            let mut r = logger.create_log_record();
            r.set_event_name("fault.publication");
            r.set_timestamp(timestamp);
            r.set_observed_timestamp(SystemTime::now());
            r.add_attribute("fault.epoch", format!("{epoch:032x}"));
            r.add_attribute("fault.revision", revision.to_string());
            r.add_attribute("fault.source", format!("{source:032x}"));
            r.add_attribute("fault.kind", kind);
            r.add_attribute("fault.connected", connected);
            r.set_severity_number(Severity::Info);
            if let Some(s) = state {
                r.add_attribute("fault.condition", i64::from(s.key.condition));
                r.add_attribute(
                    "fault.component",
                    s.definition.component.as_str().to_owned(),
                );
                r.add_attribute("fault.resource", s.definition.resource.as_str().to_owned());
                r.add_attribute("fault.code", s.definition.code.as_str().to_owned());
                r.add_attribute("fault.health", s.health.as_str_name());
                r.add_attribute("fault.freshness", s.freshness.as_str_name());
            }
            if let Some(o) = observation {
                r.set_body(AnyValue::from(o.detail.as_str().to_owned()));
                r.add_attribute("fault.sequence", o.sequence.to_string());
                r.add_attribute("fault.failures", o.failures.to_string());
                r.add_attribute("fault.successes", o.successes.to_string());
                if o.outcome == fault_reporter::Outcome::Failure {
                    r.set_severity_number(match state.map(|s| s.definition.severity) {
                        Some(fault_reporter::Severity::Warning) => Severity::Warn,
                        Some(fault_reporter::Severity::Critical) => Severity::Fatal,
                        _ => Severity::Error,
                    });
                }
                if let Some(t) = o.trace {
                    r.set_trace_context(
                        TraceId::from_bytes(t.trace_id),
                        SpanId::from_bytes(t.span_id),
                        None,
                    );
                }
            }
            records.push(r);
        }
        if !records.is_empty() {
            let batch: Vec<_> = records.iter().map(|r| (r, &self.scope)).collect();
            futures_executor::block_on(self.exporter.export(LogBatch::new(&batch)))?;
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<()> {
        self.exporter.shutdown_with_timeout(self.timeout)?;
        Ok(())
    }
}
