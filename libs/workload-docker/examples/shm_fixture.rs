//! Native POSIX/SysV payload fixture: `shm_fixture write|read|absent|remove KEY NAME`.
use std::{io, ptr};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let key = args[2].parse::<libc::key_t>()?;
    let path = std::path::Path::new("/dev/shm").join(&args[3]);
    let payload = b"producer-exited-data";
    let write = args[1] == "write";
    // SAFETY: shmget only creates/looks up the requested test key; no borrowed pointers.
    let id = unsafe {
        libc::shmget(
            key,
            payload.len(),
            if write {
                libc::IPC_CREAT | libc::IPC_EXCL | 0o600
            } else {
                0
            },
        )
    };
    if args[1] == "absent" {
        assert!(!path.exists());
        assert_eq!(id, -1);
        assert_eq!(
            io::Error::last_os_error().raw_os_error(),
            Some(libc::ENOENT)
        );
        return Ok(());
    }
    if id == -1 {
        return Err(io::Error::last_os_error().into());
    }
    if args[1] == "remove" {
        std::fs::remove_file(path)?;
        // SAFETY: IPC_RMID ignores its third argument and removes our test segment.
        if unsafe { libc::shmctl(id, libc::IPC_RMID, ptr::null_mut()) } == -1 {
            return Err(io::Error::last_os_error().into());
        }
        return Ok(());
    }
    // SAFETY: the returned segment has payload.len() bytes and stays attached below.
    let memory = unsafe { libc::shmat(id, ptr::null(), if write { 0 } else { libc::SHM_RDONLY }) };
    if memory == (-1isize) as *mut libc::c_void {
        return Err(io::Error::last_os_error().into());
    }
    if write {
        std::fs::write(path, payload)?;
        // SAFETY: the attached writable region has the size requested by shmget.
        unsafe { ptr::copy_nonoverlapping(payload.as_ptr(), memory.cast(), payload.len()) };
    } else {
        assert_eq!(std::fs::read(path)?, payload);
        // SAFETY: the attached readable region has the size requested by shmget.
        assert_eq!(
            unsafe { std::slice::from_raw_parts(memory.cast::<u8>(), payload.len()) },
            payload
        );
    }
    // SAFETY: memory is the successful shmat result, used no further after detach.
    if unsafe { libc::shmdt(memory) } == -1 {
        return Err(io::Error::last_os_error().into());
    }
    Ok(())
}
