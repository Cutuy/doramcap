//! Native accounting test worker: MARKER SOCKET, run inside a delegated service.
use std::{
    fs,
    hint::black_box,
    io::{Read, Write},
    os::unix::net::UnixListener,
    path::Path,
    time::{Duration, Instant},
};

const PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
const CPU_WORK: Duration = Duration::from_millis(100);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [marker, socket] = args.as_slice() else {
        return Err("expected MARKER SOCKET".into());
    };
    let listener = UnixListener::bind(socket)?;
    let membership = fs::read_to_string("/proc/self/cgroup")?;
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .ok_or("cgroup v2 missing")?;
    let parent =
        Path::new(workload_systemd::cgroup::CGROUP_ROOT).join(relative.trim_start_matches('/'));
    let child = parent.join("work");
    fs::create_dir(&child)?;
    fs::write(child.join("cgroup.procs"), std::process::id().to_string())?;
    fs::write(parent.join("cgroup.subtree_control"), "+cpu +memory")?;
    let payload = vec![black_box(0x35u8); PAYLOAD_BYTES];
    let start = Instant::now();
    let mut value = 0u64;
    while start.elapsed() < CPU_WORK {
        value = black_box(value.wrapping_add(1));
    }
    fs::File::create(marker)?.write_all(std::process::id().to_string().as_bytes())?;
    let (mut client, _) = listener.accept()?;
    client.read_exact(&mut [0u8])?;
    black_box((payload, value));
    Ok(())
}
