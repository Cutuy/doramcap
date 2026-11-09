//! Native shared-memory test worker: `producer|consumer|absent NAME MARKER DONE` or `live SOURCE OBSERVED DONE READ_ONLY`.
use std::{
    fs,
    io::{self, Write},
    path::Path,
    thread,
    time::{Duration, Instant},
};

const WAIT_LIMIT: Duration = Duration::from_secs(10);
const CHECK_INTERVAL: Duration = Duration::from_millis(10);

fn wait_for(mut ready: impl FnMut() -> bool) -> io::Result<()> {
    let deadline = Instant::now() + WAIT_LIMIT;
    while !ready() {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "fixture condition did not become ready",
            ));
        }
        thread::sleep(CHECK_INTERVAL);
    }
    Ok(())
}

fn memory(role: &str, name: &str, marker: &Path, done: &Path) -> io::Result<()> {
    let path = Path::new(workload_lifecycle::files::SHM_PATH).join(format!("doramcap-test-{name}"));
    match role {
        "producer" => {
            fs::File::create_new(path)?.write_all(b"live data")?;
            fs::write(marker, std::process::id().to_string())?;
        }
        "consumer" => {
            wait_for(|| marker.exists())?;
            let pid = fs::read_to_string(marker)?;
            let process = Path::new("/proc").join(pid.trim());
            wait_for(|| !process.exists())?;
            assert_eq!(fs::read(path)?, b"live data");
            fs::write(done, "Shared file read after producer exit")?;
        }
        "absent" => assert!(!path.exists()),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown memory fixture role",
            ));
        }
    }
    Ok(())
}

fn live(source: &Path, observed: &Path, done: &Path, read_only: bool) -> io::Result<()> {
    assert_eq!(fs::read(source)?, b"initial");
    fs::File::create(observed)?;
    wait_for(|| fs::read(source).unwrap() == b"host update")?;
    match fs::write(source, "app update") {
        Ok(()) => assert!(!read_only, "read-only source accepted a write"),
        Err(error) => {
            assert!(read_only);
            assert!(
                error.kind() == io::ErrorKind::PermissionDenied
                    || error.raw_os_error() == Some(rustix::io::Errno::ROFS.raw_os_error()),
                "{error}"
            );
        }
    }
    fs::write(done, "live mount verified")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [mode, source, observed, done, read_only] if mode == "live" => live(
            Path::new(source),
            Path::new(observed),
            Path::new(done),
            read_only.parse()?,
        )?,
        [role, name, marker, done] => memory(role, name, Path::new(marker), Path::new(done))?,
        _ => return Err("expected a shared-memory fixture command and its arguments".into()),
    }
    Ok(())
}
