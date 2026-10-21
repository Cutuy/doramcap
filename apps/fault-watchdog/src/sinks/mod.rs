//! Independent bounded output queues. Blocking adapters execute on their owner
//! threads; queue admission never waits. Failed deliveries request reconciliation.
#[cfg(feature = "mcap")]
pub mod mcap;
#[cfg(feature = "otel")]
pub mod otel;
use crate::{
    config::WatchdogConfig,
    engine::{detail, ms},
};
use eyre::Result;
use fault_protocol::{AggregationFrame, AggregationPayload, Limits};
use fault_reporter::{Check, Key};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
/// Encoded domain publication retained for bounded retry. Epoch/revision in its
/// payload identify duplicate delivery; retries provide at-least-once attempts.
pub struct Packet {
    pub bytes: Vec<u8>,
}
/// Blocking adapter owned exclusively by one sink executor.
/// `write` success means adapter acceptance; `finish` is its finalization barrier.
pub trait Sink {
    fn write(&mut self, packet: &Packet) -> Result<()>;
    fn tick(&mut self) -> Result<()> {
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}
/// Cumulative per-sink operational counters, independent of domain state.
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub delivered: u64,
    pub dropped: u64,
    pub errors: u64,
    pub last_error: Option<String>,
    pub needs_snapshot: bool,
}
/// Nonblocking publication endpoint. Drop closes admission; use `finish` to wait
/// for draining/finalization with a shared host shutdown deadline.
pub struct Output {
    sender: crossbeam_channel::Sender<Packet>,
    done: crossbeam_channel::Receiver<Result<()>>,
    stats: Arc<Mutex<Stats>>,
    check: Check,
    limits: Limits,
    observed: Mutex<(u64, u64)>,
}
impl Output {
    /// Start an adapter inside its owner thread, including initialization. Startup
    /// failure remains visible through diagnostics and finalization status.
    pub fn spawn(
        name: &str,
        config: &WatchdogConfig,
        limits: Limits,
        check: Check,
        factory: impl FnOnce() -> Result<Box<dyn Sink>> + Send + 'static,
    ) -> Result<Self> {
        let (sender, receiver) =
            crossbeam_channel::bounded::<Packet>(config.queue_capacity.unwrap() as usize);
        let (done_tx, done) = crossbeam_channel::bounded(1);
        let stats = Arc::new(Mutex::new(Stats::default()));
        let shared = stats.clone();
        let attempts = config.retry_count.unwrap() + 1;
        let delay = ms(config.retry_delay_ms.unwrap());
        let interval = ms(config.maintenance_ms.unwrap());
        std::thread::Builder::new()
            .name(format!("fault-{name}"))
            .spawn(move || {
                let result = (|| -> Result<()> {
                    let mut sink = factory()?;
                    loop {
                        match receiver.recv_timeout(interval) {
                            Ok(packet) => {
                                let mut delivered = false;
                                for attempt in 0..attempts {
                                    match sink.write(&packet) {
                                        Ok(()) => {
                                            delivered = true;
                                            break;
                                        }
                                        Err(e) => {
                                            record_error(&shared, &e);
                                            if attempt + 1 < attempts {
                                                std::thread::sleep(delay);
                                            }
                                        }
                                    }
                                }
                                let mut s = shared.lock().unwrap();
                                if delivered {
                                    s.delivered += 1;
                                    s.last_error = None;
                                } else {
                                    s.dropped += 1;
                                    s.needs_snapshot = true;
                                }
                            }
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                        }
                        if let Err(e) = sink.tick() {
                            record_error(&shared, &e);
                        }
                    }
                    sink.finish()
                })();
                if let Err(e) = &result {
                    record_error(&shared, e);
                }
                let _ = done_tx.send(result);
            })?;
        Ok(Self {
            sender,
            done,
            stats,
            check,
            limits,
            observed: Mutex::new((0, 0)),
        })
    }
    /// Offer a frame immediately, removing this sink's diagnostic condition to
    /// prevent feedback recursion. Full/closed queues count loss and request a snapshot.
    pub fn offer(&self, frame: &AggregationFrame) -> Result<bool> {
        let frame = filtered(frame, self.check.key());
        let bytes = match fault_protocol::encode_aggregation(&frame, self.limits) {
            Ok(bytes) => bytes,
            Err(e) => {
                record_error(&self.stats, &e);
                return Ok(false);
            }
        };
        match self.sender.try_send(Packet { bytes }) {
            Ok(()) => Ok(true),
            Err(_) => {
                let mut s = self.stats.lock().unwrap();
                s.dropped += 1;
                s.needs_snapshot = true;
                Ok(false)
            }
        }
    }
    /// Clear the repair request before capture/admission so a concurrent worker
    /// failure cannot be erased by completion of a snapshot publication.
    pub fn take_repair(&self) -> bool {
        let mut s = self.stats.lock().unwrap();
        std::mem::take(&mut s.needs_snapshot)
    }
    /// Read operational diagnostics and feed the local registered condition.
    pub fn observe(&self) -> Result<Stats> {
        let s = self.stats.lock().unwrap().clone();
        let mut previous = self.observed.lock().unwrap();
        if (s.errors, s.dropped) != *previous {
            self.check.failure(detail(&format!(
                "errors={} dropped={}: {}",
                s.errors,
                s.dropped,
                s.last_error.as_deref().unwrap_or("output queue loss")
            )))?;
            *previous = (s.errors, s.dropped);
        } else if s.last_error.is_none() && !s.needs_snapshot {
            self.check.success()?;
        }
        Ok(s)
    }
    /// Close admission immediately. Close every output before waiting for any
    /// completion so a blocked adapter cannot delay another adapter's finalization.
    pub fn close(self) -> Completion {
        drop(self.sender);
        Completion {
            done: self.done,
            stats: self.stats,
        }
    }
    /// Close one output and wait for finalization. Hosts with multiple outputs
    /// should call `close` on all outputs first, then wait on their completions.
    pub fn finish(self, deadline: Instant) -> Result<()> {
        self.close().finish(deadline)
    }
}
/// Finalization handle after publication admission has closed.
/// Waiting does not hold another sink open. Timeout detaches an in-flight OS call.
pub struct Completion {
    done: crossbeam_channel::Receiver<Result<()>>,
    stats: Arc<Mutex<Stats>>,
}
impl Completion {
    /// Wait until the shared host deadline. A completed result remains readable
    /// after the deadline; unfinished or persistently failed output returns an error.
    pub fn finish(self, deadline: Instant) -> Result<()> {
        self.done
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| eyre::eyre!("sink finalization deadline exceeded"))??;
        let stats = self.stats.lock().unwrap();
        if stats.dropped > 0 {
            eprintln!(
                "sink shutdown: {} dropped publications, {} errors",
                stats.dropped, stats.errors
            );
        }
        eyre::ensure!(
            stats.last_error.is_none(),
            "sink remains failed: {:?}",
            stats.last_error
        );
        Ok(())
    }
}

fn record_error(stats: &Mutex<Stats>, e: &impl std::fmt::Display) {
    let mut s = stats.lock().unwrap();
    s.errors += 1;
    if s.errors.is_power_of_two() {
        eprintln!("fault sink error ({}): {}", s.errors, detail(e).as_str());
    }
    s.last_error = Some(detail(e).as_str().into());
    s.needs_snapshot = true;
}
fn filtered(frame: &AggregationFrame, key: Key) -> AggregationFrame {
    let mut f = frame.clone();
    match &mut f.payload {
        AggregationPayload::Updates(b) => b.updates.retain(|u| u.key != Some(key)),
        AggregationPayload::Snapshot(v) => {
            for source in &mut v.sources {
                source.conditions.retain(|s| s.key != key);
            }
        }
    }
    f
}
/// Live domain publications through the configured Dora output.
pub struct DoraSink {
    pub node: dora_node_api::DoraNode,
    pub output: String,
}
impl Sink for DoraSink {
    fn write(&mut self, packet: &Packet) -> Result<()> {
        self.node.send_output_raw(
            self.output.clone().into(),
            Default::default(),
            packet.bytes.len(),
            |buffer| buffer.copy_from_slice(&packet.bytes),
        )?;
        Ok(())
    }
}
