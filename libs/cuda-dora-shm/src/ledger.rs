use eyre::{Result, ensure};
use std::collections::BTreeSet;

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Free,
    Writing,
    Published,
}
#[derive(Debug)]
struct Slot {
    generation: u64,
    phase: Phase,
    returned: BTreeSet<String>,
}
/// CPU-side single-producer accounting. No transport ACK is a completion proof.
pub(crate) struct Ledger {
    slots: Vec<Slot>,
    peers: BTreeSet<String>,
}
impl Ledger {
    pub fn new(count: usize, peers: &[String]) -> Self {
        Self {
            slots: (0..count)
                .map(|_| Slot {
                    generation: 0,
                    phase: Phase::Free,
                    returned: BTreeSet::new(),
                })
                .collect(),
            peers: peers.iter().cloned().collect(),
        }
    }
    pub fn reserve(&mut self) -> Result<Option<(usize, u64)>> {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.phase == Phase::Free {
                slot.generation = slot
                    .generation
                    .checked_add(1)
                    .ok_or_else(|| eyre::eyre!("generation exhausted"))?;
                slot.phase = Phase::Writing;
                slot.returned.clear();
                return Ok(Some((i, slot.generation)));
            }
        }
        Ok(None)
    }
    pub fn publish(&mut self, index: usize) {
        self.slots[index].phase = Phase::Published;
    }
    pub fn cancel(&mut self, index: usize) {
        // Never exposed to peers, so the unused generation may be reused.
        self.slots[index].generation -= 1;
        self.slots[index].phase = Phase::Free;
    }
    pub fn returned(&mut self, index: usize, generation: u64, peer: &str) -> Result<()> {
        ensure!(self.peers.contains(peer), "unknown peer");
        let slot = self
            .slots
            .get_mut(index)
            .ok_or_else(|| eyre::eyre!("invalid slot"))?;
        ensure!(
            slot.phase == Phase::Published && slot.generation == generation,
            "stale/unpublished generation"
        );
        ensure!(slot.returned.insert(peer.into()), "duplicate return");
        Ok(())
    }
    pub fn candidates(&self) -> Vec<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.phase == Phase::Published && s.returned == self.peers)
            .map(|(i, _)| i)
            .collect()
    }
    pub fn reclaim(&mut self, index: usize) {
        self.slots[index].phase = Phase::Free;
    }
    pub fn drained(&self) -> bool {
        self.slots.iter().all(|s| s.phase == Phase::Free)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_readers_and_completion_are_required_before_reuse() {
        // Arrange
        let mut ledger = Ledger::new(1, &["a".into(), "b".into()]);
        // Act
        assert_eq!(ledger.reserve().unwrap(), Some((0, 1)));
        ledger.publish(0);
        ledger.returned(0, 1, "a").unwrap();
        // Assert
        assert!(ledger.candidates().is_empty());
        assert_eq!(ledger.reserve().unwrap(), None);
        // Act
        ledger.returned(0, 1, "b").unwrap();
        // Assert: all returns alone still do not free the slot.
        assert_eq!(ledger.candidates(), vec![0]);
        assert_eq!(ledger.reserve().unwrap(), None);
        // Act: adapter supplies completion proof.
        ledger.reclaim(0);
        // Assert
        assert_eq!(ledger.reserve().unwrap(), Some((0, 2)));
    }
    #[test]
    fn rejects_invalid_returns_and_cancel_preserves_generation() {
        // Arrange
        let mut ledger = Ledger::new(1, &["a".into()]);
        ledger.reserve().unwrap();
        // Act / Assert
        assert!(ledger.returned(0, 1, "a").is_err());
        ledger.cancel(0);
        assert_eq!(ledger.reserve().unwrap(), Some((0, 1)));
        ledger.publish(0);
        assert!(ledger.returned(0, 0, "a").is_err());
        assert!(ledger.returned(1, 1, "a").is_err());
        assert!(ledger.returned(0, 1, "x").is_err());
        ledger.returned(0, 1, "a").unwrap();
        assert!(ledger.returned(0, 1, "a").is_err());
    }
    #[test]
    fn slow_reader_holds_only_its_generation_and_exhaustion_never_wraps() {
        // Arrange
        let mut ledger = Ledger::new(2, &["a".into(), "b".into()]);
        ledger.slots[0].generation = 41;
        ledger.reserve().unwrap();
        ledger.publish(0);
        ledger.reserve().unwrap();
        ledger.publish(1);
        // Act
        ledger.returned(0, 42, "a").unwrap();
        ledger.returned(1, 1, "b").unwrap();
        ledger.returned(1, 1, "a").unwrap();
        // Assert
        assert_eq!(ledger.candidates(), vec![1]);
        assert!(ledger.reserve().unwrap().is_none());
        // Act
        ledger.reclaim(1);
        let independent = ledger.reserve().unwrap();
        ledger.returned(0, 42, "b").unwrap();
        ledger.reclaim(0);
        let next_generation = ledger.reserve().unwrap();
        // Assert
        assert_eq!(independent, Some((1, 2)));
        assert_eq!(next_generation, Some((0, 43)));
        assert!(ledger.returned(0, 42, "a").is_err());
        // Arrange
        ledger.cancel(0);
        ledger.slots[0].generation = u64::MAX;
        // Act
        let exhausted = ledger.reserve();
        // Assert
        assert!(exhausted.is_err());
        assert_eq!(ledger.slots[0].generation, u64::MAX);
        assert_eq!(ledger.slots[0].phase, Phase::Free);
    }
}
