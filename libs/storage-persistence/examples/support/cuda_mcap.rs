//! Hardware qualification shared by the executable example and ignored integration test.
use cudarc::{
    driver::{
        CudaContext, CudaEvent, CudaFunction, CudaSlice, CudaStream, DevicePtrMut, LaunchConfig,
        PushKernelArg, sys::CUdevice_attribute,
    },
    nvrtc::compile_ptx,
    runtime::sys,
};
use std::{error::Error, ffi::c_void, fs::File, io, path::Path, sync::Arc, time::Instant};
use storage_persistence::{
    mcap::{ALIGNMENT, Channel, McapConfig, McapWriter, Message, PreparedBlock, Schema},
    slabs::SlabPool,
    writer::WriterConfig,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const RECORD_HEADER_BYTES: usize = 31;
const RECORD_PADDING_HEADER_BYTES: usize = 9;
const WRITER_BUFFERS: u32 = 2;
const KERNEL: &str = r#"
extern "C" __global__ void fill(unsigned char* output, unsigned int n, unsigned int sequence) {
    unsigned int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) output[i] = (unsigned char)((i * 13u) ^ (i >> 8) ^ (sequence * 17u));
}
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Mapped,
    Staged,
    IoOnly,
}
impl Mode {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "mapped" => Ok(Self::Mapped),
            "staged" => Ok(Self::Staged),
            "io-only" => Ok(Self::IoOnly),
            _ => Err("mode must be mapped, staged, or io-only".into()),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Mapped => "mapped",
            Self::Staged => "staged",
            Self::IoOnly => "io-only",
        }
    }
}

struct Mapping {
    host: usize,
    device: u64,
}
/// The pool owner outlives every CUDA registration, independently of slab reuse.
struct RegisteredPool {
    pool: Option<SlabPool>,
    context: Arc<CudaContext>,
    mappings: Vec<Mapping>,
}
fn cuda(status: sys::cudaError_t, operation: &str) -> Result<()> {
    if status == sys::cudaError::cudaSuccess {
        Ok(())
    } else {
        Err(format!("{operation}: {status:?}").into())
    }
}
impl RegisteredPool {
    fn new(context: Arc<CudaContext>, count: usize, size: usize) -> Result<Self> {
        context.bind_to_thread()?;
        let mut out = Self {
            pool: Some(SlabPool::new(count, size, ALIGNMENT)?),
            context,
            mappings: Vec::new(),
        };
        let mut held = Vec::with_capacity(count);
        for _ in 0..count {
            let mut slab = out.pool().acquire()?;
            let host = slab.bytes_mut().as_mut_ptr().cast::<c_void>();
            // SAFETY: the retained pool owns this stable, page-aligned allocation.
            // No GPU work exists yet; holding all slabs visits each allocation once.
            cuda(
                unsafe { sys::cudaHostRegister(host, size, sys::cudaHostRegisterMapped) },
                "cudaHostRegister",
            )?;
            out.mappings.push(Mapping {
                host: host as usize,
                device: 0,
            });
            let mut device = std::ptr::null_mut();
            // SAFETY: registration succeeded and its pool owner remains retained.
            cuda(
                unsafe { sys::cudaHostGetDevicePointer(&mut device, host, 0) },
                "cudaHostGetDevicePointer",
            )?;
            out.mappings.last_mut().unwrap().device = device as u64;
            held.push(slab);
        }
        Ok(out)
    }
    fn pool(&self) -> &SlabPool {
        self.pool.as_ref().unwrap()
    }
    fn device_base(&self, host: usize) -> Result<u64> {
        self.mappings
            .iter()
            .find(|m| m.host == host)
            .map(|m| m.device)
            .ok_or_else(|| "slab was not registered".into())
    }
    fn unregister(&mut self) -> Result<()> {
        self.context.bind_to_thread()?;
        while let Some(mapping) = self.mappings.last() {
            // SAFETY: all CUDA work has completed; the pool still retains the allocation.
            cuda(
                unsafe { sys::cudaHostUnregister(mapping.host as *mut c_void) },
                "cudaHostUnregister",
            )?;
            self.mappings.pop();
        }
        Ok(())
    }
    fn close(mut self) -> Result<()> {
        if self.pool().available() != self.pool().capacity() {
            return Err("slab still retained after writer finish".into());
        }
        self.context.synchronize()?;
        self.unregister()
    }
}
impl Drop for RegisteredPool {
    fn drop(&mut self) {
        if self.mappings.is_empty() {
            return;
        }
        // An asynchronous CUDA failure cannot permit freeing registered storage.
        if self.context.synchronize().is_err() || self.unregister().is_err() {
            std::mem::forget(self.pool.take());
            std::mem::forget(self.context.clone());
        }
    }
}

struct Producer {
    stream: Arc<CudaStream>,
    kernel: CudaFunction,
    start: CudaEvent,
    ready: CudaEvent,
    device: Option<CudaSlice<u8>>,
}
impl Producer {
    fn new(context: &Arc<CudaContext>, payload: usize, mode: Mode) -> Result<Self> {
        let stream = context.new_stream()?;
        if stream.cu_stream().is_null() {
            return Err("default CUDA stream is forbidden".into());
        }
        let module = context.load_module(compile_ptx(KERNEL)?)?;
        let kernel = module.load_function("fill")?;
        let start =
            context.new_event(Some(cudarc::driver::sys::CUevent_flags::CU_EVENT_DEFAULT))?;
        // Event waits block the CPU thread instead of spinning on GPU completion.
        let ready = context.new_event(Some(
            cudarc::driver::sys::CUevent_flags::CU_EVENT_BLOCKING_SYNC,
        ))?;
        let device = if mode == Mode::Staged {
            Some(stream.alloc_zeros(payload)?)
        } else {
            None
        };
        Ok(Self {
            stream,
            kernel,
            start,
            ready,
            device,
        })
    }
    fn produce(
        &mut self,
        memory: &RegisteredPool,
        writer: &mut McapWriter,
        sequence: u32,
        payload: usize,
        mode: Mode,
    ) -> Result<(PreparedBlock, f64)> {
        let mut slab = match memory.pool().acquire() {
            Ok(slab) => slab,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                writer.flush_written()?;
                memory.pool().acquire()?
            }
            Err(error) => return Err(error.into()),
        };
        let host_base = slab.bytes_mut().as_mut_ptr() as usize;
        let mapped_base = memory.device_base(host_base)?;
        let mut builder = writer.prepare_records(slab);
        let bytes = builder.reserve(
            Message {
                channel: 1,
                sequence,
                log_time: u64::from(sequence),
                publish_time: u64::from(sequence),
            },
            payload,
        )?;
        let host = bytes.as_mut_ptr();
        let offset = host as usize - host_base;
        let n = u32::try_from(payload)?;
        let operation = (|| -> Result<f64> {
            self.start.record(&self.stream)?;
            let (pointer, guard) = if let Some(device) = &mut self.device {
                let (pointer, guard) = device.device_ptr_mut(&self.stream);
                (pointer, Some(guard))
            } else {
                (mapped_base + offset as u64, None)
            };
            // SAFETY: the exclusive writable slab is retained until GPU completion.
            // The kernel writes exactly n payload bytes on our non-default stream.
            unsafe {
                self.stream
                    .launch_builder(&self.kernel)
                    .arg(&pointer)
                    .arg(&n)
                    .arg(&sequence)
                    .launch(LaunchConfig::for_num_elems(n))?;
                if mode == Mode::Staged {
                    cuda(
                        sys::cudaMemcpyAsync(
                            host.cast(),
                            pointer as *const c_void,
                            payload,
                            sys::cudaMemcpyKind::cudaMemcpyDeviceToHost,
                            self.stream.cu_stream().cast(),
                        ),
                        "cudaMemcpyAsync",
                    )?;
                }
            }
            drop(guard);
            self.ready.record(&self.stream)?;
            self.ready.synchronize()?;
            Ok(f64::from(self.start.elapsed_ms(&self.ready)?))
        })();
        let gpu_ms = match operation {
            Ok(ms) => ms,
            Err(error) => {
                // Early launch/copy/event failure must not recycle a GPU-written slab.
                if self.stream.synchronize().is_err() {
                    std::mem::forget(builder);
                }
                return Err(error);
            }
        };
        let block = builder.seal().map_err(|error| error.error)?;
        // These are the allocation and payload addresses submitted to the slab path.
        if block.slab().as_slice().as_ptr() as usize != host_base {
            return Err("slab backing changed".into());
        }
        Ok((block, gpu_ms))
    }
}

fn append(writer: &mut McapWriter, block: PreparedBlock) -> Result<()> {
    match writer.append_prepared_records(block) {
        Ok(_) => Ok(()),
        Err(rejected) if rejected.error.kind() == io::ErrorKind::WouldBlock => {
            writer.flush_written()?;
            writer
                .append_prepared_records(rejected.value)
                .map(|_| ())
                .map_err(|error| error.error.into())
        }
        Err(error) => Err(error.error.into()),
    }
}

#[derive(Clone, Copy)]
struct Cpu {
    user: f64,
    system: f64,
}
impl Cpu {
    fn read() -> Result<Self> {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        // SAFETY: getrusage initializes the output on success.
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        let usage = unsafe { usage.assume_init() };
        let seconds = |t: libc::timeval| t.tv_sec as f64 + t.tv_usec as f64 / 1e6;
        Ok(Self {
            user: seconds(usage.ru_utime),
            system: seconds(usage.ru_stime),
        })
    }
}

pub struct Stats {
    pub gpu: String,
    pub integrated: bool,
    pub mode: Mode,
    pub payload: usize,
    pub messages: u32,
    pub slab_count: usize,
    pub setup_ms: f64,
    pub wall_seconds: f64,
    pub user_seconds: f64,
    pub system_seconds: f64,
    pub gpu_ms: f64,
    pub file_bytes: u64,
}
impl Stats {
    pub fn print(&self) {
        let payload_bytes = self.payload as f64 * f64::from(self.messages);
        println!(
            "gpu={:?} integrated={} mode={} payload_bytes={} messages={} slabs={} setup_ms={:.3} wall_s={:.6} payload_mib_s={:.3} file_mib_s={:.3} cpu_user_s={:.6} cpu_system_s={:.6} cpu_pct_one_core={:.3} gpu_ms={:.3} file_bytes={} verified=true",
            self.gpu,
            self.integrated,
            self.mode.name(),
            self.payload,
            self.messages,
            self.slab_count,
            self.setup_ms,
            self.wall_seconds,
            payload_bytes / self.wall_seconds / 1048576.0,
            self.file_bytes as f64 / self.wall_seconds / 1048576.0,
            self.user_seconds,
            self.system_seconds,
            (self.user_seconds + self.system_seconds) / self.wall_seconds * 100.0,
            self.gpu_ms,
            self.file_bytes
        );
    }
}

/// Timed production, slab submission, drain and final fsync; setup/readback are excluded.
pub fn run(
    path: &Path,
    mode: Mode,
    payload: usize,
    messages: u32,
    slab_count: usize,
) -> Result<Stats> {
    if payload == 0 || payload > u32::MAX as usize || messages == 0 || slab_count == 0 {
        return Err(
            "payload/messages/slabs must be nonzero; payload must fit a CUDA u32 launch".into(),
        );
    }
    let setup = Instant::now();
    let slab_bytes = payload
        .checked_add(RECORD_HEADER_BYTES + RECORD_PADDING_HEADER_BYTES)
        .and_then(|n| n.checked_next_multiple_of(ALIGNMENT))
        .ok_or("slab size overflow")?;
    let registrations = slab_count
        .checked_mul(slab_bytes / ALIGNMENT)
        .and_then(|n| n.checked_add(WRITER_BUFFERS as usize * 2))
        .and_then(|n| u32::try_from(n).ok())
        .ok_or("registration count overflow")?;
    let context = CudaContext::new(0)?;
    let gpu = context.name()?;
    let integrated = context.attribute(CUdevice_attribute::CU_DEVICE_ATTRIBUTE_INTEGRATED)? != 0;
    let memory = RegisteredPool::new(context.clone(), slab_count, slab_bytes)?;
    let mut writer = McapWriter::create(
        path,
        Schema {
            name: "gpu-pattern",
            encoding: "",
            data: &[],
        },
        &[Channel {
            id: 1,
            topic: "gpu",
            encoding: "",
        }],
        McapConfig {
            buffers: WRITER_BUFFERS,
            buffer_size: ALIGNMENT * 2,
            io: WriterConfig {
                registration_capacity: registrations,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    writer.prepare_slabs(memory.pool())?;
    let mut producer = Producer::new(&context, payload, mode)?;
    // Warm the actual kernel, mapped memory and slab framing without recording it.
    let (warm, _) = producer.produce(
        &memory,
        &mut writer,
        0,
        payload,
        if mode == Mode::Staged {
            Mode::Staged
        } else {
            Mode::Mapped
        },
    )?;
    let template = if mode == Mode::IoOnly {
        Some(warm)
    } else {
        drop(warm);
        None
    };
    writer.flush_written()?;
    let setup_ms = setup.elapsed().as_secs_f64() * 1000.0;
    let cpu = Cpu::read()?;
    let start = Instant::now();
    let mut gpu_ms = 0.0;
    for sequence in 0..messages {
        let block = if let Some(template) = &template {
            template.clone()
        } else {
            let (block, elapsed) =
                producer.produce(&memory, &mut writer, sequence, payload, mode)?;
            gpu_ms += elapsed;
            block
        };
        append(&mut writer, block)?;
    }
    writer.finish()?;
    let wall_seconds = start.elapsed().as_secs_f64();
    let after = Cpu::read()?;
    drop(template);
    memory.close()?;
    verify(path, mode, payload, messages)?;
    Ok(Stats {
        gpu,
        integrated,
        mode,
        payload,
        messages,
        slab_count,
        setup_ms,
        wall_seconds,
        user_seconds: after.user - cpu.user,
        system_seconds: after.system - cpu.system,
        gpu_ms,
        file_bytes: std::fs::metadata(path)?.len(),
    })
}

fn verify(path: &Path, mode: Mode, payload: usize, messages: u32) -> Result<()> {
    let file = File::open(path)?;
    // SAFETY: the finalized owned test output is immutable throughout readback.
    let bytes = unsafe { memmap2::Mmap::map(&file)? };
    let mut count = 0_u32;
    for message in mcap::MessageStream::new(&bytes)? {
        let message = message?;
        let sequence = if mode == Mode::IoOnly { 0 } else { count };
        if message.sequence != sequence
            || message.data.len() != payload
            || message.channel.topic != "gpu"
        {
            return Err(format!("MCAP metadata mismatch at message {count}").into());
        }
        for (i, &byte) in message.data.iter().enumerate() {
            let i = i as u32;
            let expected = (i.wrapping_mul(13) ^ (i >> 8) ^ sequence.wrapping_mul(17)) as u8;
            if byte != expected {
                return Err(format!(
                    "GPU payload mismatch at message {count}, byte {i}: {byte} != {expected}"
                )
                .into());
            }
        }
        count += 1;
    }
    if count != messages {
        return Err(format!("MCAP message count {count} != {messages}").into());
    }
    Ok(())
}
