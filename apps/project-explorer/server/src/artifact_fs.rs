//! Descriptor-relative access to retained artifacts without following symlinks.
use rustix::fs::Dir;
use std::{fs::File, io};
pub(crate) use workload_docker::evidence::{open, root};

pub(crate) fn names(directory: &File) -> io::Result<Vec<String>> {
    let mut dir = Dir::read_from(directory)?;
    let mut names = Vec::new();
    let mut entries = 0;
    while let Some(entry) = dir.read() {
        let entry = entry?;
        entries += 1;
        if entries > 4098 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "artifact directory has too many entries",
            ));
        }
        let Ok(name) = entry.file_name().to_str() else {
            continue;
        };
        if name == "." || name == ".." {
            continue;
        }
        names.push(name.to_owned());
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_beneath_rejects_traversal_and_symlinks() {
        // Arrange
        let base =
            std::env::temp_dir().join(format!("gallery-path-{:032x}", rand::random::<u128>()));
        let results = base.join("results");
        std::fs::create_dir_all(results.join("run")).unwrap();
        std::fs::write(results.join("run/file"), b"inside").unwrap();
        std::fs::write(base.join("outside"), b"outside").unwrap();
        std::os::unix::fs::symlink(&base, results.join("escape")).unwrap();
        std::os::unix::fs::symlink(base.join("outside"), results.join("run/link")).unwrap();
        let root = root(&results).unwrap();

        // Act / Assert: ordinary files open; traversal and either symlink are rejected.
        assert!(open(&root, "run/file", false).is_ok());
        assert!(open(&root, "../outside", false).is_err());
        assert!(open(&root, "escape/outside", false).is_err());
        assert!(open(&root, "run/link", false).is_err());
        assert!(open(&root, "run", false).is_err());
        std::fs::remove_dir_all(base).unwrap();
    }
}
