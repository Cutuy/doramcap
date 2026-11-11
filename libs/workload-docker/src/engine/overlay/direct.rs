//! Offline interpretation of the supported kernel OverlayFS upper format.
use super::{OverlayTree, Result, cstr, invalid};
use std::{
    collections::BTreeSet,
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
            Ok(_) => {
                return Err(invalid(format!(
                    "unsafe destination ancestor: {}",
                    current.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(Some(fs::symlink_metadata(&tree.source)?))
}

/// Inspect all upper entries after unmount; no host mutation or payload reads.
/// Reject unsupported features before returning an action list for application.
pub(super) fn inspect(tree: &OverlayTree) -> Result<ApplyPlan> {
    let mut plan = ApplyPlan {
        remove: BTreeSet::new(),
        transfer: BTreeSet::new(),
    };
    let mut reset = BTreeSet::<PathBuf>::new();
    let mut upper_inodes = BTreeSet::<(u64, u64)>::new();
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
            }
            continue;
        }
        let kind = meta.file_type();
        if !(kind.is_file() || kind.is_dir() || kind.is_symlink()) {
            return Err(invalid(format!(
                "unsupported upper file type: {}",
                upper.display()
            )));
        }
        if kind.is_file() {
            if !upper_inodes.insert((meta.dev(), meta.ino())) {
                return Err(invalid(format!(
                    "unsupported hardlink at {}",
                    upper.display()
                )));
            }
            // index=on may add one internal workdir hardlink for a copied-up inode.
            // Without an origin, every extra link is a user-visible ambiguity.
            let origin = overlay_attr(&upper, "origin")?.is_some();
            let allowed = if origin { 2 } else { 1 };
            if meta.nlink() > allowed
                || lower.as_ref().is_some_and(|m| m.is_file() && m.nlink() > 1)
            {
                return Err(invalid(format!(
                    "unsupported hardlink at {}",
                    upper.display()
                )));
            }
        }
        if let Some(old) = &lower
            && (old.mode() & libc::S_IFMT) != (meta.mode() & libc::S_IFMT)
        {
            if path.as_os_str().is_empty() {
                return Err(invalid("overlay root type changed"));
            }
            plan.remove.insert(bytes.clone());
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
                            plan.remove.insert(
                                path.join(entry?.file_name())
                                    .as_os_str()
                                    .as_bytes()
                                    .to_vec(),
                            );
                        }
                    }
                    reset.insert(path.clone());
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
    Ok(plan)
}
