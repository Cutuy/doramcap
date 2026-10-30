//! Compile with rustc for privileged lifecycle fault injection; never shipped.
use std::{fs, process::Command, thread, time::Duration};
unsafe extern "C" {
    fn signal(signal: i32, handler: usize) -> usize;
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|a| a == "--check" || a == "--check-bpf") {
        println!(
            "{{\"bpf_compiled\":true,\"kernel_version_ok\":true,\"cgroup_v2\":true,\"btf_readable\":true,\"attachment_verified\":true}}"
        );
        return;
    }
    let executable = std::env::current_exe().unwrap();
    if executable
        .file_name()
        .unwrap()
        .to_string_lossy()
        .contains("early")
    {
        std::process::exit(23);
    }
    // Ignore TERM and keep a descendant to exercise group/cgroup escalation.
    unsafe {
        signal(15, 1);
    }
    let _descendant = Command::new("/bin/sleep").arg("300").spawn().unwrap();
    let value = |flag| args[args.iter().position(|a| a == flag).unwrap() + 1].clone();
    let stream = value("--output");
    fs::write(value("--record"), b"\x89MCAP0\r\n").unwrap();
    let mut bytes = vec![1];
    loop {
        fs::write(&stream, &bytes).unwrap();
        bytes.push(1);
        thread::sleep(Duration::from_millis(100));
    }
}
