//! Docker socket-peer mount visibility.
use crate::{Result, error::invalid};
use std::path::Path;
/// Require the local Docker socket peer to share this process's mount namespace.
/// Overlay binds must resolve there; proxies require separate namespace guarantees.
pub async fn verify_namespace(socket: &Path) -> Result<()> {
    use std::os::fd::AsRawFd;
    let stream = tokio::net::UnixStream::connect(socket).await?;
    let mut cred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: destination and length point to correctly sized live storage.
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut len,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let ours = tokio::fs::read_link("/proc/self/ns/mnt").await?;
    let theirs = tokio::fs::read_link(format!("/proc/{}/ns/mnt", cred.pid)).await?;
    if ours != theirs {
        return Err(invalid(
            "Docker peer and manager mount namespaces differ; host overlay visibility is unqualified",
        ));
    }
    Ok(())
}
