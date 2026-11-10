#[path = "../src/support.rs"]
mod support;

#[test]
#[ignore = "requires an NVIDIA CUDA GPU, NVRTC, and Linux O_DIRECT/io_uring"]
fn cuda_registered_slabs_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    // Arrange: cover partial pages, large payloads, and more messages than slabs.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    for mode in [
        support::Mode::Mapped,
        support::Mode::Staged,
        support::Mode::IoOnly,
    ] {
        assert_eq!(support::Mode::parse(mode.name())?, mode);
        for payload in [1, 4096 - 31, 65536 + 7, 1024 * 1024] {
            let path = std::env::temp_dir().join(format!(
                "cuda-mcap-{}-{stamp}-{}-{payload}.mcap",
                std::process::id(),
                mode.name()
            ));
            // Act: register real slabs, run CUDA on a non-default stream, and finalize.
            let result = support::run(&path, mode, payload, 19, 3);
            // Assert: run() checks every byte with the independent MCAP decoder,
            // exact message counts, allocation identity, and complete slab recycling.
            let _ = std::fs::remove_file(&path);
            result?.print();
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires an NVIDIA CUDA GPU, NVRTC, and Linux O_DIRECT/io_uring"]
fn large_cuda_slabs_use_writer_registration_budget() -> Result<(), Box<dyn std::error::Error>> {
    // Arrange: four 64-MiB payload slabs exceed u16::MAX when counted as pages.
    const PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
    const SLABS: usize = 4;
    const MESSAGES: u32 = 5;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    for mode in [support::Mode::Mapped, support::Mode::Staged] {
        let path = std::env::temp_dir().join(format!(
            "cuda-mcap-large-{}-{stamp}-{}.mcap",
            std::process::id(),
            mode.name()
        ));
        // Act: create the writer, recycle registered slabs and finalize real CUDA output.
        let result = support::run(&path, mode, PAYLOAD_BYTES, MESSAGES, SLABS);
        let _ = std::fs::remove_file(&path);
        // Assert: run() checks every payload byte and complete slab recycling.
        result?.print();
    }
    Ok(())
}
