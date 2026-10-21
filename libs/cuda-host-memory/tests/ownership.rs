//! CPU initialization → direct mapped-memory kernel accesses → CPU verification.
use cuda_host_memory::{Access, AllocationMode, CudaContext, HostMappedMemory};
use cudarc::{
    driver::{LaunchConfig, PushKernelArg},
    nvrtc::compile_ptx,
};
use std::error::Error;

const KERNEL: &str = r#"
extern "C" __global__ void update(unsigned char* bytes, unsigned int n) {
    unsigned int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) bytes[i] = (unsigned char)(bytes[i] + 1);
}
"#;

fn exercise(len: u32, mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    // Arrange
    let context = CudaContext::new(0)?;
    let stream = context.new_stream()?; // CU_STREAM_NON_BLOCKING, not stream 0.
    assert!(!stream.cu_stream().is_null());
    let module = context.load_module(compile_ptx(KERNEL)?)?;
    let kernel = module.load_function("update")?;
    let memory = allocate(context, len as usize, mode)?;
    // Assert: both allocation paths expose initialized, correctly-sized storage.
    assert_eq!(memory.len(), len as usize);
    assert!(!memory.is_empty());
    assert!(memory.read()?.iter().all(|&byte| byte == 0));
    for (i, byte) in memory.write()?.iter_mut().enumerate() {
        *byte = (i % 251) as u8;
    }
    // Act
    let mut gpu = memory.gpu(stream.clone(), Access::Write)?;
    assert!(matches!(memory.read(), Err(cuda_host_memory::Error::Busy)));
    // SAFETY: both kernels use only the supplied stream and pointer, stay in
    // bounds, and write under the declared exclusive GPU reservation.
    unsafe {
        gpu.enqueue(|ptr, stream| {
            for _ in 0..2 {
                stream
                    .launch_builder(&kernel)
                    .arg(&ptr)
                    .arg(&len)
                    .launch(LaunchConfig::for_num_elems(len))?;
            }
            Ok(())
        })?;
    }
    gpu.wait()?; // Releases access only after stream completion.
    // Assert
    for (i, &byte) in memory.read()?.iter().enumerate() {
        assert_eq!(byte, (i % 251) as u8 + 2, "byte {i}");
    }
    println!(
        "PASS: {len} bytes; checked CPU/GPU leases; two kernels on non-default stream {:?}",
        stream.cu_stream()
    );
    Ok(())
}

fn mapped_memory_on_non_default_stream(mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    // Arrange
    let lengths = [1, 4095, 4096, 4097, 1024 * 1024];
    for len in lengths {
        // Act
        let result = exercise(len, mode);

        // Assert
        assert!(result.is_ok(), "length {len}: {result:?}");
    }
    Ok(())
}

fn leases_retain_storage_and_exclude_conflicts(mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    // Arrange
    let context = CudaContext::new(0)?;
    let stream = context.new_stream()?;
    let memory = allocate(context.clone(), 4096, mode)?;
    // Act
    let reader = memory.read()?;
    let gpu_reader = memory.gpu(stream.clone(), Access::Read)?;
    // Assert
    assert!(matches!(memory.write(), Err(cuda_host_memory::Error::Busy)));
    assert!(matches!(
        memory.gpu(stream.clone(), Access::Write),
        Err(cuda_host_memory::Error::Busy)
    ));
    // Act
    drop(reader);
    assert!(matches!(memory.write(), Err(cuda_host_memory::Error::Busy)));
    // Act
    drop(gpu_reader);
    memory.write()?[0] = 7;
    let alias = memory.clone();
    let writer = memory.write()?;
    assert!(matches!(alias.read(), Err(cuda_host_memory::Error::Busy)));
    drop(writer);

    let module = context.load_module(compile_ptx(KERNEL)?)?;
    let kernel = module.load_function("update")?;
    let mut gpu = memory.gpu(stream, Access::Write)?;
    let len = 4096u32;
    // SAFETY: bounded write on the supplied stream under a write lease.
    unsafe {
        gpu.enqueue(|ptr, stream| {
            stream
                .launch_builder(&kernel)
                .arg(&ptr)
                .arg(&len)
                .launch(LaunchConfig::for_num_elems(len))?;
            Ok(())
        })?;
    }
    // Act
    drop(memory); // GPU lease + alias still own storage.
    drop(gpu); // Early drop waits and releases access.
    let cpu = alias.read()?;
    drop(alias); // CPU guard still owns storage.
    // Assert
    assert_eq!(cpu[0], 8);
    Ok(())
}

fn unwind_after_enqueue_retires_gpu_use(mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    // Arrange
    let context = CudaContext::new(0)?;
    let stream = context.new_stream()?;
    let memory = allocate(context.clone(), 4096, mode)?;
    let module = context.load_module(compile_ptx(KERNEL)?)?;
    let kernel = module.load_function("update")?;
    // Act
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut gpu = memory.gpu(stream, Access::Write).unwrap();
        // SAFETY: bounded write; only the lease's stream is used.
        unsafe {
            gpu.enqueue(|ptr, stream| {
                let len = 4096u32;
                stream
                    .launch_builder(&kernel)
                    .arg(&ptr)
                    .arg(&len)
                    .launch(LaunchConfig::for_num_elems(len))?;
                panic!("simulate caller panic after submission");
            })
            .unwrap();
        }
    }));
    // Assert
    assert!(result.is_err());
    assert!(memory.read()?.iter().all(|&x| x == 1));
    Ok(())
}

fn launch_error_keeps_reservation_until_retired(
    mode: AllocationMode,
) -> Result<(), Box<dyn Error>> {
    // Arrange
    let context = CudaContext::new(0)?;
    let stream = context.new_stream()?;
    let memory = allocate(context.clone(), 4096, mode)?;
    let module = context.load_module(compile_ptx(KERNEL)?)?;
    let kernel = module.load_function("update")?;
    let mut gpu = memory.gpu(stream, Access::Write)?;
    // Act
    // SAFETY: bounded exclusive write on the supplied stream. Simulate a
    // subsequent submission failure after one successful enqueue.
    let result = unsafe {
        gpu.enqueue(|ptr, stream| {
            let len = 4096u32;
            stream
                .launch_builder(&kernel)
                .arg(&ptr)
                .arg(&len)
                .launch(LaunchConfig::for_num_elems(len))?;
            Err(cudarc::driver::DriverError(
                cudarc::driver::sys::CUresult::CUDA_ERROR_INVALID_VALUE,
            ))
        })
    };
    // Assert
    assert!(result.is_err());
    assert!(matches!(memory.read(), Err(cuda_host_memory::Error::Busy)));
    // Act
    gpu.wait()?;

    // Assert
    assert!(memory.read()?.iter().all(|&x| x == 1));
    Ok(())
}

fn foreign_context_is_rejected(mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    // Arrange
    let context = CudaContext::new(0)?;
    let memory = allocate(context, 4096, mode)?;
    let other = CudaContext::new_non_primary(0, 0)?;
    // Act
    let result = memory.gpu(other.new_stream()?, Access::Read);

    // Assert
    assert!(matches!(result, Err(cuda_host_memory::Error::WrongContext)));
    memory.write()?[0] = 9;
    Ok(())
}

#[path = "../examples/support/mod.rs"]
mod support;

fn cpu_gpu_reads_actually_overlap(mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    // Arrange
    // The shared demo sets up the mapping and bounded handshake.

    // Act
    let result = support::concurrent_read(mode);

    // Assert
    assert!(result.is_ok(), "concurrent read: {result:?}");
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    CpuRead,
    CpuWrite,
    GpuRead,
    GpuWrite,
}
fn acquire(
    memory: &HostMappedMemory,
    stream: &std::sync::Arc<cuda_host_memory::CudaStream>,
    kind: Kind,
) -> Result<Box<dyn std::any::Any + Send>, cuda_host_memory::Error> {
    Ok(match kind {
        Kind::CpuRead => Box::new(memory.read()?),
        Kind::CpuWrite => Box::new(memory.write()?),
        Kind::GpuRead => Box::new(memory.gpu(stream.clone(), Access::Read)?),
        Kind::GpuWrite => Box::new(memory.gpu(stream.clone(), Access::Write)?),
    })
}

fn full_access_matrix_and_reacquisition(mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    // Arrange
    let ctx = CudaContext::new(0)?;
    let stream = ctx.new_stream()?;
    let memory = allocate(ctx, 4096, mode)?;
    let kinds = [Kind::CpuRead, Kind::CpuWrite, Kind::GpuRead, Kind::GpuWrite];
    for first in kinds {
        for second in kinds {
            let held = acquire(&memory, &stream, first)?;
            // Act
            let result = acquire(&memory, &stream, second);
            let readers = matches!(first, Kind::CpuRead | Kind::GpuRead)
                && matches!(second, Kind::CpuRead | Kind::GpuRead);
            // Assert
            assert_eq!(result.is_ok(), readers, "{first:?} then {second:?}");
            if !readers {
                assert!(matches!(result, Err(cuda_host_memory::Error::Busy)));
            }
            drop(result);
            drop(held);
            drop(acquire(&memory, &stream, second)?);
        }
    }
    Ok(())
}

fn simultaneous_cpu_thread_acquisitions(mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    // Arrange
    use std::sync::{Arc, Barrier};
    let ctx = CudaContext::new(0)?;
    let stream = ctx.new_stream()?;
    let memory = allocate(ctx, 4096, mode)?;
    for kind in [Kind::CpuWrite, Kind::GpuWrite, Kind::CpuRead, Kind::GpuRead] {
        let start = Arc::new(Barrier::new(3));
        let held = Arc::new(Barrier::new(3));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let (memory, stream, start, held) =
                    (memory.clone(), stream.clone(), start.clone(), held.clone());
                std::thread::spawn(move || {
                    // Act
                    start.wait();
                    let lease = acquire(&memory, &stream, kind);
                    let won = lease.is_ok();
                    // Neither contender releases until both have attempted acquisition.
                    held.wait();
                    drop(lease);
                    won
                })
            })
            .collect();
        // Act
        start.wait();
        held.wait();
        let successes = handles
            .into_iter()
            .map(|h| usize::from(h.join().unwrap()))
            .sum::<usize>();
        // Assert
        assert_eq!(
            successes,
            if matches!(kind, Kind::CpuRead | Kind::GpuRead) {
                2
            } else {
                1
            }
        );
        memory.write()?[0] = 1;
    }
    Ok(())
}

fn gpu_lease_is_last_owner_and_invalid_sizes_fail(
    mode: AllocationMode,
) -> Result<(), Box<dyn Error>> {
    // Arrange
    let ctx = CudaContext::new(0)?;
    for len in [0, usize::MAX] {
        // Act
        let result = allocate(ctx.clone(), len, mode);

        // Assert
        assert!(matches!(result, Err(cuda_host_memory::Error::Mapping(_))));
    }
    let stream = ctx.new_stream()?;
    let module = ctx.load_module(compile_ptx(KERNEL)?)?;
    let kernel = module.load_function("update")?;
    let memory = allocate(ctx, 4096, mode)?;
    let mut gpu = memory.gpu(stream.clone(), Access::Write)?;
    // Act
    drop(memory); // No other allocation owner survives.
    // SAFETY: the lease retains the mapping; kernel is bounded and uses its stream.
    unsafe {
        gpu.enqueue(|ptr, stream| {
            let len = 4096u32;
            stream
                .launch_builder(&kernel)
                .arg(&ptr)
                .arg(&len)
                .launch(LaunchConfig::for_num_elems(len))?;
            Ok(())
        })?;
    }
    // Assert
    // Completion must succeed while the lease is the sole owner.
    gpu.wait()?;
    Ok(())
}

fn two_stream_read_leases_retire_independently(mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    // Arrange
    let ctx = CudaContext::new(0)?;
    let a = ctx.new_stream()?;
    let b = ctx.new_stream()?;
    let memory = allocate(ctx.clone(), 4096, mode)?;
    memory.write()?.fill(7);
    let module = ctx.load_module(compile_ptx(
        r#"
        extern "C" __global__ void verify(const unsigned char* p) {
            unsigned int i = blockIdx.x * blockDim.x + threadIdx.x;
            if (i < 4096 && p[i] != 7) asm("trap;");
        }
    "#,
    )?)?;
    let kernel = module.load_function("verify")?;
    let mut first = memory.gpu(a, Access::Read)?;
    let mut second = memory.gpu(b, Access::Read)?;
    // Act
    for lease in [&mut first, &mut second] {
        // SAFETY: read-only, bounded kernel on each lease's own stream.
        unsafe {
            lease.enqueue(|ptr, stream| {
                stream
                    .launch_builder(&kernel)
                    .arg(&ptr)
                    .launch(LaunchConfig::for_num_elems(4096))?;
                Ok(())
            })?;
        }
    }
    first.wait()?;
    // Assert
    assert!(matches!(memory.write(), Err(cuda_host_memory::Error::Busy)));
    assert!(memory.read()?.iter().all(|&b| b == 7));
    // Act
    second.wait()?;

    // Assert
    memory.write()?.fill(9);
    Ok(())
}

fn cpu_unwind_releases_write_reservation(mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    // Arrange
    let memory = allocate(CudaContext::new(0)?, 4096, mode)?;
    // Act
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut guard = memory.write().unwrap();
        guard[0] = 11;
        panic!("intentional CPU unwind");
    }));
    // Assert
    assert!(result.is_err());
    assert_eq!(memory.read()?[0], 11);
    memory.write()?[0] = 12;
    Ok(())
}

// Exercise the default constructor for registered mode, explicit opt-in for managed.
fn allocate(
    ctx: std::sync::Arc<CudaContext>,
    len: usize,
    mode: AllocationMode,
) -> Result<HostMappedMemory, cuda_host_memory::Error> {
    match mode {
        AllocationMode::Registered => HostMappedMemory::new(ctx, len),
        AllocationMode::Managed => HostMappedMemory::new_with_mode(ctx, len, mode),
    }
}

macro_rules! allocation_suite {
    ($module:ident, $mode:expr, [$($scenario:ident),+ $(,)?]) => {
        mod $module {
            $(
                #[test]
                #[ignore = "requires CUDA GPU, runtime, NVRTC and toolkit headers"]
                fn $scenario() -> Result<(), Box<dyn std::error::Error>> {
                    // Arrange
                    let mode = $mode;
                    // Act / Assert: shared scenario owns its phases and checks.
                    super::$scenario(mode)
                }
            )+
        }
    };
}
macro_rules! both_allocations {
    ($($scenario:ident),+ $(,)?) => {
        allocation_suite!(registered, cuda_host_memory::AllocationMode::Registered, [$($scenario),+]);
        allocation_suite!(managed, cuda_host_memory::AllocationMode::Managed, [$($scenario),+]);
    };
}
both_allocations!(
    mapped_memory_on_non_default_stream,
    leases_retain_storage_and_exclude_conflicts,
    unwind_after_enqueue_retires_gpu_use,
    launch_error_keeps_reservation_until_retired,
    foreign_context_is_rejected,
    cpu_gpu_reads_actually_overlap,
    full_access_matrix_and_reacquisition,
    simultaneous_cpu_thread_acquisitions,
    gpu_lease_is_last_owner_and_invalid_sizes_fail,
    two_stream_read_leases_retire_independently,
    cpu_unwind_releases_write_reservation,
);
