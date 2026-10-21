use cuda_host_memory::{Access, CudaContext, Error, HostMappedMemory};
use memmap2::MmapOptions;
use std::io::{Read, Seek, SeekFrom, Write};

#[test]
#[ignore = "requires CUDA host registration"]
fn existing_file_contents_and_guarded_access_survive_registration() -> eyre::Result<()> {
    // Arrange
    let ctx = CudaContext::new(0)?;
    let stream = ctx.new_stream()?;
    let mut file = tempfile::tempfile_in("/dev/shm")?;
    file.write_all(&vec![42u8; 4096])?;
    // SAFETY: test owns file; no truncation or unguarded concurrent access.
    let mapping = unsafe { MmapOptions::new().map_mut(&file)? };
    // Act
    // SAFETY: file is accessed only before registration and after owner drop.
    let memory = unsafe { HostMappedMemory::from_mapping(ctx, mapping)? };
    let gpu = memory.gpu(stream, Access::Read)?;
    // Assert
    assert!(memory.read()?.iter().all(|&b| b == 42));
    assert!(matches!(memory.write(), Err(Error::Busy)));
    // Act
    gpu.wait()?;
    memory.write()?.fill(43);
    drop(memory);
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    // Assert
    assert_eq!(bytes, vec![43; 4096]);
    Ok(())
}

#[test]
#[ignore = "requires CUDA and sufficient RLIMIT_MEMLOCK"]
fn one_ring_file_is_locked_in_ram() -> eyre::Result<()> {
    // Arrange
    let ctx = CudaContext::new(0)?;
    // Act
    // SAFETY: test exclusively controls backing files and admits no consumers.
    let pool = unsafe {
        cuda_dora_shm::Producer::new(
            ctx,
            cuda_dora_shm::PoolConfig {
                slots: 3,
                capacity: 4097,
                peers: vec!["reader".into()],
                allocation: cuda_host_memory::AllocationMode::Registered,
            },
        )?
    };
    let manifest = pool.manifest();
    let files: Vec<_> = std::fs::read_dir(&manifest.directory)?.collect();
    let path = format!("{}/ring", manifest.directory);
    let smaps = std::fs::read_to_string("/proc/self/smaps")?;
    let section = smaps.split(&path).nth(1).expect("ring mapping in smaps");
    let locked: usize = section
        .lines()
        .find(|l| l.starts_with("Locked:"))
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()?;
    // Assert
    assert_eq!(files.len(), 1);
    assert_eq!(manifest.stride, 4160);
    assert_eq!(
        std::fs::metadata(&path)?.len(),
        (3 * manifest.stride) as u64
    );
    assert!(locked * 1024 >= 3 * manifest.stride);
    // Act
    drop(pool);
    // Assert: final slot drop unmaps and releases the process's lock.
    assert!(!std::fs::read_to_string("/proc/self/smaps")?.contains(&path));
    Ok(())
}

#[test]
#[ignore = "requires CUDA; isolates RLIMIT_MEMLOCK changes in a child"]
fn lock_failure_rejects_pool_without_fallback() -> eyre::Result<()> {
    if std::env::var_os("CUDA_RING_LOCK_CHILD").is_some() {
        // Arrange
        let ctx = CudaContext::new(0)?;
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: this isolated child lowers only its own memory-lock limit.
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &limit) }, 0);
        // Act
        // SAFETY: no external access to this test's private backing files.
        let result = unsafe {
            cuda_dora_shm::Producer::new(
                ctx,
                cuda_dora_shm::PoolConfig {
                    slots: 1,
                    capacity: 4096,
                    peers: vec!["reader".into()],
                    allocation: cuda_host_memory::AllocationMode::Registered,
                },
            )
        };
        // Assert
        let error = result
            .err()
            .expect("mlock must fail for unprivileged child");
        assert!(error.to_string().contains("mlock entire pool"), "{error}");
        return Ok(());
    }
    // Arrange
    let executable = std::env::current_exe()?;
    // Act
    let output = std::process::Command::new(executable)
        .args([
            "--ignored",
            "--exact",
            "lock_failure_rejects_pool_without_fallback",
        ])
        .env("CUDA_RING_LOCK_CHILD", "1")
        .output()?;
    // Assert
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
