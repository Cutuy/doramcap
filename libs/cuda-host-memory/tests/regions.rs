use cuda_host_memory::{Access, CudaContext, Error, HostMappedMemory};
use cudarc::{
    driver::{LaunchConfig, PushKernelArg},
    nvrtc::compile_ptx,
};

#[test]
#[ignore = "requires CUDA and NVRTC"]
fn disjoint_slots_allow_cpu_reads_and_gpu_writes_and_retain_backing()
-> Result<(), Box<dyn std::error::Error>> {
    // Arrange
    let ctx = CudaContext::new(0)?;
    let stream = ctx.new_stream()?;
    let memory = HostMappedMemory::new(ctx.clone(), 3 * 4096)?;
    let slots = memory.into_slots(3, 4096)?;
    let module = ctx.load_module(compile_ptx(
        "extern \"C\" __global__ void fill(unsigned char* p) { if (threadIdx.x < 32) p[threadIdx.x] = 43; }",
    )?)?;
    let kernel = module.load_function("fill")?;
    let reader = slots[0].read()?;
    let mut other_writer = slots[2].write()?;
    let mut gpu = slots[1].gpu(stream, Access::Write)?;
    // Act
    // SAFETY: writes 32 bytes within slot 1, on the lease's stream.
    unsafe {
        gpu.enqueue(|ptr, stream| {
            stream
                .launch_builder(&kernel)
                .arg(&ptr)
                .launch(LaunchConfig::for_num_elems(32))?;
            Ok(())
        })?;
    }
    other_writer[0] = 9;
    // Assert
    assert!(matches!(slots[1].read(), Err(Error::Busy)));
    assert!(matches!(slots[0].write(), Err(Error::Busy)));
    assert!(reader.iter().all(|&b| b == 0));
    // Act
    gpu.wait()?;
    let retained = slots[1].read()?;
    drop(reader);
    drop(other_writer);
    drop(slots);
    // Assert: guard alone owns the entire registered backing.
    assert_eq!(&retained[..32], &[43; 32]);
    assert!(retained[32..].iter().all(|&b| b == 0));
    Ok(())
}

#[test]
#[ignore = "requires CUDA"]
fn partition_rejects_aliases_leases_and_invalid_layouts() -> Result<(), Box<dyn std::error::Error>>
{
    // Arrange
    let ctx = CudaContext::new(0)?;
    let memory = HostMappedMemory::new(ctx.clone(), 8192)?;
    // Act
    let aliased = memory.clone().into_slots(2, 4096);
    let guard = memory.read()?;
    let leased = memory.into_slots(2, 4096);
    // Assert
    assert!(matches!(aliased, Err(Error::Busy)));
    assert!(matches!(leased, Err(Error::Busy)));
    assert_eq!(guard.len(), 8192);
    for (count, stride) in [(0, 4096), (2, 0), (3, 4096), (usize::MAX, 2)] {
        // Arrange
        let memory = HostMappedMemory::new(ctx.clone(), 8192)?;
        // Act
        let invalid = memory.into_slots(count, stride);
        // Assert
        assert!(matches!(invalid, Err(Error::Mapping(_))));
    }
    Ok(())
}
