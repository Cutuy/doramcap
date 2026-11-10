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
