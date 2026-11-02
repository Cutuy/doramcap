//! Bounded cgroup identity state for publishers that send names out of band.
use crate::Snapshot;
use std::{
    collections::{HashMap, HashSet},
    fs,
    os::unix::fs::MetadataExt,
    path::Path,
    time::{Duration, Instant},
};

const CGROUP_ROOT: &str = "/sys/fs/cgroup";
const REFRESH: Duration = Duration::from_secs(2);

/// One cgroup identity; `path` stays `None` until discovery resolves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CgroupEntry {
    pub id: u64,
    pub parent_id: u64,
    pub path: Option<String>,
}

/// Tracks at most `capacity` identities seen in snapshots and their cgroupfs paths.
/// Single owner; `update` may read cgroupfs, so call it outside the collection loop.
pub struct Dictionary {
    entries: Vec<CgroupEntry>,
    index: HashMap<u64, usize>,
    unresolved: HashSet<u64>,
    capacity: usize,
    revision: u64,
    // Cumulative unresolved unique identities per update, not distinct identities
    // over the session. Never retain an unbounded overflow-ID set.
    overflow: u64,
    next_refresh: Instant,
}

impl Dictionary {
    /// Preallocates state for `capacity` identities.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            index: HashMap::with_capacity(capacity),
            unresolved: HashSet::with_capacity(capacity),
            capacity,
            revision: 0,
            overflow: 0,
            next_refresh: Instant::now(),
        }
    }

    /// Admits new identities from `snapshot` and refreshes paths at most every two seconds.
    /// Added identities and changed paths increment the revision; overflow counts the rest.
    pub fn update(&mut self, snapshot: &Snapshot) {
        let now = Instant::now();
        // Discovery is rate limited even if unresolved/new IDs appear in every
        // batch. It also revisits names so renames and temporary failures recover.
        let mut paths = HashMap::new();
        if now >= self.next_refresh {
            scan_cgroups(Path::new(CGROUP_ROOT), &mut paths, self.capacity, 0);
            self.next_refresh = now + REFRESH;
            for entry in &mut self.entries {
                if let Some(path) = paths.get(&entry.id)
                    && entry.path.as_ref() != Some(path)
                {
                    entry.path = Some(path.clone());
                    self.revision = self.revision.saturating_add(1);
                }
            }
        }
        self.unresolved.clear();
        for record in &snapshot.records {
            if self.index.contains_key(&record.cgroup_id) {
                continue;
            }
            if self.entries.len() == self.capacity {
                self.unresolved.insert(record.cgroup_id);
                continue;
            }
            let path = paths.remove(&record.cgroup_id);
            self.index.insert(record.cgroup_id, self.entries.len());
            self.entries.push(CgroupEntry {
                id: record.cgroup_id,
                parent_id: record.parent_id,
                path,
            });
            self.revision = self.revision.saturating_add(1);
        }
        self.overflow = self.overflow.saturating_add(self.unresolved.len() as u64);
    }

    pub fn entries(&self) -> &[CgroupEntry] {
        &self.entries
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn overflow(&self) -> u64 {
        self.overflow
    }
}

// Bound metadata strings and traversal depth as well as entry count. Deeper or
// longer paths remain unresolved (numeric records still carry identity).
fn scan_cgroups(path: &Path, paths: &mut HashMap<u64, String>, capacity: usize, depth: usize) {
    if paths.len() >= capacity || depth > 64 || path.as_os_str().len() > 4096 {
        return;
    }
    if let Ok(meta) = fs::metadata(path) {
        let relative = path.strip_prefix(CGROUP_ROOT).unwrap_or(path);
        let name = if relative.as_os_str().is_empty() {
            "/".into()
        } else {
            format!("/{}", relative.to_string_lossy())
        };
        paths.insert(meta.ino(), name);
    }
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if paths.len() >= capacity {
                break;
            }
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                scan_cgroups(&entry.path(), paths, capacity, depth + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CpuRecord;

    fn snapshot(ids: &[u64]) -> Snapshot {
        let mut snapshot = Snapshot::with_capacity(ids.len());
        for &cgroup_id in ids {
            assert!(snapshot.push(CpuRecord {
                cgroup_id,
                parent_id: 1,
                ..Default::default()
            }));
        }
        snapshot
    }

    #[test]
    fn new_identities_advance_revision_once_and_repeats_do_not() {
        // Arrange
        let mut dictionary = Dictionary::new(4);

        // Act
        dictionary.update(&snapshot(&[u64::MAX, u64::MAX - 1, u64::MAX]));
        let revision = dictionary.revision();
        dictionary.update(&snapshot(&[u64::MAX, u64::MAX - 1]));

        // Assert
        assert_eq!(revision, 2);
        assert_eq!(dictionary.revision(), 2);
        assert_eq!(dictionary.entries().len(), 2);
        assert_eq!(dictionary.entries()[0].id, u64::MAX);
        assert_eq!(dictionary.entries()[0].parent_id, 1);
        assert_eq!(dictionary.overflow(), 0);
    }

    #[test]
    fn capacity_bounds_entries_and_overflow_accumulates_per_update() {
        // Arrange
        let mut dictionary = Dictionary::new(1);

        // Act
        dictionary.update(&snapshot(&[u64::MAX, u64::MAX - 1, u64::MAX - 2]));
        dictionary.update(&snapshot(&[u64::MAX - 1]));

        // Assert
        assert_eq!(dictionary.entries().len(), 1);
        assert_eq!(dictionary.revision(), 1);
        assert_eq!(dictionary.overflow(), 3);
    }
}
