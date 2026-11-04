use crate::{
    Result,
    capture::{Block, Config, Core, Handle, Losses, Output, Slot},
    wire::Encoder,
};
use crossbeam_channel::bounded;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, RwLockReadGuard, atomic::Ordering},
    thread::{self, JoinHandle},
    time::Duration,
};

/// A borrowed immutable batch; sinks may use it only during `write`.
#[derive(Clone, Copy)]
pub struct BatchView<'a> {
    pub bytes: &'a [u8],
    /// Ascending within one sink: messages it has written plus blocks it has dropped.
    /// A gap means that sink lost blocks; numbers differ between sinks.
    pub sequence: u64,
    pub unix_ns: u64,
}
/// Published blocks a sink's worker took from its queue, in publish order.
pub struct Blocks<'a> {
    core: &'a Core,
    encoder: &'a mut Encoder,
    buffer: &'a mut Vec<u8>,
    blocks: &'a [RwLockReadGuard<'a, Block>],
    output: &'a Output,
    /// Messages the sink wrote before this call.
    earlier: u64,
    written: u64,
    done: usize,
}
impl Blocks<'_> {
    /// Numbers the next message; see [`BatchView::sequence`].
    fn sequence(&self) -> u64 {
        self.earlier + self.written + 1 + self.output.dropped.load(Ordering::Relaxed) as u64
    }
    /// Encodes one self-contained protobuf `Batch` per block and passes each to `write`.
    /// Errors: stops at the first `write` error and returns it.
    pub fn for_each_batch(
        &mut self,
        mut write: impl FnMut(BatchView<'_>) -> Result<()>,
    ) -> Result<()> {
        while self.done < self.blocks.len() {
            let blocks = &self.blocks[self.done..=self.done];
            let sequence = self.sequence();
            let plan = self.encoder.plan(self.core, blocks, sequence, usize::MAX);
            self.buffer.resize(plan.len, 0);
            self.encoder.write(self.buffer);
            write(BatchView {
                bytes: self.buffer,
                sequence: plan.sequence,
                unix_ns: plan.unix_ns,
            })?;
            self.written += 1;
            self.done += 1;
        }
        Ok(())
    }
    /// Measures one message merging as many remaining blocks as fit `limit` bytes.
    /// A single larger block forms its own message. Returns `None` when none remain.
    #[cfg(feature = "mcap")]
    pub(crate) fn plan(&mut self, limit: usize) -> Option<crate::wire::Plan> {
        let sequence = self.sequence();
        let blocks = self.blocks.get(self.done..).filter(|b| !b.is_empty())?;
        Some(self.encoder.plan(self.core, blocks, sequence, limit))
    }
    /// Serializes the latest batch from [`Self::plan`] into `out`.
    #[cfg(feature = "mcap")]
    pub(crate) fn encode(&self, out: &mut [u8]) {
        self.encoder.write(out);
    }
    /// Marks `plan`'s blocks as written.
    #[cfg(feature = "mcap")]
    pub(crate) fn commit(&mut self, plan: &crate::wire::Plan) {
        self.written += 1;
        self.done += plan.blocks;
    }
}
/// Runs on its own worker. Implementations must bound external I/O timeouts.
pub trait Sink: 'static {
    fn name(&self) -> &str;
    fn write(&mut self, batch: BatchView<'_>) -> Result<()>;
    /// Handles newly published blocks. By default, each block becomes one [`Self::write`].
    fn write_blocks(&mut self, blocks: &mut Blocks<'_>) -> Result<()> {
        blocks.for_each_batch(|batch| self.write(batch))
    }
    /// Period for collecting blocks between calls. `None` delivers promptly after each publish.
    fn batch_interval(&self) -> Option<Duration> {
        None
    }
    /// Runs on every worker wake, including idle ones.
    fn poll(&mut self) -> Result<()> {
        Ok(())
    }
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
/// Serializes and writes the blocks in `taken`, then releases them.
fn deliver(
    core: &Core,
    sink: &mut dyn Sink,
    encoder: &mut Encoder,
    buffer: &mut Vec<u8>,
    taken: &mut Vec<Arc<Slot>>,
    report: &mut SinkReport,
    output: &Output,
) {
    let total = taken.len();
    let mut done = 0;
    if report.error.is_none() {
        let guards: Vec<_> = taken.iter().map(|s| s.block.read().unwrap()).collect();
        let mut blocks = Blocks {
            core,
            encoder,
            buffer,
            blocks: &guards,
            output,
            earlier: report.written,
            written: 0,
            done: 0,
        };
        guarded(report, || sink.write_blocks(&mut blocks));
        report.written += blocks.written;
        done = blocks.done;
    }
    output.dropped.fetch_add(total - done, Ordering::Relaxed);
    for slot in taken.drain(..) {
        core.release(slot);
    }
}
fn run_worker(core: &Core, output: &Output, sink: &mut dyn Sink, report: &mut SinkReport) {
    let mut encoder = Encoder::new(core.config.records_per_block);
    let mut buffer = Vec::new();
    let mut taken = Vec::with_capacity(core.config.sink_queue);
    let interval = sink.batch_interval();
    let park = interval.unwrap_or(PROMPT_POLL);
    // Without batching, take one block at a time so each returns to the pool promptly.
    let take = interval.map_or(1, |_| core.config.sink_queue);
    loop {
        // Read first: publishing has stopped, so an empty queue afterward is the end.
        let closed = !core.accepting.load(Ordering::Acquire);
        while taken.len() < take
            && let Some(slot) = output.queue.pop()
        {
            taken.push(slot);
        }
        let idle = taken.is_empty();
        if !idle {
            deliver(
                core,
                sink,
                &mut encoder,
                &mut buffer,
                &mut taken,
                report,
                output,
            );
        }
        if report.error.is_none() {
            guarded(report, || sink.poll());
        }
        if closed && idle {
            return;
        }
        if output.queue.is_empty() && !closed {
            thread::park_timeout(park);
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
                // A batching sink wakes early at half a queue so bursts are not dropped.
                if sink.batch_interval().is_some() {
                    let half = (c.config.sink_queue / 2).max(1);
                    output.wake_at.store(half, Ordering::Relaxed);
                }
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
