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
