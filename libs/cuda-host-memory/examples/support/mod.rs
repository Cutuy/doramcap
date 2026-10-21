//! Test/demo-only GPU handshake. Independent control pages, never payload bytes.
use cuda_host_memory::{Access, AllocationMode, CudaContext, HostMappedMemory};
use cudarc::{
    driver::{LaunchConfig, PushKernelArg},
    runtime::sys,
};
use std::{
    error::Error,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};

// Only aligned 32-bit system-scope loads/stores are used, not CPU/GPU RMW.
// CUDA guarantees these mapped-memory operations on supported CUDA platforms.
struct Control {
    map: Option<memmap2::MmapMut>,
    ctx: std::sync::Arc<CudaContext>,
    ptr: u64,
    host_addr: usize,
}
impl Control {
    fn new(ctx: std::sync::Arc<CudaContext>) -> Result<Self, Box<dyn Error>> {
        let mut map = memmap2::MmapMut::map_anon(4096)?;
        let host = map.as_mut_ptr();
        // SAFETY: fresh page-aligned mapping; initialize atomics before sharing.
        unsafe {
            for i in 0..4 {
                host.cast::<AtomicU32>().add(i).write(AtomicU32::new(0));
            }
            let code = sys::cudaHostRegister(host.cast(), map.len(), sys::cudaHostRegisterMapped);
            if code != sys::cudaError::cudaSuccess {
                return Err(format!("register control: {code:?}").into());
            }
        }
        let mut out = Self {
            map: Some(map),
            ctx,
            ptr: 0,
            host_addr: host as usize,
        };
        let mut device = std::ptr::null_mut();
        // SAFETY: registered mapping remains alive through stream completion.
        let code = unsafe { sys::cudaHostGetDevicePointer(&mut device, host.cast(), 0) };
        if code != sys::cudaError::cudaSuccess {
            return Err(format!("map control: {code:?}").into());
        }
        out.ptr = device as u64;
        Ok(out)
    }
    fn word(&self, i: usize) -> &AtomicU32 {
        assert!(i < 4);
        // SAFETY: aligned, initialized, CPU/device access is exclusively atomic.
        unsafe { &*(self.host_addr as *const AtomicU32).add(i) }
    }
}
impl Drop for Control {
    fn drop(&mut self) {
        let mapping = self.map.take().expect("control mapping");
        // Synchronize before unregistering; retain storage if completion failed.
        let safe_to_unmap = self.ctx.synchronize().is_ok()
            && unsafe {
                sys::cudaHostUnregister((self.host_addr as *mut u8).cast())
                    == sys::cudaError::cudaSuccess
            };
        if !safe_to_unmap {
            std::mem::forget(mapping);
            std::mem::forget(self.ctx.clone());
        }
    }
}

const SOURCE: &str = r#"
#include <cuda/atomic>
extern "C" __global__ void concurrent_read(const volatile unsigned char* data,
                                          unsigned int* control) {
    cuda::atomic_ref<unsigned int, cuda::thread_scope_system> started(control[0]);
    cuda::atomic_ref<unsigned int, cuda::thread_scope_system> release(control[1]);
    cuda::atomic_ref<unsigned int, cuda::thread_scope_system> progress(control[2]);
    cuda::atomic_ref<unsigned int, cuda::thread_scope_system> result(control[3]);
    started.store(1, cuda::memory_order_release);
    unsigned int rounds = 0;
    unsigned long long start = clock64();
    // Finite watchdog: a failed CPU test cannot hang GPU-lease Drop indefinitely.
    while (!release.load(cuda::memory_order_acquire) && clock64() - start < 2000000000ULL) {
        unsigned int sum = 0;
        for (unsigned int i = 0; i < 4096; ++i) sum += data[i];
        if (sum != 4096 * 7) { result.store(2, cuda::memory_order_release); return; }
        progress.store(++rounds, cuda::memory_order_release);
    }
    result.store(release.load(cuda::memory_order_acquire) ? 1 : 3, cuda::memory_order_release);
}
"#;

/// Proves GPU read progress between two CPU reads, while both keep read leases.
pub fn concurrent_read(mode: AllocationMode) -> Result<(), Box<dyn Error>> {
    let ctx = CudaContext::new(0)?;
    let stream = ctx.new_stream()?;
    let memory = HostMappedMemory::new_with_mode(ctx.clone(), 4096, mode)?;
    memory.write()?.fill(7);
    // CCCL header is supplied by the installed CUDA toolkit for this demo.
    let opts = cudarc::nvrtc::CompileOptions {
        include_paths: vec![format!(
            "{}/include",
            std::env::var("CUDA_HOME").unwrap_or_else(|_| "/usr/local/cuda".into())
        )],
        ..Default::default()
    };
    let module = ctx.load_module(cudarc::nvrtc::compile_ptx_with_opts(SOURCE, opts)?)?;
    let kernel = module.load_function("concurrent_read")?;
    let control = Control::new(ctx.clone())?;
    let cpu = memory.read()?;
    let mut gpu = memory.gpu(stream.clone(), Access::Read)?;
    // SAFETY: payload is read-only and bounded; separate control uses aligned
    // system-scope atomic loads/stores. Both mappings outlive stream completion.
    unsafe {
        gpu.enqueue(|ptr, stream| {
            stream
                .launch_builder(&kernel)
                .arg(&ptr)
                .arg(&control.ptr)
                .launch(LaunchConfig {
                    grid_dim: (1, 1, 1),
                    block_dim: (1, 1, 1),
                    shared_mem_bytes: 0,
                })?;
            Ok(())
        })?;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let wait_for = |condition: &dyn Fn() -> bool| -> Result<(), Box<dyn Error>> {
        while !condition() {
            if Instant::now() > deadline {
                return Err("GPU handshake timed out".into());
            }
            std::thread::yield_now();
        }
        Ok(())
    };
    wait_for(&|| control.word(0).load(Ordering::Acquire) == 1)?;
    let before = control.word(2).load(Ordering::Acquire);
    assert_eq!(cpu.iter().map(|&b| b as u32).sum::<u32>(), 4096 * 7);
    wait_for(&|| control.word(2).load(Ordering::Acquire) > before)?;
    assert_eq!(cpu.iter().map(|&b| b as u32).sum::<u32>(), 4096 * 7);
    assert!(matches!(memory.write(), Err(cuda_host_memory::Error::Busy)));
    assert!(matches!(
        memory.gpu(stream, Access::Write),
        Err(cuda_host_memory::Error::Busy)
    ));
    control.word(1).store(1, Ordering::Release);
    gpu.wait()?;
    assert_eq!(
        control.word(3).load(Ordering::Acquire),
        1,
        "kernel timed out or checksum failed"
    );
    assert!(matches!(memory.write(), Err(cuda_host_memory::Error::Busy)));
    drop(cpu);
    memory.write()?[0] = 9;
    println!(
        "PASS: GPU made read progress between CPU reads; both writers rejected; reuse succeeds after both readers retire"
    );
    Ok(())
}
