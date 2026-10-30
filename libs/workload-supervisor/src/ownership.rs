//! Durable ownership and handle-based containment for one foreground daemon.
use crate::{Ownership, Result, error};
use prost::Message;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

pub(crate) struct Runtime {
    pub child: Option<tokio::process::Child>,
    pub kill: Option<File>,
    pub ledger: Ownership,
    pub forced: bool,
    pub daemon_reaped: bool,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        if !self.ledger.cleanup_verified {
            // Best effort on manager-future cancellation. Durable evidence remains unfinished.
            if let Some(kill) = &mut self.kill {
                let _ = kill.write_all(b"1");
            }
            if let Some(child) = &mut self.child {
                let _ = child.start_kill();
            }
        }
    }
}
pub(crate) fn private_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(error)?;
    file.write_all(bytes).map_err(error)?;
    file.sync_all().map_err(error)?;
    File::open(path.parent().ok_or("missing parent")?)
        .map_err(error)?
        .sync_all()
        .map_err(error)
}
pub(crate) fn save(ledger: &Ownership) -> Result<()> {
    let run = ledger.run.as_ref().ok_or("ownership run missing")?;
    let dir = Path::new(&run.output_directory);
    let temp = dir.join("ownership.next");
    private_file(&temp, &ledger.encode_to_vec())?;
    std::fs::rename(temp, dir.join("ownership.pb")).map_err(error)?;
    File::open(dir).map_err(error)?.sync_all().map_err(error)
}
pub(crate) fn verify(ledger: &Ownership) -> Result<()> {
    if ledger.cgroup_inode == 0
        || ledger.boot_id
            != std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .map_err(error)?
                .trim()
    {
        return Err("unknown cgroup ownership or boot identity".into());
    }
    let meta = std::fs::symlink_metadata(&ledger.cgroup).map_err(error)?;
    if !meta.is_dir() || meta.dev() != ledger.cgroup_device || meta.ino() != ledger.cgroup_inode {
        return Err("cgroup ownership identity changed; refusing signals".into());
    }
    Ok(())
}
pub(crate) fn empty(ledger: &Ownership) -> Result<bool> {
    verify(ledger)?;
    let events =
        std::fs::read_to_string(Path::new(&ledger.cgroup).join("cgroup.events")).map_err(error)?;
    Ok(events.lines().any(|line| line == "populated 0"))
}

pub(crate) fn start_ticks(pid: u32) -> Result<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).map_err(error)?;
    Ok(stat
        .rsplit_once(')')
        .ok_or("invalid proc stat")?
        .1
        .split_whitespace()
        .nth(19)
        .ok_or("start time absent")?
        .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_or_mismatched_evidence_never_establishes_ownership() {
        // Arrange
        let mut evidence = Ownership {
            cgroup: "/sys/fs/cgroup".into(),
            boot_id: std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .unwrap()
                .trim()
                .into(),
            ..Default::default()
        };
        // Act / Assert
        assert!(verify(&evidence).is_err());
        // Arrange: a recorded inode cannot authorize a different live object.
        evidence.cgroup_inode = u64::MAX;
        evidence.cgroup_device = u64::MAX;
        // Act / Assert
        assert!(verify(&evidence).is_err());
    }
}
