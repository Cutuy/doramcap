//! Owner-thread sampling and bounded metadata export for the live dashboard.
use crate::metrics::Window;
use serde_json::{Value, json};
use std::{collections::VecDeque, io, time::Duration};
use storage_persistence::{
    mcap::{McapSnapshot, McapWriter},
    observability::{CaptureFilter, Collector, Event, EventKind, Mode},
};

pub struct Setting {
    mode: Mode,
    seconds: u64,
    filter: CaptureFilter,
}
impl Setting {
    pub fn parse(body: &Value) -> Result<Self, ()> {
        let mode = match body["mode"].as_str() {
            Some("Off") => Mode::Off,
            Some("Summary") => Mode::Summary,
            Some("Inspect") => Mode::Inspect,
            Some("Capture") => Mode::Capture,
            _ => return Err(()),
        };
        let seconds = body
            .get("seconds")
            .map_or(Ok(10), |v| v.as_u64().ok_or(()))?;
        if !(1..=60).contains(&seconds) {
            return Err(());
        }
        let id = |key: &str| -> Result<Option<u64>, ()> {
            match body.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::String(s)) if s.is_empty() => Ok(None),
                Some(Value::String(s)) => {
                    s.parse::<u64>().ok().filter(|v| *v > 0).map(Some).ok_or(())
                }
                _ => Err(()),
            }
        };
        Ok(Self {
            mode,
            seconds,
            filter: CaptureFilter {
                writer: id("writer")?,
                write: id("write")?,
                region: id("region")?,
            },
        })
    }
    pub fn apply(self, collector: &Collector) -> io::Result<()> {
        if self.mode == Mode::Capture {
            collector.capture(Duration::from_secs(self.seconds), self.filter)
        } else {
            collector.set_mode(self.mode)
        }
    }
}
pub struct Observation {
    pub collector: Collector,
    cursor: u64,
    pub lost: u64,
    pub events: VecDeque<Value>,
    contention: u64,
    overwritten: u64,
    evicted: u64,
}
impl Observation {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            collector: Collector::new(4096, Mode::Inspect)?,
            cursor: 0,
            lost: 0,
            events: VecDeque::with_capacity(256),
            contention: 0,
            overwritten: 0,
            evicted: 0,
        })
    }
    pub fn poll(&mut self, latency: &mut Window) {
        let batch = self.collector.read_after(self.cursor, 4096);
        self.cursor = batch.next_cursor;
        self.lost += batch.missed_before_cursor;
        self.contention = batch.contention_drops;
        self.overwritten = batch.overwritten_total;
        for event in batch.events {
            if event.kind == EventKind::CqeObserved
                && event.data.result.is_some_and(|r| r >= 0)
                && let Some(us) = event.data.duration_us
            {
                latency.push(us as f64 / 1000.0);
            }
            if self.events.len() == 256 {
                self.events.pop_front();
                self.evicted += 1;
            }
            self.events.push_back(event_json(event));
        }
    }
    pub fn json(&self) -> Value {
        json!({"mode":format!("{:?}",self.collector.mode()),"cursor":self.cursor.to_string(),"missed":self.lost,"contentionDrops":self.contention,"overwritten":self.overwritten,"timelineEvicted":self.evicted,"capacity":4096})
    }
}
fn event_json(e: Event) -> Value {
    let i = e.data.ids;
    json!({"id":e.sequence.to_string(),"kind":format!("{:?}",e.kind),"us":e.elapsed_ns/1000,
        "writer":i.writer.to_string(),"file":i.file.to_string(),"write":i.write.to_string(),"operation":i.operation.to_string(),"region":i.region.to_string(),"slabUse":i.slab_use.to_string(),"pool":i.pool.to_string(),
        "offset":e.data.offset,"len":e.data.len,"result":e.data.result,"latencyUs":e.data.duration_us})
}
pub fn combining(d: McapSnapshot) -> Value {
    json!({"used":d.combining_bytes,"messages":d.combining_messages,"capacity":d.combining_capacity,"ageMs":d.combining_age_us.map(|v|v as f64/1000.0),"deadlineMs":100})
}
pub fn pool(d: McapSnapshot) -> Value {
    json!({"free":d.managed_available,"total":d.managed_capacity,"allocatedBytes":d.managed_allocated_bytes,"freeBytes":d.managed_free_bytes,"inFlightBytes":d.writer.pending_bytes,"registeredBytes":d.writer.registered_bytes})
}
pub fn ring(d: McapSnapshot) -> Value {
    let w = d.writer;
    let c = w.counters;
    json!({"writer":w.writer_id.to_string(),"file":w.file_id.to_string(),"pendingCapacity":w.pending_capacity,"outstanding":w.pending,"submitted":c.queued_operations,"completions":c.observed_completions,"accepted":w.accepted_bytes,"completed":w.written_bytes,"synced":w.synced_bytes,
        "awaitingSubmission":w.awaiting_submission,"awaitingCqe":w.awaiting_cqe,"pendingBytes":w.pending_bytes,"oldestUs":w.oldest_age_us,"registeredRegions":w.registered_regions,"registrationUpdates":w.registration_updates,
        "acceptedWrites":c.accepted,"written":c.written,"failed":c.failed,"cancelled":c.cancelled,"rejectedCapacity":c.rejected_capacity,"rejectedInvalid":c.rejected_invalid,"rejectedClosed":c.rejected_closed,"rejectedOther":c.rejected_other,"retries":c.short_retries,"registrationHits":c.registration_hits,"registrationMisses":c.registration_misses,"registrationFailures":c.registration_failures,"syncFailures":c.sync_failures,"submitFailures":c.submit_failures})
}
pub fn active(writer: &McapWriter) -> Value {
    json!(writer.active_writes().map(|w|json!({"write":w.ids.write.to_string(),"operation":w.ids.operation.to_string(),"region":w.ids.region.to_string(),"slabUse":w.ids.slab_use.to_string(),"state":format!("{:?}",w.state),"offset":w.offset,"len":w.len,"completed":w.completed_bytes,"ageUs":w.age_us,"attempts":w.attempts,"retries":w.retries})).collect::<Vec<_>>())
}

// One process-wide worker uses an independent stderr subscriber. A stalled sink
// holds at most one batch; restarts replace the latest collector without joining.
#[cfg(feature = "tracing-export")]
pub struct TraceExport;
#[cfg(feature = "tracing-export")]
impl TraceExport {
    pub fn start(collector: Collector) {
        use std::sync::{
            OnceLock,
            atomic::{AtomicU64, Ordering},
        };
        static EXPORT: OnceLock<tokio::sync::watch::Sender<(u64, Collector)>> = OnceLock::new();
        static GENERATION: AtomicU64 = AtomicU64::new(1);
        if std::env::var_os("STORAGE_TRACE_STDERR").is_none() {
            return;
        }
        let generation = GENERATION.fetch_add(1, Ordering::Relaxed);
        let sender = EXPORT.get_or_init(|| {
            let (sender, mut receiver) = tokio::sync::watch::channel((generation, collector.clone()));
            std::thread::spawn(move || {
                let subscriber=tracing_subscriber::fmt().with_writer(std::io::stderr).with_ansi(false).finish();
                tracing::subscriber::with_default(subscriber, || {
                    let mut cursor=0;
                    let mut seen_generation=0;
                    loop {
                        let (generation, collector)=receiver.borrow_and_update().clone();
                        if generation != seen_generation { cursor=0; seen_generation=generation; }
                        let batch=collector.read_after(cursor,256);
                        cursor=batch.next_cursor;
                        for e in batch.events {
                            tracing::info!(target:"storage_observation",sequence=e.sequence,kind=?e.kind,ids=?e.data.ids,offset=e.data.offset,len=e.data.len,result=?e.data.result,duration_us=?e.data.duration_us);
                        }
                        if batch.missed_before_cursor>0 { tracing::warn!(target:"storage_observation",missed=batch.missed_before_cursor,"history gap"); }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                });
            });
            sender
        });
        sender.send_replace((generation, collector));
    }
}

/// Capture on the writer thread so timestamps describe the operations themselves.
#[cfg(feature = "activity-tracing")]
pub fn capture_activity(path: Option<&std::path::Path>, run: impl FnOnce()) -> crate::Result<()> {
    let Some(path) = path else {
        run();
        return Ok(());
    };
    let mut runtime = activity_tracing::Runtime::start(
        activity_tracing::Config::default(),
        vec![Box::new(activity_tracing::sinks::McapSink::create(path)?)],
    )?;
    let handle = runtime.handle();
    let registration = handle.register_thread("storage-writer")?;
    tracing::subscriber::with_default(handle.subscriber(), run);
    drop(registration);
    let report = runtime.shutdown()?;
    eprintln!("storage activity capture: {report:?}");
    if report.sinks.iter().any(|sink| sink.error.is_some()) {
        return Err("activity capture output failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage_persistence::observability::EventData;
    #[test]
    fn bounded_timeline_reports_loss_and_control_validation() {
        let mut o = Observation::new().unwrap();
        for _ in 0..5000 {
            o.collector
                .observer()
                .record(EventKind::Accepted, EventData::default);
        }
        o.poll(&mut Window::default());
        assert_eq!(o.events.len(), 256);
        assert_eq!(o.lost, 904);
        assert_eq!(o.json()["timelineEvicted"], 3840);
        assert!(Setting::parse(&json!({"mode":"Capture","seconds":0})).is_err());
        assert!(Setting::parse(&json!({"mode":"Capture","region":"oops"})).is_err());
        Setting::parse(&json!({"mode":"Off"}))
            .unwrap()
            .apply(&o.collector)
            .unwrap();
        assert_eq!(o.collector.mode(), Mode::Off);
    }
}
