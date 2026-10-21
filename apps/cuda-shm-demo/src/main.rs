use cuda_dora_shm::{Consumer, Message, PoolConfig, Producer, send};
use cuda_host_memory::{AllocationMode, CudaContext, CudaStream};
use cudarc::{
    driver::{CudaFunction, LaunchConfig, PushKernelArg},
    nvrtc::compile_ptx,
};
use dora_node_api::{DoraNode, Event, EventStream, TryRecvError};
use eyre::{Result, bail, ensure};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

const SOURCE: &str = r#"
extern "C" __global__ void fill(unsigned char* p, unsigned int n, unsigned int seq, unsigned long long delay) {
    unsigned long long start = clock64();
    while (clock64() - start < delay) {}
    unsigned int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) p[i] = (unsigned char)((seq % 251) + (i % 13));
}
extern "C" __global__ void verify(const unsigned char* p, unsigned int n, unsigned int seq, unsigned long long delay) {
    unsigned long long start = clock64();
    while (clock64() - start < delay) {}
    unsigned int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n && p[i] != (unsigned char)((seq % 251) + (i % 13))) asm("trap;");
}
"#;
fn option(args: &[String], name: &str, default: usize) -> Result<usize> {
    match args.iter().position(|a| a == name) {
        Some(i) => Ok(args
            .get(i + 1)
            .ok_or_else(|| eyre::eyre!("missing {name}"))?
            .parse()?),
        None => Ok(default),
    }
}
fn event_message(events: &mut EventStream) -> Result<Option<(String, Message)>> {
    match events.try_recv() {
        Ok(Event::Input { id, data, .. }) => Ok(Some((
            id.to_string(),
            Message::decode(<&[u8]>::try_from(&data)?)?,
        ))),
        Ok(Event::Error(error)) => bail!("Dora: {error}"),
        Ok(Event::Stop(_)) | Err(TryRecvError::Closed) => {
            bail!("Dora stopped before protocol completed")
        }
        Ok(_) | Err(TryRecvError::Empty) => Ok(None),
    }
}
fn producer(
    args: &[String],
    ctx: Arc<CudaContext>,
    mut node: DoraNode,
    mut events: EventStream,
    kernel: CudaFunction,
) -> Result<()> {
    let peers = option(args, "--consumers", 2)?;
    let frames = option(args, "--frames", 12)?;
    let slots = option(args, "--slots", 2)?;
    let size = option(args, "--bytes", 4096)?;
    let timeout = Duration::from_secs(option(args, "--timeout", 30)? as u64);
    ensure!(
        frames > 0 && frames <= u32::MAX as usize,
        "invalid frame count"
    );
    // SAFETY: only this demo's trusted adapter peers access the private files;
    // they never truncate them or write outside the producer reservation.
    let mut pool = unsafe {
        Producer::new(
            ctx.clone(),
            PoolConfig {
                slots,
                capacity: size,
                slot_alignment: option(
                    args,
                    "--slot-alignment",
                    cuda_dora_shm::DEFAULT_SLOT_ALIGNMENT,
                )?,
                peers: (0..peers).map(|i| format!("consumer{i}")).collect(),
                allocation: AllocationMode::Registered,
            },
        )?
    };
    let streams: Vec<_> = (0..slots)
        .map(|_| ctx.new_stream())
        .collect::<Result<_, _>>()?;
    let manifest = Message::Manifest(pool.manifest().clone());
    let deadline = Instant::now() + timeout;
    let mut last_hello = Instant::now() - Duration::from_secs(1);
    let mut sequence = 0;
    let mut full = 0u64;
    let mut shutdown = false;
    loop {
        if Instant::now() >= deadline {
            bail!(
                "TIMEOUT: no unsafe reuse; published={sequence}, full={full}, stats={:?}",
                pool.stats()
            );
        }
        if let Some((input, message)) = event_message(&mut events)? {
            let peer = match &message {
                Message::Attached { peer, .. }
                | Message::Returned { peer, .. }
                | Message::Closed { peer, .. } => peer,
                _ => bail!("unexpected peer message"),
            };
            ensure!(&input == peer, "peer identity does not match Dora route");
            // SAFETY: configured local demo peers use only the adapter's generated
            // messages and retained events. Input route binds claimed identity.
            unsafe {
                pool.handle(&message)?;
            }
        }
        if !pool.attached() && !shutdown && last_hello.elapsed() >= Duration::from_millis(100) {
            send(&mut node, "frames", &manifest)?;
            last_hello = Instant::now();
        }
        if pool.attached() && !shutdown {
            pool.reclaim_completed()?;
            if sequence < frames {
                if let Some(mut writer) = pool.try_acquire(streams[sequence % slots].clone())? {
                    let n = size as u32;
                    let seq = sequence as u32;
                    let delay = 20_000_000u64;
                    // SAFETY: fill is bounded, writes only this lease's payload,
                    // and is enqueued on its provided non-default stream.
                    unsafe {
                        writer.enqueue(|ptr, stream| {
                            stream
                                .launch_builder(&kernel)
                                .arg(&ptr)
                                .arg(&n)
                                .arg(&seq)
                                .arg(&delay)
                                .launch(LaunchConfig::for_num_elems(n))?;
                            Ok(())
                        })?;
                    }
                    let descriptor = writer.publish(size, sequence as u64)?;
                    send(&mut node, "frames", &descriptor)?;
                    sequence += 1;
                } else {
                    full += 1;
                }
            } else if pool.drained() {
                let stop = pool.shutdown()?;
                send(&mut node, "frames", &stop)?;
                shutdown = true;
            }
        }
        if pool.all_closed() {
            println!(
                "PRODUCER_SUMMARY {}",
                serde_json::json!({"frames":frames,"slots":slots,"consumers":peers,"full":full,"stats":pool.stats()})
            );
            pool.close()?;
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn consumer(
    args: &[String],
    ctx: Arc<CudaContext>,
    mut node: DoraNode,
    mut events: EventStream,
    kernel: CudaFunction,
) -> Result<()> {
    let index = option(args, "--id", 0)?;
    let peer = format!("consumer{index}");
    let delay = option(args, "--delay-cycles", 40_000_000)? as u64;
    let stall = args.iter().any(|a| a == "--stall");
    let duplicate = args.iter().any(|a| a == "--duplicate");
    let mut consumer: Option<Consumer> = None;
    let mut streams: Vec<Arc<CudaStream>> = Vec::new();
    let mut seen = 0u64;
    let mut max_generation = 0;
    let mut expected_manifest = None;
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        ensure!(Instant::now() < deadline, "consumer timeout");
        if let Some((_, message)) = event_message(&mut events)? {
            match &message {
                Message::Manifest(manifest) => {
                    if let Some(old) = &expected_manifest {
                        ensure!(old == manifest, "manifest changed");
                    }
                    if consumer.is_none() {
                        streams = (0..manifest.slots)
                            .map(|_| ctx.new_stream())
                            .collect::<Result<_, _>>()?;
                        // SAFETY: this demo uses its trusted local producer and
                        // consumes/returns generations only through the adapter.
                        consumer = Some(unsafe {
                            Consumer::connect(ctx.clone(), manifest.clone(), peer.clone())?
                        });
                        expected_manifest = Some(manifest.clone());
                    }
                    send(
                        &mut node,
                        "returns",
                        &consumer.as_ref().unwrap().attachment()?,
                    )?;
                }
                Message::Frame {
                    slot,
                    sequence,
                    generation,
                    ..
                } => {
                    ensure!(*sequence == seen, "missing/duplicate frame sequence");
                    let reader = consumer
                        .as_mut()
                        .ok_or_else(|| eyre::eyre!("frame before manifest"))?;
                    let stream = streams
                        .get(*slot)
                        .ok_or_else(|| eyre::eyre!("invalid slot"))?
                        .clone();
                    // SAFETY: the sole frames input comes from our trusted producer,
                    // which records Ready and reserves this peer before sending.
                    let mut frame = unsafe { reader.begin(&message, stream)? };
                    let n = frame.len() as u32;
                    let seq = frame.sequence() as u32;
                    // SAFETY: bounded read-only verification, Ready wait is
                    // already in this consumer-chosen non-default stream.
                    unsafe {
                        frame.enqueue(|ptr, stream| {
                            stream
                                .launch_builder(&kernel)
                                .arg(&ptr)
                                .arg(&n)
                                .arg(&seq)
                                .arg(&delay)
                                .launch(LaunchConfig::for_num_elems(n))?;
                            Ok(())
                        })?;
                    }
                    let returned = frame.finish()?; // Done record queued; no CPU wait.
                    if !stall {
                        send(&mut node, "returns", &returned)?;
                        if duplicate && seen == 0 {
                            send(&mut node, "returns", &returned)?;
                        }
                    }
                    max_generation = max_generation.max(*generation);
                    seen += 1;
                }
                Message::Shutdown { .. } => {
                    let reader = consumer
                        .take()
                        .ok_or_else(|| eyre::eyre!("shutdown before attach"))?;
                    // SAFETY: our producer sends Shutdown only after draining and
                    // destroying its imported Done events.
                    let closed = unsafe { reader.shutdown(&message)? };
                    send(&mut node, "returns", &closed)?;
                    println!(
                        "CONSUMER_SUMMARY {}",
                        serde_json::json!({"peer":peer,"frames":seen,"max_generation":max_generation,"gpu_verified":true})
                    );
                    return Ok(());
                }
                _ => bail!("unexpected consumer message"),
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let ctx = CudaContext::new(0)?;
    let module = ctx.load_module(compile_ptx(SOURCE)?)?;
    let role = args.get(1).map(String::as_str).unwrap_or("");
    let kernel = module.load_function(if role == "producer" { "fill" } else { "verify" })?;
    let (node, events) = DoraNode::init_from_env_force()?;
    match role {
        "producer" => producer(&args, ctx, node, events, kernel),
        "consumer" => consumer(&args, ctx, node, events, kernel),
        _ => bail!("expected producer or consumer role"),
    }
}
