use eyre::{Result, ensure};
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 3;
pub const MAX_MESSAGE: usize = 256 * 1024;

/// Fixed membership and bounded storage, negotiated before any publication.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub pool: String,
    pub boot: String,
    pub gpu: Vec<u8>,
    pub directory: String,
    pub slots: usize,
    pub capacity: usize,
    /// Slot distance in bytes, including alignment padding.
    pub stride: usize,
    pub slot_alignment: usize,
    pub peers: Vec<String>,
    pub ready: Vec<Vec<u8>>,
}
impl Manifest {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.version == VERSION, "unsupported protocol version");
        ensure!(
            !self.pool.is_empty() && self.pool.len() <= 128,
            "invalid pool id"
        );
        ensure!((1..=64).contains(&self.slots), "slots must be 1..=64");
        ensure!(
            (1..=64 * 1024 * 1024).contains(&self.capacity),
            "capacity must be 1..=64 MiB"
        );
        ensure!(
            !self.peers.is_empty() && self.peers.len() <= 64,
            "peers must be 1..=64"
        );
        ensure!(
            self.stride == crate::layout::slot_stride(self.capacity, self.slot_alignment)?,
            "invalid slot stride"
        );
        let mut names = std::collections::BTreeSet::new();
        for p in &self.peers {
            ensure!(
                !p.is_empty() && p.len() <= 128 && names.insert(p),
                "invalid/duplicate peer"
            );
        }
        ensure!(self.gpu.len() == 16, "invalid GPU UUID");
        ensure!(
            self.ready.len() == self.slots && self.ready.iter().all(|h| h.len() == 64),
            "invalid Ready event handles"
        );
        Ok(())
    }
}

/// No payload bytes or CUDA IPC memory handles appear on the wire.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Message {
    Manifest(Manifest),
    Attached {
        pool: String,
        peer: String,
        done: Vec<Vec<u8>>,
    },
    Frame {
        pool: String,
        slot: usize,
        generation: u64,
        len: usize,
        sequence: u64,
    },
    Returned {
        pool: String,
        peer: String,
        slot: usize,
        generation: u64,
    },
    Shutdown {
        pool: String,
    },
    Closed {
        pool: String,
        peer: String,
    },
}
impl Message {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(self)?;
        ensure!(bytes.len() <= MAX_MESSAGE, "control message too large");
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= MAX_MESSAGE, "control message too large");
        Ok(serde_json::from_slice(bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest {
            version: VERSION,
            pool: "pool".into(),
            boot: "boot".into(),
            gpu: vec![1; 16],
            directory: "/dev/shm/pool".into(),
            slots: 2,
            capacity: 4096,
            stride: 4096,
            slot_alignment: crate::DEFAULT_SLOT_ALIGNMENT,
            peers: vec!["a".into(), "b".into()],
            ready: vec![vec![0; 64]; 2],
        }
    }

    #[test]
    fn manifest_rejects_invalid_resources_and_version() {
        // Arrange
        let valid = manifest();
        let mutations: [fn(&mut Manifest); 10] = [
            |m| m.version += 1,
            |m| m.slot_alignment = 0,
            |m| m.stride += 1,
            |m| m.slots = 0,
            |m| m.capacity = 0,
            |m| m.capacity = 64 * 1024 * 1024 + 1,
            |m| m.peers.push("a".into()),
            |m| m.gpu.clear(),
            |m| m.ready.pop().map(drop).unwrap(),
            |m| m.ready[0].clear(),
        ];
        // Act / Assert
        assert!(valid.validate().is_ok());
        for mutate in mutations {
            let mut invalid = valid.clone();
            mutate(&mut invalid);
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn every_message_round_trips_and_unknown_fields_fail() {
        // Arrange
        let messages = [
            Message::Manifest(manifest()),
            Message::Attached {
                pool: "p".into(),
                peer: "a".into(),
                done: vec![vec![0; 64]],
            },
            Message::Frame {
                pool: "p".into(),
                slot: 0,
                generation: 42,
                len: 10,
                sequence: 0,
            },
            Message::Returned {
                pool: "p".into(),
                peer: "a".into(),
                slot: 0,
                generation: 42,
            },
            Message::Shutdown { pool: "p".into() },
            Message::Closed {
                pool: "p".into(),
                peer: "a".into(),
            },
        ];
        for message in messages {
            // Act
            let bytes = message.encode().unwrap();
            let round_trip = Message::decode(&bytes).unwrap();
            let mut extra: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            extra["unexpected"] = true.into();
            // Assert
            assert_eq!(message, round_trip);
            assert!(Message::decode(&serde_json::to_vec(&extra).unwrap()).is_err());
        }
    }
}
