//! Exclusive ownership and crash-safe marker persistence for the data root.
use crate::{
    policy,
    protocol::{Marker, Phase},
};
use prost::Message;
use rustix::fs::{FlockOperation, OFlags, flock};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

/// Restored admission state; unfinished or invalid markers always block.
pub enum Restored {
    Idle,
    Finished(Marker),
    Blocked(Option<Marker>),
}

/// Holds the data-root lock for the lifetime of one Console owner.
pub struct StateStore {
    root: PathBuf,
    _lock: File,
}

impl StateStore {
    /// Acquire the exclusive lock and inspect the persisted marker.
    pub fn open(root: &Path) -> Result<(Self, Restored), String> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(root)
            .map_err(|e| e.to_string())?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(OFlags::NOFOLLOW.bits() as i32)
            .open(root.join("console.lock"))
            .map_err(|e| e.to_string())?;
        if !lock.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("data-root lock is not a regular file".into());
        }
        flock(&lock, FlockOperation::NonBlockingLockExclusive)
            .map_err(|e| format!("data root is already in use: {e}"))?;
        let store = Self { root, _lock: lock };
        let restored = match store.read_marker() {
            Ok(None) => Restored::Idle,
            Ok(Some(marker))
                if marker.version == policy::MARKER_VERSION
                    && marker.outcome.as_ref().and_then(|o| o.boundary_clean) == Some(true)
                    && marker.phase == Phase::Idle as i32 =>
            {
                Restored::Idle
            }
            Ok(Some(marker))
                if marker.version == policy::MARKER_VERSION
                    && marker.outcome.as_ref().and_then(|o| o.boundary_clean) == Some(true)
                    && marker.phase == Phase::Finished as i32 =>
            {
                Restored::Finished(marker)
            }
            Ok(Some(marker)) => Restored::Blocked(Some(marker)),
            Err(_) => Restored::Blocked(None),
        };
        Ok((store, restored))
    }

    fn read_marker(&self) -> Result<Option<Marker>, String> {
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags((OFlags::NOFOLLOW | OFlags::NONBLOCK).bits() as i32)
            .open(self.root.join("current.pb"))
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("marker is not a regular file".into());
        }
        let mut bytes = Vec::new();
        file.take(policy::MAX_MARKER_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > policy::MAX_MARKER_BYTES {
            return Err("marker too large".into());
        }
        Marker::decode(bytes.as_slice())
            .map(Some)
            .map_err(|e| e.to_string())
    }

    /// Durably replace the marker using a private temporary file and parent sync.
    pub fn write(&self, marker: &Marker) -> Result<(), String> {
        let bytes = marker.encode_to_vec();
        if bytes.len() as u64 > policy::MAX_MARKER_BYTES {
            return Err("marker too large".into());
        }
        let mut random = [0_u8; policy::RANDOM_ID_BYTES];
        getrandom::fill(&mut random).map_err(|e| e.to_string())?;
        let temp = self.root.join(format!(".current-{}", hex::encode(random)));
        let result = (|| -> Result<(), String> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(OFlags::NOFOLLOW.bits() as i32)
                .open(&temp)
                .map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            fs::rename(&temp, self.root.join("current.pb")).map_err(|e| e.to_string())?;
            File::open(&self.root)
                .and_then(|dir| dir.sync_all())
                .map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }
}
