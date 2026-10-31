//! Run the host Console worker through its public socket protocol.
use std::{
    io::{Read, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
    process::Command,
    time::Duration,
};

#[test]
#[ignore = "requires native Docker/BPF/OverlayFS capabilities and a monitor image"]
fn console_runs_successive_experiments_in_the_same_repository_project() {
    // Arrange
    let root = std::env::temp_dir().join(format!("console-worker-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let output = root.join("runs");
    std::fs::create_dir(&output).unwrap();
    let socket = root.join("worker.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let image = std::env::var("DORAMCAP_MONITOR_IMAGE").unwrap();
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_owned();
    let mut worker = Command::new(env!("CARGO_BIN_EXE_worker"))
        .args([
            socket.to_str().unwrap(),
            &image,
            output.to_str().unwrap(),
            repository.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    let (mut stream, _) = listener.accept().unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(45)))
        .unwrap();
    let mut projects = Vec::new();
    for _ in 0..2 {
        // Act
        stream.write_all(b"{\"inject_failure\":false}\n").unwrap();
        loop {
            let mut header = [0u8; 5];
            stream.read_exact(&mut header).unwrap();
            let length = u32::from_be_bytes(header[1..].try_into().unwrap()) as usize;
            assert!(length <= 4 * 1024 * 1024);
            let mut body = vec![0; length];
            stream.read_exact(&mut body).unwrap();
            if header[0] != 2 {
                continue;
            }
            let status: serde_json::Value = serde_json::from_slice(&body).unwrap();
            // Assert
            assert!(status["error"].is_null(), "{status}");
            assert_eq!(status["report"]["boundary_clean"], true, "{status}");
            assert_eq!(status["report"]["recording_finalized"], true, "{status}");
            projects.push(status["report"]["cleanup"]["project"].clone());
            break;
        }
    }
    assert_eq!(projects[0], projects[1]);
    // Act / Assert: closing control input permits the worker to exit cleanly.
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    drop(stream);
    assert!(worker.wait().unwrap().success());
}
