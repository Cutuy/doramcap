//! Offline interpretation of the supported kernel OverlayFS upper format.
use super::{OverlayTree, Result, cstr, invalid};
use crate::filesystem_proto::{ChangeKind, FileChange, FileMetadata, StorageCategory};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, Metadata},
    os::unix::{
        ffi::OsStrExt,
        fs::{FileTypeExt, MetadataExt},
    },
    path::{Path, PathBuf},
};

/// A byte-safe, run-owned description of changes. Paths are relative to the source.
#[derive(Debug)]
pub(super) struct ApplyPlan {
    pub remove: BTreeSet<Vec<u8>>,
    pub transfer: BTreeSet<Vec<u8>>,
    pub warnings: Vec<String>,
}

fn xattr(path: &Path, name: &str) -> Result<Option<Vec<u8>>> {
    let path = cstr(path)?;
    let name = std::ffi::CString::new(name).map_err(|_| invalid("NUL in xattr name"))?;
    // SAFETY: the two C strings remain valid; null buffer asks for the value length.
    let size = unsafe { libc::lgetxattr(path.as_ptr(), name.as_ptr(), std::ptr::null_mut(), 0) };
    if size < 0 {
        let error = std::io::Error::last_os_error();
        return if error.raw_os_error() == Some(libc::ENODATA) {
            Ok(None)
        } else {
            Err(error.into())
        };
    }
    let mut value = vec![0u8; size as usize];
    // SAFETY: value is writable for its length, and the C strings remain valid.
    let read = unsafe {
        libc::lgetxattr(
            path.as_ptr(),
            name.as_ptr(),
            value.as_mut_ptr().cast(),
            value.len(),
        )
    };
    if read < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    value.truncate(read as usize);
    Ok(Some(value))
}

fn overlay_attr(path: &Path, suffix: &str) -> Result<Option<Vec<u8>>> {
    // Mounts created by this adapter do not use userxattr. user.overlay.*
    // therefore belongs to the application and must never become an action.
    xattr(path, &format!("trusted.overlay.{suffix}"))
}

fn lower_metadata(
    tree: &OverlayTree,
    path: &Path,
    reset: &BTreeSet<PathBuf>,
) -> Result<Option<Metadata>> {
    if reset
        .iter()
        .any(|parent| path != parent && path.starts_with(parent))
    {
        return Ok(None);
    }
    let mut current = tree.source.clone();
    let mut components = path.components().peekable();
    while let Some(part) = components.next() {
        current.push(part.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(meta) if components.peek().is_none() => return Ok(Some(meta)),
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Ok(None),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(Some(fs::symlink_metadata(&tree.source)?))
}

/// Read a stopped upper, mounted or unmounted, without reading file contents.
/// The callback receives metadata transitions; the returned actions drive write-back.
/// Memory follows the upper and deleted subtrees; unsupported overlay features fail explicitly.
pub(super) fn inspect(
    tree: &OverlayTree,
    mut emit: impl FnMut(FileChange) -> Result<()>,
) -> Result<ApplyPlan> {
    let mut deleted_paths = BTreeSet::new();
    let mut emit = |row: FileChange| {
        if row.kind() != ChangeKind::Deleted || deleted_paths.insert(row.path.clone()) {
            emit(row)?;
        }
        Ok(())
    };
    let mut plan = ApplyPlan {
        remove: BTreeSet::new(),
        transfer: BTreeSet::new(),
        warnings: Vec::new(),
    };
    let mut reset = BTreeSet::<PathBuf>::new();
    let mut hardlinks = BTreeMap::<(u64, u64), Vec<(PathBuf, bool)>>::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(path) = pending.pop() {
        let upper = tree.upper.join(&path);
        let meta = fs::symlink_metadata(&upper)?;
        for feature in ["redirect", "metacopy"] {
            if overlay_attr(&upper, feature)?.is_some() {
                return Err(invalid(format!(
                    "unsupported overlay {feature}: {}",
                    upper.display()
                )));
            }
        }
        let whiteout_attr = overlay_attr(&upper, "whiteout")?;
        let whiteout = (meta.file_type().is_char_device() && meta.rdev() == 0)
            || (meta.is_file() && meta.len() == 0 && whiteout_attr.is_some());
        if whiteout_attr.is_some() && !whiteout {
            return Err(invalid(format!(
                "invalid overlay whiteout: {}",
                upper.display()
            )));
        }
        let lower = lower_metadata(tree, &path, &reset)?;
        let bytes = path.as_os_str().as_bytes().to_vec();
        if whiteout {
            if lower.is_some() {
                plan.remove.insert(bytes);
                deleted(tree, &path, true, &mut emit)?;
            }
            continue;
        }
        let kind = meta.file_type();
        let before = lower
            .as_ref()
            .map(|m| metadata(&tree.source.join(&path), m))
            .transpose()?;
        let after = metadata(&upper, &meta)?;
        let changed = before.as_ref().is_none_or(|old| differs(old, &after));
        if changed {
            emit(FileChange {
                category: StorageCategory::HostBind as i32,
                source: tree.source.to_string_lossy().into(),
                consumers: Vec::new(),
                path: path.as_os_str().as_bytes().to_vec(),
                kind: if before.is_some() {
                    ChangeKind::Modified as i32
                } else {
                    ChangeKind::Added as i32
                },
                before,
                after: Some(after),
            })?;
        }
        let supported = kind.is_file() || kind.is_dir() || kind.is_symlink();
        if !supported {
            plan.warnings.push(format!(
                "{}: socket, FIFO or device skipped during write-back",
                tree.source.join(&path).display()
            ));
            continue;
        }
        if kind.is_file() {
            hardlinks
                .entry((meta.dev(), meta.ino()))
                .or_default()
                .push((
                    path.clone(),
                    lower.as_ref().is_some_and(|m| m.is_file() && m.nlink() > 1),
                ));
        }
        if let Some(old) = &lower
            && (old.mode() & libc::S_IFMT) != (meta.mode() & libc::S_IFMT)
        {
            if path.as_os_str().is_empty() {
                return Err(invalid("overlay root type changed"));
            }
            plan.remove.insert(bytes.clone());
            if old.is_dir() {
                deleted(tree, &path, false, &mut emit)?;
            }
            if kind.is_dir() {
                reset.insert(path.clone());
            }
        }
        if kind.is_dir() {
            let opaque = overlay_attr(&upper, "opaque")?;
            match opaque.as_deref() {
                None | Some(b"x") => {}
                Some(b"y") => {
                    if lower.as_ref().is_some_and(Metadata::is_dir) {
                        for entry in fs::read_dir(tree.source.join(&path))? {
                            let child = path.join(entry?.file_name());
                            plan.remove.insert(child.as_os_str().as_bytes().to_vec());
                            // Upper children replace matching lower names; only hidden paths are deletions.
                            deleted_hidden(tree, &child, &mut emit)?;
                        }
                    }
                }
                Some(_) => {
                    return Err(invalid(format!(
                        "unsupported opaque attribute: {}",
                        upper.display()
                    )));
                }
            }
            for entry in fs::read_dir(&upper)? {
                pending.push(path.join(entry?.file_name()));
            }
        }
        if !path.as_os_str().is_empty() {
            plan.transfer.insert(bytes);
        }
    }
    for names in hardlinks.into_values() {
        for (path, linked_lower) in &names {
            if *linked_lower {
                plan.warnings.push(format!(
                    "{}: copied during write-back; other hard-link names keep their old contents",
                    tree.source.join(path).display()
                ));
            } else if names.len() > 1 {
                plan.warnings.push(format!(
                    "{}: hard link copied as a separate file during write-back",
                    tree.source.join(path).display()
                ));
            }
        }
    }
    Ok(plan)
}

/// Convert stat data without following symlinks or opening payloads.
pub(crate) fn metadata(path: &Path, m: &Metadata) -> Result<FileMetadata> {
    Ok(FileMetadata {
        mode: m.mode(),
        size: m.size(),
        uid: m.uid(),
        gid: m.gid(),
        mtime_seconds: m.mtime(),
        mtime_nanoseconds: m.mtime_nsec(),
        ctime_seconds: m.ctime(),
        ctime_nanoseconds: m.ctime_nsec(),
        symlink: if m.file_type().is_symlink() {
            Some(fs::read_link(path)?.into_os_string().into_encoded_bytes())
        } else {
            None
        },
    })
}
fn differs(before: &FileMetadata, after: &FileMetadata) -> bool {
    before.mode != after.mode
        || before.uid != after.uid
        || before.gid != after.gid
        || before.mtime_seconds != after.mtime_seconds
        || before.mtime_nanoseconds != after.mtime_nanoseconds
        || (matches!(after.mode & libc::S_IFMT, libc::S_IFREG | libc::S_IFLNK)
            && before.size != after.size)
}
fn deleted(
    tree: &OverlayTree,
    path: &Path,
    include_root: bool,
    emit: &mut impl FnMut(FileChange) -> Result<()>,
) -> Result<()> {
    let mut pending = vec![(path.to_path_buf(), include_root)];
    while let Some((path, include)) = pending.pop() {
        let absolute = tree.source.join(&path);
        let meta = fs::symlink_metadata(&absolute)?;
        if meta.is_dir() {
            for child in fs::read_dir(&absolute)? {
                pending.push((path.join(child?.file_name()), true));
            }
        }
        if include {
            emit(FileChange {
                category: StorageCategory::HostBind as i32,
                source: tree.source.to_string_lossy().into(),
                consumers: Vec::new(),
                path: path.as_os_str().as_bytes().to_vec(),
                kind: ChangeKind::Deleted as i32,
                before: Some(metadata(&absolute, &meta)?),
                after: None,
            })?;
        }
    }
    Ok(())
}
fn deleted_hidden(
    tree: &OverlayTree,
    path: &Path,
    emit: &mut impl FnMut(FileChange) -> Result<()>,
) -> Result<()> {
    match fs::symlink_metadata(tree.upper.join(path)) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => deleted(tree, path, true, emit),
        Err(e) => Err(e.into()),
        Ok(meta) if meta.is_dir() && fs::symlink_metadata(tree.source.join(path))?.is_dir() => {
            for child in fs::read_dir(tree.source.join(path))? {
                deleted_hidden(tree, &path.join(child?.file_name()), emit)?;
            }
            Ok(())
        }
        // Replacements and whiteouts are handled when their upper entry is visited.
        Ok(_) => Ok(()),
    }
}
