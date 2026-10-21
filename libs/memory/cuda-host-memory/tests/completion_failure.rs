//! CUDA faults poison their context: run the actual fault only in a child process.
use cuda_host_memory::{Access, AllocationMode, CudaContext, Error, HostMappedMemory};
use cudarc::{driver::LaunchConfig, nvrtc::compile_ptx};

#[test]
#[ignore = "requires CUDA; deliberately faults an isolated child CUDA context"]
fn completion_failure_quarantines_all_access() {
    // Arrange
    let executable = std::env::current_exe().unwrap();

    for mode in ["registered", "managed"] {
        // Act
        let output = std::process::Command::new(&executable)
            .args(["--ignored", "--exact", "fault_child", "--nocapture"])
            .env("CUDA_HOST_MEMORY_FAULT_CHILD", "1")
            .env("CUDA_HOST_MEMORY_MODE", mode)
            .output()
            .unwrap();
        // Assert
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("QUARANTINE VERIFIED"));
    }
}

#[test]
#[ignore = "helper: launched only by completion_failure_quarantines_all_access"]
fn fault_child() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("CUDA_HOST_MEMORY_FAULT_CHILD").is_none() {
        return Ok(());
    }
    // Arrange
    let ctx = CudaContext::new(0)?;
    let stream = ctx.new_stream()?;
    let mode = match std::env::var("CUDA_HOST_MEMORY_MODE")?.as_str() {
        "registered" => AllocationMode::Registered,
        "managed" => AllocationMode::Managed,
        other => panic!("unknown allocation mode {other}"),
    };
    let memory = HostMappedMemory::new_with_mode(ctx.clone(), 4096, mode)?;
    let module = ctx.load_module(compile_ptx(
        r#"
        extern "C" __global__ void fail() { asm("trap;"); }
    "#,
    )?)?;
    let kernel = module.load_function("fail")?;
    let mut lease = memory.gpu(stream.clone(), Access::Write)?;
    // Act
    // SAFETY: kernel accesses no memory and deliberately traps to report a device execution failure.
    unsafe {
        lease.enqueue(|_, stream| {
            stream
                .launch_builder(&kernel)
                .launch(LaunchConfig::for_num_elems(1))?;
            Ok(())
        })?;
    }
    let completion = lease.wait();

    // Assert
    assert!(matches!(completion, Err(Error::Driver(_))));
    assert!(matches!(memory.read(), Err(Error::Poisoned)));
    assert!(matches!(memory.write(), Err(Error::Poisoned)));
    assert!(matches!(
        memory.gpu(stream.clone(), Access::Read),
        Err(Error::Poisoned)
    ));
    assert!(matches!(
        memory.gpu(stream, Access::Write),
        Err(Error::Poisoned)
    ));
    println!("QUARANTINE VERIFIED");
    Ok(())
}
