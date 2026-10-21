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
    let context = CudaContext::new(0)?;
    let stream = context.new_stream()?; // CU_STREAM_NON_BLOCKING, not stream 0.
    assert!(!stream.cu_stream().is_null());
    let module = context.load_module(compile_ptx(KERNEL)?)?;
    let kernel = module.load_function("update")?;
    let memory = HostMappedMemory::new_with_mode(context, len as usize, mode)?;
    for (i, byte) in memory.write()?.iter_mut().enumerate() {
        *byte = (i % 251) as u8;
    }
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
    for (i, &byte) in memory.read()?.iter().enumerate() {
        assert_eq!(byte, (i % 251) as u8 + 2, "byte {i}");
    }
    println!(
        "PASS: {len} bytes; checked CPU/GPU leases; two kernels on non-default stream {:?}",
        stream.cu_stream()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mode = if std::env::args().any(|arg| arg == "--managed") {
        AllocationMode::Managed
    } else {
        AllocationMode::Registered
    };
    exercise(1024 * 1024, mode)
}
