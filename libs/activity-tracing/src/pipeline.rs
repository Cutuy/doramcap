use crate::{
    Result,
    capture::{Config, Core, Handle, Losses},
    wire::Encoder,
};
use crossbeam_channel::{Sender, bounded};
use crossbeam_queue::ArrayQueue;
use std::{
    sync::{
        Arc, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

/// A borrowed immutable batch; sinks may use it only during `write`.
#[derive(Clone, Copy)]
pub struct BatchView<'a> {
    pub bytes: &'a [u8],
    pub sequence: u64,
    pub unix_ns: u64,
}
/// Runs on its own worker. Implementations must bound external I/O timeouts.
pub trait Sink: Send + 'static {
    fn name(&self) -> &str;
    fn write(&mut self, batch: BatchView<'_>) -> Result<()>;
    fn finish(&mut self) -> Result<()> {
        Ok(())
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
struct Data {
    bytes: Vec<u8>,
    sequence: u64,
    unix_ns: u64,
}
struct Slot {
    data: RwLock<Data>,
    readers: AtomicUsize,
}
pub(crate) struct Pool {
    free: ArrayQueue<Arc<Slot>>,
}
impl Pool {
    pub(crate) fn available(&self) -> usize {
        self.free.len()
    }
}
struct Lease {
    slot: Arc<Slot>,
    pool: Arc<Pool>,
}
impl Clone for Lease {
    fn clone(&self) -> Self {
        self.slot.readers.fetch_add(1, Ordering::Relaxed);
        Self {
            slot: self.slot.clone(),
            pool: self.pool.clone(),
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if self.slot.readers.fetch_sub(1, Ordering::AcqRel) == 1 {
            assert!(self.pool.free.push(self.slot.clone()).is_ok());
        }
    }
}
struct Output {
    tx: Sender<Lease>,
    dropped: Arc<AtomicUsize>,
}
pub struct Runtime {
    core: Arc<Core>,
    encoder: Option<JoinHandle<()>>,
    sinks: Vec<JoinHandle<SinkReport>>,
    drop_counts: Vec<Arc<AtomicUsize>>,
}
impl Runtime {
    pub fn start(config: Config, sinks: Vec<Box<dyn Sink>>) -> Result<Self> {
        let required = sinks
            .len()
            .checked_mul(
                config
                    .sink_queue
                    .checked_add(1)
                    .ok_or("queue capacity overflow")?,
            )
            .and_then(|n| n.checked_add(1))
            .ok_or("pool capacity overflow")?;
        if config.encoded_buffers < required {
            return Err(
                "encoded_buffers must cover every sink queue, in-flight writer, and encoder".into(),
            );
        }
        let core = Core::new(config)?;
        let pool = Arc::new(Pool {
            free: ArrayQueue::new(core.config.encoded_buffers),
        });
        for _ in 0..core.config.encoded_buffers {
            assert!(
                pool.free
                    .push(Arc::new(Slot {
                        data: RwLock::new(Data {
                            bytes: {
                                let mut v = vec![0; core.config.encoded_capacity().unwrap()];
                                v.clear();
                                v
                            },
                            sequence: 0,
                            unix_ns: 0
                        }),
                        readers: AtomicUsize::new(0)
                    }))
                    .is_ok()
            );
        }
        assert!(core.encoded_pool.set(Arc::downgrade(&pool)).is_ok());
        let mut outputs = Vec::new();
        let mut workers = Vec::new();
        let mut drop_counts = Vec::new();
        for mut sink in sinks {
            let (tx, rx) = bounded::<Lease>(core.config.sink_queue);
            let dropped = Arc::new(AtomicUsize::new(0));
            let count = dropped.clone();
            drop_counts.push(dropped.clone());
            workers.push(thread::spawn(move || {
                let mut report = SinkReport {
                    name: sink.name().into(),
                    written: 0,
                    dropped: 0,
                    error: None,
                };
                for lease in rx {
                    if report.error.is_some() {
                        count.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    let d = lease.slot.data.read().unwrap();
                    match sink.write(BatchView {
                        bytes: &d.bytes,
                        sequence: d.sequence,
                        unix_ns: d.unix_ns,
                    }) {
                        Ok(()) => report.written += 1,
                        Err(e) => {
                            report.error = Some(e.to_string());
                            count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                if let Err(e) = sink.finish() {
                    report.error.get_or_insert_with(|| e.to_string());
                }
                report.dropped = count.load(Ordering::Relaxed) as u64;
                report
            }));
            outputs.push(Output { tx, dropped });
        }
        let c = core.clone();
        let encoder = thread::spawn(move || {
            let mut encoder = Encoder::new(c.config.records_per_block);
            let mut sequence = 0;
            loop {
                while let Some(block) = c.ready.pop() {
                    sequence += 1;
                    if let Some(slot) = pool.free.pop() {
                        {
                            let mut data = slot.data.write().unwrap();
                            encoder.encode(&c, &block, sequence, &mut data.bytes);
                            data.sequence = sequence;
                            data.unix_ns = c
                                .unix_origin
                                .saturating_add(block.records.first().map_or(0, |r| r.time));
                        }
                        slot.readers.store(1, Ordering::Release);
                        let lease = Lease {
                            slot,
                            pool: pool.clone(),
                        };
                        for output in &outputs {
                            if output.tx.try_send(lease.clone()).is_err() {
                                output.dropped.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    } else {
                        c.losses.encoded_batches.fetch_add(1, Ordering::Relaxed);
                    }
                    c.recycle(block);
                }
                if !c.accepting.load(Ordering::Acquire) {
                    break;
                }
                thread::park_timeout(Duration::from_millis(2));
            }
        });
        core.wake.set(encoder.thread().clone()).unwrap();
        Ok(Self {
            core,
            encoder: Some(encoder),
            sinks: workers,
            drop_counts,
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
        if let Some(t) = self.core.wake.get() {
            t.unpark();
        }
        if let Some(j) = self.encoder.take() {
            j.join().map_err(|_| "encoder worker panicked")?;
        }
        let mut sinks = Vec::new();
        for (i, j) in self.sinks.drain(..).enumerate() {
            let mut report = j.join().map_err(|_| "sink worker panicked")?;
            report.dropped = self.drop_counts[i].load(Ordering::Relaxed) as u64;
            sinks.push(report);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_last_reader_recycles() {
        let pool = Arc::new(Pool {
            free: ArrayQueue::new(1),
        });
        let slot = Arc::new(Slot {
            data: RwLock::new(Data {
                bytes: Vec::new(),
                sequence: 0,
                unix_ns: 0,
            }),
            readers: AtomicUsize::new(1),
        });
        let a = Lease {
            slot,
            pool: pool.clone(),
        };
        let b = a.clone();
        let c = a.clone();
        drop(a);
        drop(b);
        assert!(pool.free.is_empty());
        drop(c);
        assert_eq!(pool.free.len(), 1);
    }
}
