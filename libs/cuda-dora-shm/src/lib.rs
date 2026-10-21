//! Same-host SPMC fixed-slot adapter over Dora control messages.
//!
//! Payloads live in bounded `/dev/shm` files. Every process mmaps and registers
//! its own view with CUDA. Ready/Done event handles are exchanged once; frames
//! carry only pool/slot/generation/length. No CUDA IPC memory handles are used.
//! Dora/Zenoh owns control-message storage; this adapter owns payload lifetimes.
//! Consumers and transport peers are trusted and must preserve the protocol.
mod event;
mod ledger;
mod protocol;

use cuda_host_memory::{
    Access, AllocationMode, CudaContext, CudaStream, GpuLease, HostMappedMemory,
};
use cudarc::driver::DriverError;
use event::Event;
use eyre::{Result, bail, ensure};
use ledger::Ledger;
use memmap2::MmapOptions;
pub use protocol::{MAX_MESSAGE, Manifest, Message, VERSION};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
    sync::Arc,
};

/// Immutable membership and bounded storage. Membership changes require a new pool.
pub struct PoolConfig {
    pub slots: usize,
    pub capacity: usize,
    pub peers: Vec<String>,
    pub allocation: AllocationMode,
}
impl PoolConfig {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.allocation == AllocationMode::Registered,
            "managed memory is local-only; IPC requires mmap + cudaHostRegister"
        );
        ensure!((1..=64).contains(&self.slots), "slots must be 1..=64");
        ensure!(
            (1..=64 * 1024 * 1024).contains(&self.capacity),
            "capacity must be 1..=64 MiB"
        );
        ensure!(
            !self.peers.is_empty() && self.peers.len() <= 64,
            "peers must be 1..=64"
        );
        let unique: BTreeSet<_> = self.peers.iter().collect();
        ensure!(
            unique.len() == self.peers.len()
                && self.peers.iter().all(|p| !p.is_empty() && p.len() <= 128),
            "invalid peers"
        );
        Ok(())
    }
}
fn boot_id() -> Result<String> {
    Ok(std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?
        .trim()
        .into())
}
fn gpu_id(ctx: &CudaContext) -> Result<Vec<u8>> {
    ctx.bind_to_thread()?;
    let mut uuid = cudarc::driver::sys::CUuuid { bytes: [0; 16] };
    // SAFETY: initialized out-pointer and device belonging to this context.
    unsafe {
        cudarc::driver::sys::cuDeviceGetUuid_v2(&mut uuid, ctx.cu_device()).result()?;
    }
    Ok(uuid.bytes.iter().flat_map(|x| x.to_ne_bytes()).collect())
}
fn map_slot(
    ctx: Arc<CudaContext>,
    directory: &Path,
    index: usize,
    capacity: usize,
    create: bool,
) -> Result<HostMappedMemory> {
    let path = directory.join(format!("slot-{index}"));
    let file = if create {
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)?;
        // Materialize tmpfs pages before mmap: ENOSPC is a normal initialization
        // error rather than a later SIGBUS. No payload allocation after warmup.
        let zeros = [0u8; 65536];
        let mut left = capacity;
        while left != 0 {
            let n = left.min(zeros.len());
            f.write_all(&zeros[..n])?;
            left -= n;
        }
        f
    } else {
        OpenOptions::new().read(true).write(true).open(path)?
    };
    ensure!(
        file.metadata()?.len() == capacity as u64,
        "slot size mismatch"
    );
    // SAFETY: trusted pool owner never truncates files; protocol excludes writes
    // while remote readers hold a generation. File inode stays alive via mmap.
    let mapping = unsafe { MmapOptions::new().len(capacity).map_mut(&file)? };
    // SAFETY: all external accesses are governed by the pool's Ready/Done protocol.
    Ok(unsafe { HostMappedMemory::from_mapping(ctx, mapping)? })
}

struct ProducerSlot {
    memory: HostMappedMemory,
    ready: Event,
    writer: Option<GpuLease>,
}
struct Peer {
    handles: Vec<Vec<u8>>,
    done: Vec<Event>,
}
/// Diagnostic counters; payload storage remains fixed after initialization.
#[derive(Default, Debug, serde::Serialize)]
pub struct Stats {
    pub published: u64,
    pub published_pending: u64,
    pub reclaimed: u64,
    pub completion_blocks: u64,
}

/// Single producer. Mutable operations serialize membership, reservation and reuse.
///
/// Full pools return `None`; no growth, heap fallback, overwrite or timeout-based
/// reclamation occurs. Lost messages/consumers hold slots until the flow is aborted.
/// `reclaim_completed` queries cached events; it never waits for incomplete events.
/// Underlying local lease destruction may wait for other work on the same stream.
/// Use per-slot streams when strict nonblocking reclamation is required.
pub struct Producer {
    stats: Stats,
    ctx: Arc<CudaContext>,
    manifest: Manifest,
    slots: Vec<ProducerSlot>,
    peers: BTreeMap<String, Peer>,
    ledger: Ledger,
    closing: bool,
    closed: BTreeSet<String>,
    _directory: tempfile::TempDir,
}
impl Producer {
    /// Preallocate, populate and register every slot in private tmpfs storage.
    /// # Safety
    /// Backing files must remain under this protocol's exclusive control until all
    /// mappings close. Never truncate or modify them through other file handles or
    /// mappings. Every admitted peer must obey the read-only consumer protocol.
    /// File paths are exposed for IPC; Rust cannot enforce these external rules.
    pub unsafe fn new(ctx: Arc<CudaContext>, config: PoolConfig) -> Result<Self> {
        config.validate()?;
        let directory = tempfile::Builder::new()
            .prefix("cuda-dora-")
            .tempdir_in("/dev/shm")?;
        let mut slots = Vec::new();
        for index in 0..config.slots {
            slots.push(ProducerSlot {
                memory: map_slot(ctx.clone(), directory.path(), index, config.capacity, true)?,
                ready: Event::new(ctx.clone())?,
                writer: None,
            });
        }
        let manifest = Manifest {
            version: VERSION,
            pool: directory
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            boot: boot_id()?,
            gpu: gpu_id(&ctx)?,
            directory: directory.path().to_string_lossy().into_owned(),
            slots: config.slots,
            capacity: config.capacity,
            peers: config.peers.clone(),
            ready: slots
                .iter()
                .map(|s| s.ready.export())
                .collect::<Result<_>>()?,
        };
        manifest.validate()?;
        Ok(Self {
            stats: Stats::default(),
            ctx,
            manifest,
            slots,
            peers: BTreeMap::new(),
            ledger: Ledger::new(config.slots, &config.peers),
            closing: false,
            closed: BTreeSet::new(),
            _directory: directory,
        })
    }
    pub fn stats(&self) -> &Stats {
        &self.stats
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn attached(&self) -> bool {
        self.peers.len() == self.manifest.peers.len()
    }
    pub fn drained(&self) -> bool {
        self.ledger.drained()
    }
    /// Validate and adopt an attachment, generation return or orderly close.
    /// The transport must authenticate `peer` against its routed input identity.
    /// # Safety
    /// Messages must come from the named trusted peer. Event handles must remain
    /// alive through shutdown; Returned must cover every CPU use and queued GPU use.
    pub unsafe fn handle(&mut self, message: &Message) -> Result<()> {
        match message {
            Message::Attached { pool, peer, done } => {
                ensure!(
                    pool == &self.manifest.pool && self.manifest.peers.contains(peer),
                    "foreign pool/peer"
                );
                ensure!(!self.closing, "pool closing");
                ensure!(
                    done.len() == self.slots.len() && done.iter().all(|h| h.len() == 64),
                    "invalid Done handles"
                );
                if let Some(old) = self.peers.get(peer) {
                    ensure!(&old.handles == done, "peer changed its event handles");
                    return Ok(());
                }
                let events = done
                    .iter()
                    .map(|h| Event::open(self.ctx.clone(), h))
                    .collect::<Result<_>>()?;
                self.peers.insert(
                    peer.clone(),
                    Peer {
                        handles: done.clone(),
                        done: events,
                    },
                );
            }
            Message::Returned {
                pool,
                peer,
                slot,
                generation,
            } => {
                ensure!(
                    !self.closing && pool == &self.manifest.pool,
                    "foreign/closing pool"
                );
                self.ledger.returned(*slot, *generation, peer)?;
            }
            Message::Closed { pool, peer } => {
                ensure!(
                    self.closing
                        && pool == &self.manifest.pool
                        && self.manifest.peers.contains(peer),
                    "unexpected close"
                );
                ensure!(self.closed.insert(peer.clone()), "duplicate close");
            }
            _ => bail!("unexpected producer message"),
        }
        Ok(())
    }
    /// Retire generations only after every return and both directions of GPU use.
    pub fn reclaim_completed(&mut self) -> Result<usize> {
        let mut count = 0;
        for index in self.ledger.candidates() {
            if !self.slots[index].ready.complete()? {
                continue;
            }
            let mut completed = true;
            for peer in self.peers.values() {
                if !peer.done[index].complete()? {
                    completed = false;
                    break;
                }
            }
            if !completed {
                self.stats.completion_blocks += 1;
            }
            if completed {
                if let Some(writer) = self.slots[index].writer.take() {
                    writer.wait()?;
                }
                self.ledger.reclaim(index);
                count += 1;
                self.stats.reclaimed += 1;
            }
        }
        Ok(count)
    }
    /// Reserve a free slot. Readiness is recorded during `WriteFrame::publish`.
    pub fn try_acquire(&mut self, stream: Arc<CudaStream>) -> Result<Option<WriteFrame<'_>>> {
        ensure!(
            !self.closing && self.attached(),
            "pool not ready for publication"
        );
        ensure!(
            stream.context().as_ref() == self.ctx.as_ref(),
            "foreign stream context"
        );
        let Some((slot, generation)) = self.ledger.reserve()? else {
            return Ok(None);
        };
        Ok(Some(WriteFrame {
            pool: self,
            slot,
            generation,
            stream,
            gpu: None,
            committed: false,
        }))
    }
    /// Start two-phase shutdown after draining; closes imported Done handles first.
    pub fn shutdown(&mut self) -> Result<Message> {
        ensure!(
            self.drained() && self.attached() && !self.closing,
            "pool not drained/attached"
        );
        self.peers.clear();
        self.closing = true;
        Ok(Message::Shutdown {
            pool: self.manifest.pool.clone(),
        })
    }
    pub fn all_closed(&self) -> bool {
        self.closing && self.closed.len() == self.manifest.peers.len()
    }
    /// Destroy exported Ready events only after all consumers closed their imports.
    pub fn close(mut self) -> Result<()> {
        ensure!(self.all_closed(), "consumers still hold event imports");
        for slot in &mut self.slots {
            slot.ready.allow_close();
        }
        Ok(())
    }
}

/// Exclusive local write reservation. Dropping an unpublished frame cancels safely.
pub struct WriteFrame<'a> {
    pool: &'a mut Producer,
    slot: usize,
    generation: u64,
    stream: Arc<CudaStream>,
    gpu: Option<GpuLease>,
    committed: bool,
}
impl WriteFrame<'_> {
    pub fn slot(&self) -> usize {
        self.slot
    }
    /// CPU access is available before CUDA submission; reference cannot escape.
    pub fn with_cpu_write<R>(&mut self, write: impl FnOnce(&mut [u8]) -> R) -> Result<R> {
        ensure!(self.gpu.is_none(), "GPU write already queued");
        Ok(write(&mut self.pool.slots[self.slot].memory.write()?))
    }
    /// Enqueue producer work on this frame's stream.
    /// # Safety
    /// Same kernel bounds/access/stream obligations as `GpuLease::enqueue`.
    pub unsafe fn enqueue(
        &mut self,
        submit: impl FnOnce(u64, &CudaStream) -> Result<(), DriverError>,
    ) -> Result<()> {
        if self.gpu.is_none() {
            self.gpu = Some(
                self.pool.slots[self.slot]
                    .memory
                    .gpu(self.stream.clone(), Access::Write)?,
            );
        }
        // SAFETY: caller upholds the underlying lease's kernel submission contract.
        unsafe {
            self.gpu.as_mut().unwrap().enqueue(submit)?;
        }
        Ok(())
    }
    /// Record Ready, reserve every consumer, and return metadata immediately.
    /// No CPU wait for CUDA completion. Failure to send metadata keeps the slot busy.
    pub fn publish(mut self, len: usize, sequence: u64) -> Result<Message> {
        ensure!(
            len > 0 && len <= self.pool.manifest.capacity,
            "frame exceeds slot capacity"
        );
        self.pool.slots[self.slot].ready.record(&self.stream)?;
        self.pool.stats.published += 1;
        if !self.pool.slots[self.slot].ready.complete()? {
            self.pool.stats.published_pending += 1;
        }
        self.pool.ledger.publish(self.slot);
        self.pool.slots[self.slot].writer = self.gpu.take();
        self.committed = true;
        Ok(Message::Frame {
            pool: self.pool.manifest.pool.clone(),
            slot: self.slot,
            generation: self.generation,
            len,
            sequence,
        })
    }
}
impl Drop for WriteFrame<'_> {
    fn drop(&mut self) {
        if !self.committed {
            // On completion failure leave the slot permanently reserved.
            if let Some(gpu) = self.gpu.take()
                && gpu.wait().is_err()
            {
                return;
            }
            self.pool.ledger.cancel(self.slot);
        }
    }
}

struct ConsumerSlot {
    memory: HostMappedMemory,
    ready: Event,
    done: Event,
    previous: Option<GpuLease>,
    generation: u64,
    abandoned: bool,
}
/// Process-local mappings and cached fences for one named consumer.
pub struct Consumer {
    manifest: Manifest,
    peer: String,
    slots: Vec<ConsumerSlot>,
    ctx: Arc<CudaContext>,
}
impl Consumer {
    /// Map/register the producer's tmpfs slots and import Ready event handles.
    ///
    /// # Safety
    /// The manifest and all subsequent control messages must originate from a
    /// trusted matching producer. It must preserve backing files and Ready events,
    /// write only free generations, and obey the shutdown handshake. External
    /// truncation or protocol violations can invalidate CPU/GPU memory safety.
    pub unsafe fn connect(ctx: Arc<CudaContext>, manifest: Manifest, peer: String) -> Result<Self> {
        manifest.validate()?;
        ensure!(
            manifest.boot == boot_id()?,
            "remote host/boot rejected; device payload has no network fallback"
        );
        ensure!(manifest.gpu == gpu_id(&ctx)?, "GPU UUID mismatch");
        ensure!(
            manifest.peers.contains(&peer),
            "consumer not in pool membership"
        );
        let directory = std::fs::canonicalize(&manifest.directory)?;
        ensure!(
            directory.parent() == Some(Path::new("/dev/shm")),
            "pool is not in /dev/shm"
        );
        ensure!(
            directory.file_name().unwrap().to_string_lossy() == manifest.pool,
            "pool identity mismatch"
        );
        let mut slots = Vec::new();
        for index in 0..manifest.slots {
            slots.push(ConsumerSlot {
                memory: map_slot(ctx.clone(), &directory, index, manifest.capacity, false)?,
                ready: Event::open(ctx.clone(), &manifest.ready[index])?,
                done: Event::new(ctx.clone())?,
                previous: None,
                generation: 0,
                abandoned: false,
            });
        }
        Ok(Self {
            manifest,
            peer,
            slots,
            ctx,
        })
    }
    pub fn attachment(&self) -> Result<Message> {
        Ok(Message::Attached {
            pool: self.manifest.pool.clone(),
            peer: self.peer.clone(),
            done: self
                .slots
                .iter()
                .map(|s| s.done.export())
                .collect::<Result<_>>()?,
        })
    }
    /// Acquire this generation and insert Ready wait on the caller's chosen stream.
    /// # Safety
    /// `frame` must be an unmodified descriptor from the connected trusted producer,
    /// sent after recording Ready and reserving this reader. Generation validation
    /// cannot establish that a CUDA event was recorded for a forged descriptor.
    pub unsafe fn begin(
        &mut self,
        frame: &Message,
        stream: Arc<CudaStream>,
    ) -> Result<ReadFrame<'_>> {
        let Message::Frame {
            pool,
            slot,
            generation,
            len,
            sequence,
        } = frame
        else {
            bail!("expected frame");
        };
        ensure!(
            pool == &self.manifest.pool && *len > 0 && *len <= self.manifest.capacity,
            "invalid frame pool/length"
        );
        ensure!(
            stream.context().as_ref() == self.ctx.as_ref(),
            "foreign stream context"
        );
        let local = self
            .slots
            .get_mut(*slot)
            .ok_or_else(|| eyre::eyre!("invalid slot"))?;
        ensure!(
            !local.abandoned && local.generation.checked_add(1) == Some(*generation),
            "stale/skipped generation"
        );
        if let Some(previous) = local.previous.take() {
            previous.wait()?;
        }
        local.ready.wait(&stream)?;
        let gpu = local.memory.gpu(stream.clone(), Access::Read)?;
        local.generation = *generation;
        local.abandoned = true;
        Ok(ReadFrame {
            consumer: self,
            slot: *slot,
            generation: *generation,
            len: *len,
            sequence: *sequence,
            stream,
            gpu: Some(gpu),
        })
    }
    /// Close Ready imports and Done exporters after producer closed Done imports.
    /// # Safety
    /// `message` must originate from the connected producer after it closed all
    /// Done imports and stopped publishing. Pool identity alone is insufficient.
    pub unsafe fn shutdown(mut self, message: &Message) -> Result<Message> {
        ensure!(
            matches!(message, Message::Shutdown { pool } if pool == &self.manifest.pool),
            "unexpected shutdown"
        );
        for slot in &mut self.slots {
            ensure!(!slot.abandoned, "unreturned frame");
            if let Some(gpu) = slot.previous.take() {
                gpu.wait()?;
            }
            slot.done.allow_close();
        }
        self.slots.clear(); // close imports before sending Closed.
        Ok(Message::Closed {
            pool: self.manifest.pool.clone(),
            peer: self.peer.clone(),
        })
    }
}
/// Read-only consumer use. `finish` transfers completion responsibility to producer.
pub struct ReadFrame<'a> {
    consumer: &'a mut Consumer,
    slot: usize,
    generation: u64,
    len: usize,
    sequence: u64,
    stream: Arc<CudaStream>,
    gpu: Option<GpuLease>,
}
impl ReadFrame<'_> {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        false
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Wait Ready on CPU, then run a scoped read. No references escape.
    pub fn with_cpu_read<R>(&self, read: impl FnOnce(&[u8]) -> R) -> Result<R> {
        let slot = &self.consumer.slots[self.slot];
        slot.ready.synchronize()?;
        Ok(read(&slot.memory.read()?[..self.len]))
    }
    /// Submit read-only kernels on the selected stream after its Ready dependency.
    /// # Safety
    /// Obey `GpuLease::enqueue`; accesses must be read-only and within `len()`.
    pub unsafe fn enqueue(
        &mut self,
        submit: impl FnOnce(u64, &CudaStream) -> Result<(), DriverError>,
    ) -> Result<()> {
        // SAFETY: caller upholds bounded, read-only, stream-confined access.
        unsafe {
            self.gpu.as_mut().unwrap().enqueue(submit)?;
        }
        Ok(())
    }
    /// Record Done and return metadata without CPU waiting for GPU completion.
    /// The caller must send this return exactly once; no more uses are permitted.
    pub fn finish(mut self) -> Result<Message> {
        let slot = &mut self.consumer.slots[self.slot];
        slot.done.record(&self.stream)?;
        slot.previous = self.gpu.take();
        slot.abandoned = false;
        Ok(Message::Returned {
            pool: self.consumer.manifest.pool.clone(),
            peer: self.consumer.peer.clone(),
            slot: self.slot,
            generation: self.generation,
        })
    }
}

/// Publish only the bounded descriptor/control message through Dora's native IPC.
pub fn send(node: &mut dora_node_api::DoraNode, output: &str, message: &Message) -> Result<()> {
    let bytes = message.encode()?;
    node.send_output_raw(output.into(), Default::default(), bytes.len(), |out| {
        out.copy_from_slice(&bytes)
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn managed_and_invalid_pool_config_are_rejected_without_cuda() {
        // Arrange
        let mut config = PoolConfig {
            slots: 2,
            capacity: 4096,
            peers: vec!["a".into()],
            allocation: AllocationMode::Managed,
        };
        // Act / Assert
        assert!(config.validate().is_err());
        config.allocation = AllocationMode::Registered;
        assert!(config.validate().is_ok());
        config.peers.push("a".into());
        assert!(config.validate().is_err());
    }
    #[test]
    fn malformed_wire_messages_are_rejected() {
        // Arrange
        let frame = Message::Frame {
            pool: "p".into(),
            slot: 0,
            generation: 42,
            len: 10,
            sequence: 1,
        };
        // Act
        let decoded = Message::decode(&frame.encode().unwrap()).unwrap();
        // Assert
        assert_eq!(frame, decoded);
        assert!(Message::decode(b"{\"type\":\"Unknown\"}").is_err());
        assert!(Message::decode(&vec![0; MAX_MESSAGE + 1]).is_err());
    }
}
