use crate::{
    Result,
    capture::{Config, Core, Handle, Losses, Output, Slot},
    wire::Encoder,
};
use crossbeam_channel::bounded;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, atomic::Ordering},
    thread::{self, JoinHandle},
    time::Duration,
};

/// A borrowed immutable batch; sinks may use it only during `write`.
#[derive(Clone, Copy)]
pub struct BatchView<'a> {
    pub bytes: &'a [u8],
    /// Ascending within one sink: batches it has written plus blocks it has dropped.
    /// A gap means that sink lost blocks; numbers differ between sinks.
    pub sequence: u64,
    pub unix_ns: u64,
}
/// Runs on its own worker. Implementations must bound external I/O timeouts.
pub trait Sink: 'static {
    fn name(&self) -> &str;
    fn write(&mut self, batch: BatchView<'_>) -> Result<()>;
    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}
/// Creates a sink on its worker, for sinks bound to the thread that creates them.
/// Every `Send` sink builds itself; an error fails [`Runtime::start`].
pub trait SinkBuilder: Send + 'static {
    fn build(self: Box<Self>) -> Result<Box<dyn Sink>>;
}
impl<S: Sink + Send> SinkBuilder for S {
    fn build(self: Box<Self>) -> Result<Box<dyn Sink>> {
        Ok(self)
    }
}
#[derive(Debug)]
pub struct SinkReport {
    pub name: String,
    pub written: u64,
    pub dropped: u64,
    pub error: Option<String>,
}
#[derive(Debug)]
pub struct ShutdownReport {
    pub capture: Losses,
    pub sinks: Vec<SinkReport>,
}
const PROMPT_POLL: Duration = Duration::from_millis(2);
/// Runs one sink call, recording its error or panic so the worker keeps releasing blocks.
fn guarded(report: &mut SinkReport, call: impl FnOnce() -> Result<()>) {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => report.error = Some(e.to_string()),
        Err(_) => report.error = Some("sink panicked".into()),
    }
}
/// Serializes and writes one block unless the sink has failed, then releases it.
fn deliver(
    core: &Core,
    sink: &mut dyn Sink,
    encoder: &mut Encoder,
    buffer: &mut Vec<u8>,
    slot: Arc<Slot>,
    report: &mut SinkReport,
    output: &Output,
) {
    if report.error.is_none() {
        let block = slot.block.read().unwrap();
        let sequence = report.written + 1 + output.dropped.load(Ordering::Relaxed) as u64;
        encoder.encode(core, &block, sequence, buffer);
        let unix_ns = core
            .unix_origin
            .saturating_add(block.records.first().map_or(0, |r| r.time));
        let batch = BatchView {
            bytes: buffer,
            sequence,
            unix_ns,
        };
        guarded(report, || sink.write(batch));
    }
    if report.error.is_none() {
        report.written += 1;
    } else {
        output.dropped.fetch_add(1, Ordering::Relaxed);
    }
    core.release(slot);
}
fn run_worker(core: &Core, output: &Output, sink: &mut dyn Sink, report: &mut SinkReport) {
    let mut encoder = Encoder::new(core.config.records_per_block);
    let mut buffer = Vec::new();
    loop {
        // Read first: publishing has stopped, so an empty queue afterward is the end.
        let closed = !core.accepting.load(Ordering::Acquire);
        match output.queue.pop() {
            Some(slot) => deliver(core, sink, &mut encoder, &mut buffer, slot, report, output),
            None if closed => return,
            None => thread::park_timeout(PROMPT_POLL),
        }
    }
}
pub struct Runtime {
    core: Arc<Core>,
    sinks: Vec<JoinHandle<SinkReport>>,
}
impl Runtime {
    /// Starts capture after building each sink on its worker.
    /// Errors: invalid capacities and the first sink build failure; built sinks finish first.
    pub fn start(config: Config, sinks: Vec<Box<dyn SinkBuilder>>) -> Result<Self> {
        let core = Core::new(config)?;
        let mut outputs = Vec::new();
        let mut workers = Vec::new();
        let mut built = Vec::new();
        for builder in sinks {
            let output = Arc::new(Output::new(core.config.sink_queue));
            outputs.push(output.clone());
            let c = core.clone();
            let (ready, ready_rx) = bounded(1);
            built.push(ready_rx);
            workers.push(thread::spawn(move || {
                let mut report = SinkReport {
                    name: String::new(),
                    written: 0,
                    dropped: 0,
                    error: None,
                };
                let mut sink = match builder.build() {
                    Ok(sink) => sink,
                    Err(e) => {
                        let _ = ready.send(Some(e.to_string()));
                        return report;
                    }
                };
                report.name = sink.name().into();
                let _ = output.thread.set(thread::current());
                let _ = ready.send(None);
                run_worker(&c, &output, sink.as_mut(), &mut report);
                if let Err(e) = sink.finish() {
                    report.error.get_or_insert_with(|| e.to_string());
                }
                report.dropped = output.dropped.load(Ordering::Relaxed) as u64;
                report
            }));
        }
        let failure = built.iter().find_map(|ready| {
            ready
                .recv()
                .unwrap_or_else(|_| Some("sink worker panicked".into()))
        });
        if let Some(error) = failure {
            core.accepting.store(false, Ordering::Release);
            for worker in workers {
                let _ = worker.join();
            }
            return Err(error.into());
        }
        assert!(core.outputs.set(outputs).is_ok());
        Ok(Self {
            core,
            sinks: workers,
        })
    }
    pub fn handle(&self) -> Handle {
        Handle(self.core.clone())
    }
    /// Drop all registration guards first. Drains accepted batches and finalizes sinks.
    pub fn shutdown(&mut self) -> Result<ShutdownReport> {
        {
            let _r = self.core.registry.lock().unwrap();
            if self.core.registered.load(Ordering::Acquire) != 0 {
                return Err("drop thread registrations before shutdown".into());
            }
            self.core.accepting.store(false, Ordering::Release);
        }
        self.join()
    }
    fn join(&mut self) -> Result<ShutdownReport> {
        for worker in &self.sinks {
            worker.thread().unpark();
        }
        let mut sinks = Vec::new();
        for worker in self.sinks.drain(..) {
            sinks.push(worker.join().map_err(|_| "sink worker panicked")?);
        }
        Ok(ShutdownReport {
            capture: self.core.losses.snapshot(),
            sinks,
        })
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.core.accepting.store(false, Ordering::Release);
        let _ = self.join();
    }
}
