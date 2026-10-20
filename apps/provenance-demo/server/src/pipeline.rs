use crate::{envelope, node_id, pb, publish};
use activity_tracing::{BatchView, Config, Handle, Runtime, Sink};
use crossbeam_channel::{Receiver, Sender, bounded};
use dora_node_api::{DoraNode, Event, TryRecvError};
use eyre::{Result, bail, eyre};
use prost::Message;
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use step_provenance::pb::{GenerationRef, Inputs, StepUpdate};

struct ActivitySink {
    tx: Sender<Vec<u8>>,
    drops: Arc<AtomicU64>,
}
impl Sink for ActivitySink {
    fn name(&self) -> &str {
        "dora-demo"
    }
    fn write(&mut self, batch: BatchView<'_>) -> activity_tracing::Result<()> {
        if self.tx.try_send(batch.bytes.to_vec()).is_err() {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
}
struct Shared {
    stop: AtomicBool,
    drained: AtomicBool,
    control_idle: AtomicBool,
    queue_drops: Arc<AtomicU64>,
    report_drops: AtomicU64,
    deadline: Instant,
    drop_every: u64,
    report_delay: Duration,
}
pub fn run(args: &[String]) -> Result<()> {
    let mut duration = 3600;
    let mut drop_every = 0;
    let mut report_delay = 0;
    for pair in args.chunks(2) {
        let value = pair.get(1).ok_or_else(|| eyre!("missing option value"))?;
        match pair[0].as_str() {
            "--duration" => duration = value.parse()?,
            "--drop-every" => drop_every = value.parse()?,
            "--report-delay-ms" => report_delay = value.parse::<u64>()?.min(1000),
            _ => bail!("unknown pipeline option {}", pair[0]),
        }
    }
    let (tx, rx) = bounded(64);
    let drops = Arc::new(AtomicU64::new(0));
    let mut runtime = Runtime::start(
        Config {
            blocks: 128,
            encoded_buffers: 68,
            sink_queue: 32,
            ..Default::default()
        },
        vec![Box::new(ActivitySink {
            tx,
            drops: drops.clone(),
        })],
    )
    .map_err(|e| eyre!(e.to_string()))?;
    let handle = runtime.handle();
    let shared = Arc::new(Shared {
        stop: AtomicBool::new(false),
        drained: AtomicBool::new(false),
        control_idle: AtomicBool::new(false),
        queue_drops: drops,
        report_drops: AtomicU64::new(0),
        deadline: Instant::now() + Duration::from_secs(duration),
        drop_every,
        report_delay: Duration::from_millis(report_delay),
    });
    let signal = shared.clone();
    let signal_task = tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        signal.stop.store(true, Ordering::Release);
    });
    let mut workers = Vec::new();
    for name in ["camera", "lidar", "detect", "control"] {
        let h = handle.clone();
        let s = shared.clone();
        let rx = rx.clone();
        workers.push(thread::Builder::new().name(name.into()).spawn(move || {
            let result = worker(name, h, s.clone(), rx);
            if result.is_err() {
                s.stop.store(true, Ordering::Release);
            }
            if name == "control" {
                s.control_idle.store(true, Ordering::Release);
            }
            result
        })?);
    }
    let control = workers.pop().unwrap();
    let mut error = None;
    for worker in workers {
        if let Err(e) = worker
            .join()
            .unwrap_or_else(|_| Err(eyre!("pipeline thread panicked")))
        {
            error.get_or_insert(e);
        }
    }
    shared.stop.store(true, Ordering::Release);
    while !shared.control_idle.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(2));
    }
    let shutdown = runtime.shutdown().map_err(|e| eyre!(e.to_string()));
    shared.drained.store(true, Ordering::Release);
    if let Err(e) = control
        .join()
        .unwrap_or_else(|_| Err(eyre!("control thread panicked")))
    {
        error.get_or_insert(e);
    }
    signal_task.abort();
    let shutdown = shutdown?;
    println!(
        "pipeline stopped: {:?}; activity queue drops={}",
        shutdown.capture,
        shared.queue_drops.load(Ordering::Relaxed)
    );
    if let Some(e) = error {
        return Err(e);
    }
    Ok(())
}
fn worker(
    name: &'static str,
    handle: Handle,
    shared: Arc<Shared>,
    batches: Receiver<Vec<u8>>,
) -> Result<()> {
    // Dynamic IDs allow independent connections without mutating process environment.
    let (mut node, mut events) = DoraNode::init_from_node_id(node_id(name).into())?;
    println!("READY {name} pid={}", std::process::id());
    let clock = handle.clock();
    let epoch = format!("{:016x}", clock.session);
    let domain = format!("activity/{epoch}");
    let status = || {
        envelope(pb::envelope::Body::Status(pb::NodeStatus {
            node: name.into(),
            pid: std::process::id(),
            epoch: epoch.clone(),
            activity_queue_drops: shared.queue_drops.load(Ordering::Relaxed),
            simulated_report_drops: shared.report_drops.load(Ordering::Relaxed),
        }))
    };
    publish(&mut node, "telemetry", &status())?;
    let dispatch = tracing::Dispatch::new(handle.subscriber());
    tracing::dispatcher::with_default(&dispatch, || -> Result<()> {
        let registration = handle
            .register_thread(name)
            .map_err(|e| eyre!(e.to_string()))?;
        let mut generation = 0;
        let mut lidar = None;
        let mut tick = Instant::now();
        let mut heartbeat = Instant::now();
        let mut delayed = VecDeque::new();
        let mut transport_open = true;
        while !shared.stop.load(Ordering::Acquire) && Instant::now() < shared.deadline {
            let mut inputs = Vec::new();
            for _ in 0..64 {
                match events.try_recv() {
                    Ok(Event::Input { id, data, .. }) => {
                        if data.null_count() != 0 {
                            bail!("null payload bytes");
                        }
                        let bytes: &[u8] = (&data).try_into()?;
                        eyre::ensure!(bytes.len() <= crate::MAX_FRAME, "oversized payload");
                        let payload = pb::Payload::decode(bytes)?;
                        let r = payload
                            .generation
                            .ok_or_else(|| eyre!("missing payload reference"))?;
                        if name == "detect" && id.as_str() == "lidar" {
                            lidar = Some(r);
                        } else if name == "detect" && id.as_str() == "camera" {
                            if let Some(l) = &lidar {
                                inputs.push(vec![r, l.clone()]);
                            }
                        } else if name == "control" && id.as_str() == "objects" {
                            inputs.push(vec![r]);
                        }
                    }
                    Ok(Event::Stop(_)) | Err(TryRecvError::Closed) => {
                        shared.stop.store(true, Ordering::Release);
                        transport_open = false;
                        break;
                    }
                    Ok(Event::Error(e)) => bail!("Dora error: {e}"),
                    Err(TryRecvError::Empty) => break,
                    _ => {}
                }
            }
            if !transport_open {
                break;
            }
            if (name == "camera" || name == "lidar") && Instant::now() >= tick {
                inputs.push(vec![]);
                tick =
                    Instant::now() + Duration::from_millis(if name == "camera" { 50 } else { 100 });
            }
            for input_refs in inputs {
                generation += 1;
                let r = GenerationRef {
                    producer: name.into(),
                    epoch: epoch.clone(),
                    step: name.into(),
                    generation: generation.to_string(),
                    sequence: generation,
                };
                let span = tracing::info_span!(target: "activity::provenance", "step", producer = name, epoch = epoch.as_str(), step = name, generation, sequence = generation);
                let entered = span.enter();
                let start = handle.clock().elapsed_ns;
                let mut update = StepUpdate {
                    version: 1,
                    generation: Some(r.clone()),
                    clock_domain: domain.clone(),
                    invocation: String::new(),
                    start_ns: Some(start),
                    end_ns: None,
                    inputs: Some(Inputs { refs: input_refs }),
                };
                let drop_report = name == "camera"
                    && shared.drop_every != 0
                    && generation % shared.drop_every == 0;
                if !drop_report {
                    let message = envelope(pb::envelope::Body::Step(update.clone()));
                    if shared.report_delay.is_zero() {
                        publish(&mut node, "telemetry", &message)?;
                    } else if delayed.len() < 128 {
                        delayed.push_back((Instant::now() + shared.report_delay, message));
                    } else {
                        shared.report_drops.fetch_add(1, Ordering::Relaxed);
                    }
                } else {
                    shared.report_drops.fetch_add(1, Ordering::Relaxed);
                }
                {
                    let _work =
                        tracing::info_span!(target: "activity::provenance", "compute").entered();
                    thread::sleep(Duration::from_millis(match name {
                        "detect" => 12,
                        "control" => 4,
                        _ => 1,
                    }));
                }
                let end = handle.clock().elapsed_ns;
                update.start_ns = None;
                update.end_ns = Some(end);
                update.inputs = None;
                if !drop_report {
                    publish(
                        &mut node,
                        "telemetry",
                        &envelope(pb::envelope::Body::Step(update)),
                    )?;
                }
                publish(
                    &mut node,
                    "data",
                    &pb::Payload {
                        generation: Some(r),
                        produced_ns: end,
                    },
                )?;
                drop(entered);
                drop(span);
                registration.flush();
            }
            while delayed
                .front()
                .is_some_and(|(due, _)| Instant::now() >= *due)
            {
                publish(&mut node, "telemetry", &delayed.pop_front().unwrap().1)?;
            }
            if name == "control" {
                drain(&mut node, &batches, &handle)?;
            }
            if heartbeat.elapsed() >= Duration::from_secs(1) {
                publish(&mut node, "telemetry", &status())?;
                heartbeat = Instant::now();
            }
            thread::sleep(Duration::from_millis(1));
        }
        if transport_open {
            for (_, message) in delayed {
                publish(&mut node, "telemetry", &message)?;
            }
        }
        drop(registration);
        if name == "control" {
            shared.control_idle.store(true, Ordering::Release);
            while !shared.drained.load(Ordering::Acquire) {
                if transport_open {
                    drain(&mut node, &batches, &handle)?;
                } else {
                    while batches.try_recv().is_ok() {}
                }
                thread::sleep(Duration::from_millis(2));
            }
            if transport_open {
                drain(&mut node, &batches, &handle)?;
                publish(&mut node, "telemetry", &status())?;
            }
        }
        Ok(())
    })
}
fn drain(node: &mut DoraNode, batches: &Receiver<Vec<u8>>, handle: &Handle) -> Result<()> {
    for bytes in batches.try_iter() {
        let batch = activity_tracing::wire::proto::Batch::decode(bytes.as_slice())?;
        let clock = handle.clock();
        publish(
            node,
            "telemetry",
            &envelope(pb::envelope::Body::Clock(pb::Clock {
                session: clock.session,
                unix_origin_ns: clock.unix_origin_ns,
                domain: format!("activity/{:016x}", clock.session),
            })),
        )?;
        publish(
            node,
            "telemetry",
            &envelope(pb::envelope::Body::Activity(batch)),
        )?;
    }
    Ok(())
}
