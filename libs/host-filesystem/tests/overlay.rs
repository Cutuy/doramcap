//! Opt-in CAP_SYS_ADMIN tests; build normally, elevate only this executable.
use host_filesystem::filesystem_proto::DirectoryMount as BindMount;
use host_filesystem::overlay::*;
use std::{
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, PermissionsExt},
    },
    path::{Path, PathBuf},
};
async fn prepare(input: PrepareInput) -> host_filesystem::Result<OverlaySet> {
    host_filesystem::overlay::prepare(resolve(input).await?).await
}
fn plan(_root: &Path, source: &Path) -> Vec<BindMount> {
    vec![
        BindMount {
            consumer: "project/a".into(),
            source: source.to_string_lossy().into(),
            target: "/work".into(),
            ..Default::default()
        },
        BindMount {
            consumer: "project/a".into(),
            source: source.join("child").to_string_lossy().into(),
            target: "/child".into(),
            read_only: true,
        },
        BindMount {
            consumer: "project/b".into(),
            source: source.to_string_lossy().into(),
            target: "/shared".into(),
            ..Default::default()
        },
    ]
}
fn binds(projects: &[Vec<BindMount>]) -> Vec<BindMount> {
    projects.iter().flatten().cloned().collect()
}
fn assert_unmounted(set: &OverlaySet) {
    let mounts = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
    for tree in &set.trees {
        assert!(!mounts.contains(&format!(" {} ", tree.merged.display())));
    }
}
#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn serial_overlay_isolation_and_harvest() {
    // Arrange: unchanged lower file, hardlinks, a sparse file and nested/opaque directories.
    let root = std::env::temp_dir().join(format!("overlay-proof-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(source.join("child")).unwrap();
    std::fs::write(source.join("unchanged"), b"model").unwrap();
    std::fs::write(source.join("child/unchanged"), b"model").unwrap();
    std::fs::write(source.join("rename"), b"renamed").unwrap();
    std::fs::write(source.join("edit"), b"before").unwrap();
    std::fs::write(source.join("gone"), b"delete").unwrap();
    std::fs::create_dir(source.join("opaque")).unwrap();
    std::fs::write(source.join("opaque/old"), b"old").unwrap();
    std::fs::write(source.join("hard-a"), b"hard").unwrap();
    std::fs::hard_link(source.join("hard-a"), source.join("hard-b")).unwrap();
    for (i, disposition) in [
        Disposition::Discard,
        Disposition::WriteBack,
        Disposition::Discard,
    ]
    .into_iter()
    .enumerate()
    {
        let artifacts = root.join(format!("run-{i}"));
        std::fs::create_dir(&artifacts).unwrap();
        let original = plan(&artifacts, &source);
        // Act: prepare shared views; writes through the same source have one upper tree.
        let set = prepare(PrepareInput {
            workspace: artifacts.join("views"),
            artifacts,
            bind_mounts: binds(std::slice::from_ref(&original)),
            policy: OverlayPolicy {
                disposition: disposition.clone(),
            },
        })
        .await
        .unwrap();
        assert_eq!(set.trees.len(), 1);
        let view = &set.trees[0].merged;
        // Assert: no unchanged payload copied up.
        assert!(!set.trees[0].upper.join("unchanged").exists());
        assert_eq!(set.mappings[0].effective, set.mappings[2].effective);
        assert!(set.mappings[1].read_only);
        let prior = std::fs::read(source.join("edit")).unwrap();
        // Act
        std::fs::write(view.join("edit"), b"after!").unwrap();
        std::fs::write(view.join("new"), b"result").unwrap();
        std::fs::write(view.join("child/result"), b"child").unwrap();
        std::fs::set_permissions(view.join("new"), std::fs::Permissions::from_mode(0o640)).unwrap();
        let attr_path =
            std::ffi::CString::new(view.join("new").as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: path/name/value pointers and lengths remain valid for the syscall.
        assert_eq!(
            unsafe {
                libc::setxattr(
                    attr_path.as_ptr(),
                    c"user.test".as_ptr(),
                    b"value".as_ptr().cast(),
                    5,
                    0,
                )
            },
            0
        );
        if view.join("rename").exists() {
            std::fs::rename(view.join("rename"), view.join("renamed")).unwrap();
        }
        if view.join("gone").exists() {
            std::fs::remove_file(view.join("gone")).unwrap();
        }
        std::fs::remove_dir_all(view.join("opaque")).unwrap();
        std::fs::create_dir(view.join("opaque")).unwrap();
        std::fs::write(view.join("opaque/new"), b"new").unwrap();
        if disposition == Disposition::Discard {
            std::fs::write(view.join("hard-a"), b"linked").unwrap();
        }
        let sparse = std::fs::File::create(view.join("sparse")).unwrap();
        sparse.set_len(64 * 1024 * 1024).unwrap();
        drop(sparse);
        let link = view.join("link");
        if link.symlink_metadata().is_ok() {
            std::fs::remove_file(&link).unwrap();
        }
        std::os::unix::fs::symlink("edit", link).unwrap();
        // Assert: isolation exists BEFORE harvest, including reads through aliases.
        assert_eq!(std::fs::read(source.join("edit")).unwrap(), prior);
        assert_eq!(
            std::fs::read(set.mappings[2].effective.join("edit")).unwrap(),
            b"after!"
        );
        if i == 0 {
            assert!(!source.join("new").exists());
            assert!(source.join("gone").exists());
        }
        // Act
        let result = finalize(set.clone()).await.unwrap();
        // Assert
        assert_unmounted(&set);
        if disposition == Disposition::Discard {
            assert!(result.paths.is_empty());
            assert!(!set.trees[0].upper.exists());
            assert!(!set.trees[0].work.exists());
            assert_eq!(std::fs::read(source.join("edit")).unwrap(), prior);
            continue;
        }
        let dest = &result.paths[0];
        assert_eq!(std::fs::read(dest.join("edit")).unwrap(), b"after!");
        assert!(!dest.join("gone").exists());
        assert_eq!(std::fs::read(dest.join("renamed")).unwrap(), b"renamed");
        assert_eq!(
            std::fs::metadata(dest.join("new")).unwrap().mode() & 0o777,
            0o640
        );
        assert!(!dest.join("opaque/old").exists());
        assert_eq!(std::fs::read(dest.join("opaque/new")).unwrap(), b"new");
        assert_eq!(
            std::fs::read_link(dest.join("link")).unwrap(),
            PathBuf::from("edit")
        );
        assert_eq!(
            std::fs::metadata(dest.join("hard-a")).unwrap().ino(),
            std::fs::metadata(dest.join("hard-b")).unwrap().ino()
        );
        assert!(std::fs::metadata(dest.join("sparse")).unwrap().blocks() * 512 < 1024 * 1024);
        assert_eq!(std::fs::read(source.join("edit")).unwrap(), b"after!");
    }
    std::fs::remove_dir_all(root).unwrap();
}

struct MountCleanup(std::path::PathBuf);
impl Drop for MountCleanup {
    fn drop(&mut self) {
        for line in std::fs::read_to_string("/proc/self/mountinfo")
            .unwrap()
            .lines()
            .rev()
        {
            let p = line.split_whitespace().nth(4).unwrap();
            if Path::new(p).starts_with(&self.0) {
                assert!(
                    std::process::Command::new("umount")
                        .arg(p)
                        .status()
                        .unwrap()
                        .success()
                );
            }
        }
    }
}
#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn failures_preserve_lower_and_unwind_preparation() {
    // Arrange
    let root = std::env::temp_dir().join(format!("overlay-failures-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let source = root.join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(source.join("child")).unwrap();
    std::fs::write(source.join("file"), b"before").unwrap();
    for busy in [false, true] {
        let artifacts = root.join(if busy { "busy" } else { "conflict" });
        std::fs::create_dir(&artifacts).unwrap();
        let cleanup = MountCleanup(artifacts.clone());
        let set = prepare(PrepareInput {
            workspace: artifacts.join("views"),
            bind_mounts: binds(&[plan(&artifacts, &source)]),
            artifacts,
            policy: Default::default(),
        })
        .await
        .unwrap();
        std::fs::write(set.trees[0].merged.join("file"), b"isolated").unwrap();
        let held = if busy {
            Some(std::fs::File::open(&set.trees[0].merged).unwrap())
        } else {
            std::fs::write(source.join("file"), b"external").unwrap();
            None
        };
        // Act
        let result = finalize(set.clone()).await;
        if !busy {
            result.unwrap();
            assert_unmounted(&set);
            assert_eq!(std::fs::read(source.join("file")).unwrap(), b"isolated");
            drop(cleanup);
            std::fs::write(source.join("file"), b"before").unwrap();
            continue;
        }
        let error = result.unwrap_err();
        // Assert: busy unmount preserves the upper and host source.
        assert_eq!(
            std::fs::read(source.join("file")).unwrap(),
            if busy {
                b"before".as_slice()
            } else {
                b"external".as_slice()
            }
        );
        assert!(set.trees[0].upper.join("file").exists());
        assert!(!set.directory.join("results.json").exists());
        assert!(
            if busy {
                error.to_string().contains("busy")
            } else {
                error.to_string().contains("lower tree changed")
            },
            "{error}"
        );
        drop(held);
        drop(cleanup);
        assert_unmounted(&set);
        std::fs::write(source.join("file"), b"before").unwrap();
    }
    // Arrange: second source cannot be encoded in legacy OverlayFS mount options.
    let invalid_source = root.join("z,invalid");
    std::fs::create_dir(&invalid_source).unwrap();
    std::fs::create_dir(invalid_source.join("child")).unwrap();
    let artifacts = root.join("prepare-failure");
    std::fs::create_dir(&artifacts).unwrap();
    let first = plan(&artifacts, &source);
    let other = root.join("other");
    std::fs::create_dir(&other).unwrap();
    let second = plan(&other, &invalid_source);
    let _cleanup = MountCleanup(root.clone());
    // Act
    let error = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        bind_mounts: binds(&[first, second]),
        artifacts: artifacts.clone(),
        policy: Default::default(),
    })
    .await
    .unwrap_err();
    // Assert: partial preparation unwinds its earlier successful mount.
    assert!(error.to_string().contains("option characters"));
    let plan: OverlaySet =
        serde_json::from_slice(&std::fs::read(artifacts.join("overlays/plan.json")).unwrap())
            .unwrap();
    assert_eq!(plan.trees.len(), 2);
    assert_unmounted(&plan);
    std::fs::remove_dir_all(&root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn read_only_parent_alias_and_unsupported_sources() {
    // Arrange: a writable child and read-only parent must share one view.
    let root = std::env::temp_dir().join(format!("overlay-aliases-{}", std::process::id()));
    std::fs::create_dir_all(root.join("source/child")).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    let artifacts = root.join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let project = vec![
        BindMount {
            consumer: "project/writer".into(),
            source: source.join("child").to_string_lossy().into(),
            target: "/child".into(),
            ..Default::default()
        },
        BindMount {
            consumer: "project/writer".into(),
            source: source.to_string_lossy().into(),
            target: "/parent".into(),
            read_only: true,
        },
    ];
    // Act
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        artifacts,
        bind_mounts: binds(&[project]),
        policy: Default::default(),
    })
    .await
    .unwrap();
    std::fs::write(set.mappings[0].effective.join("result"), b"shared").unwrap();
    // Assert
    assert_eq!(set.trees.len(), 1);
    assert_eq!(set.trees[0].source, source);
    assert!(set.mappings[1].read_only);
    assert_eq!(
        std::fs::read(set.mappings[1].effective.join("child/result")).unwrap(),
        b"shared"
    );
    assert!(!source.join("child/result").exists());
    finalize(set).await.unwrap();

    // Arrange: a hardlink crossing the source boundary cannot retain alias semantics.
    std::fs::write(source.join("linked"), b"input").unwrap();
    std::fs::hard_link(source.join("linked"), root.join("external-link")).unwrap();
    let artifacts = root.join("reject-link");
    std::fs::create_dir(&artifacts).unwrap();
    // Act
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        bind_mounts: binds(&[plan(&artifacts, &source)]),
        artifacts: artifacts.clone(),
        policy: Default::default(),
    })
    .await
    .unwrap();
    // Assert: preparation accepts a link with an external name without inventorying the lower.
    finalize(set).await.unwrap();
    std::fs::remove_file(root.join("external-link")).unwrap();

    // Arrange: even a same-device nested bind mount must fail preflight.
    let nested = source.join("nested");
    let outside = root.join("outside");
    std::fs::create_dir(&nested).unwrap();
    std::fs::create_dir(&outside).unwrap();
    let src = std::ffi::CString::new(outside.as_os_str().as_encoded_bytes()).unwrap();
    let dst = std::ffi::CString::new(nested.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: live C strings; test-owned source and target; no mount data for bind.
    assert_eq!(
        unsafe {
            libc::mount(
                src.as_ptr(),
                dst.as_ptr(),
                std::ptr::null(),
                libc::MS_BIND,
                std::ptr::null(),
            )
        },
        0
    );
    let artifacts = root.join("reject-mount");
    std::fs::create_dir(&artifacts).unwrap();
    // Act
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        bind_mounts: binds(&[plan(&artifacts, &source)]),
        artifacts: artifacts.clone(),
        policy: Default::default(),
    })
    .await
    .unwrap();
    // Assert: the source remains directly mounted and its nested mount stays intact.
    assert!(set.trees.is_empty());
    assert!(set.mappings[0].reason.contains("contains mount point"));
    assert_eq!(set.mappings[0].effective, source);
    finalize(set).await.unwrap();
    // SAFETY: live path to this test's bind mount; strict unmount.
    assert_eq!(unsafe { libc::umount2(dst.as_ptr(), 0) }, 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn direct_writeback_preserves_files_symlinks_and_whiteouts() {
    use std::os::unix::ffi::OsStringExt;
    // Arrange: lower objects exercise replacement, whiteouts and unchanged payloads.
    let root = std::env::temp_dir().join(format!("overlay-direct-{}", std::process::id()));
    std::fs::create_dir_all(root.join("source/child")).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    std::fs::write(source.join("unchanged"), b"large model untouched").unwrap();
    std::fs::write(source.join("same-time"), b"old").unwrap();
    std::fs::write(source.join("file-to-dir"), b"file").unwrap();
    std::fs::create_dir(source.join("dir-to-file")).unwrap();
    std::fs::write(source.join("dir-to-file/old"), b"old").unwrap();
    std::fs::create_dir(source.join("opaque")).unwrap();
    std::fs::write(source.join("opaque/old"), b"old").unwrap();
    std::fs::create_dir(root.join("outside")).unwrap();
    std::fs::write(root.join("outside/untouched"), b"safe").unwrap();
    std::os::unix::fs::symlink(root.join("outside"), source.join("replaced-link")).unwrap();
    let artifacts = root.join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        bind_mounts: binds(&[plan(&artifacts, &source)]),
        artifacts,
        policy: Default::default(),
    })
    .await
    .unwrap();
    let tree = &set.trees[0];
    let view = &tree.merged;
    // Act: exercise the kernel overlay and its actual whiteout/opaque representation.
    let old_mtime = std::fs::metadata(source.join("same-time")).unwrap();
    std::fs::write(view.join("same-time"), b"new").unwrap();
    let changed =
        std::ffi::CString::new(view.join("same-time").as_os_str().as_encoded_bytes()).unwrap();
    let times = [libc::timespec {
        tv_sec: old_mtime.mtime(),
        tv_nsec: old_mtime.mtime_nsec(),
    }; 2];
    // SAFETY: valid pathname and two initialized timestamps.
    assert_eq!(
        unsafe { libc::utimensat(libc::AT_FDCWD, changed.as_ptr(), times.as_ptr(), 0) },
        0
    );
    std::fs::remove_file(view.join("file-to-dir")).unwrap();
    std::fs::create_dir(view.join("file-to-dir")).unwrap();
    std::fs::write(view.join("file-to-dir/new"), b"child").unwrap();
    std::fs::remove_dir_all(view.join("dir-to-file")).unwrap();
    std::fs::write(view.join("dir-to-file"), b"replacement").unwrap();
    std::fs::remove_dir_all(view.join("opaque")).unwrap();
    std::fs::create_dir(view.join("opaque")).unwrap();
    std::fs::write(view.join("opaque/new"), b"new").unwrap();
    std::fs::remove_file(view.join("replaced-link")).unwrap();
    std::fs::create_dir(view.join("replaced-link")).unwrap();
    std::fs::write(view.join("replaced-link/inside"), b"private").unwrap();
    std::os::unix::fs::symlink("/outside/target", view.join("link")).unwrap();
    let name = std::ffi::OsString::from_vec(b"non-utf8-\xff".to_vec());
    std::fs::write(view.join(&name), b"bytes").unwrap();
    // Assert: source remains unchanged until finalization.
    assert_eq!(std::fs::read(source.join("same-time")).unwrap(), b"old");
    assert!(source.join("opaque/old").exists());
    // Act
    finalize(set.clone()).await.unwrap();
    // Assert: result matches the supported visible view without staging payloads.
    assert_unmounted(&set);
    assert_eq!(std::fs::read(source.join("same-time")).unwrap(), b"new");
    let after = std::fs::metadata(source.join("same-time")).unwrap();
    assert_eq!(
        (after.mtime(), after.mtime_nsec()),
        (old_mtime.mtime(), old_mtime.mtime_nsec())
    );
    assert_eq!(
        (after.uid(), after.gid()),
        (old_mtime.uid(), old_mtime.gid())
    );
    assert_eq!(
        std::fs::read(source.join("file-to-dir/new")).unwrap(),
        b"child"
    );
    assert_eq!(
        std::fs::read(source.join("dir-to-file")).unwrap(),
        b"replacement"
    );
    assert!(!source.join("opaque/old").exists());
    assert_eq!(std::fs::read(source.join("opaque/new")).unwrap(), b"new");
    assert_eq!(
        std::fs::read_link(source.join("link")).unwrap(),
        PathBuf::from("/outside/target")
    );
    assert_eq!(std::fs::read(source.join(name)).unwrap(), b"bytes");
    assert_eq!(
        std::fs::read(root.join("outside/untouched")).unwrap(),
        b"safe"
    );
    assert_eq!(
        std::fs::read(source.join("replaced-link/inside")).unwrap(),
        b"private"
    );
    assert_eq!(
        std::fs::read(source.join("unchanged")).unwrap(),
        b"large model untouched"
    );
    assert!(!tree.records.as_path().join("stage.log").exists());
    use prost::Message;
    let action_bytes = std::fs::read(tree.records.as_path().join("apply-plan.pb")).unwrap();
    let mut action_stream = action_bytes.as_slice();
    let mut actions = Vec::new();
    while !action_stream.is_empty() {
        actions.push(
            host_filesystem::filesystem_proto::FileAction::decode_length_delimited(
                &mut action_stream,
            )
            .unwrap(),
        );
    }
    assert!(
        actions
            .iter()
            .any(|a| a.kind == 1 && a.path == b"file-to-dir")
    );
    assert!(
        actions
            .iter()
            .any(|a| a.kind == 2 && a.path == b"same-time")
    );
    assert!(
        actions
            .iter()
            .any(|a| a.kind == 2 && a.path == b"non-utf8-\xff")
    );
    let log = std::fs::read_to_string(tree.records.as_path().join("apply.log")).unwrap();
    assert!(log.contains("same-time"));
    assert!(!log.contains("unchanged"));
    assert!(tree.upper.join("same-time").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn modified_hardlink_warns_and_leaves_other_name_unchanged() {
    // Arrange: a real copied-up hardlink group and a second ordinary change.
    let root = std::env::temp_dir().join(format!("overlay-hardlinks-{}", std::process::id()));
    std::fs::create_dir_all(root.join("source/child")).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    std::fs::write(source.join("a"), b"before").unwrap();
    std::fs::hard_link(source.join("a"), source.join("b")).unwrap();
    let artifacts = root.join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        bind_mounts: binds(&[plan(&artifacts, &source)]),
        artifacts,
        policy: Default::default(),
    })
    .await
    .unwrap();
    std::fs::write(set.trees[0].merged.join("a"), b"changed").unwrap();
    std::fs::write(set.trees[0].merged.join("ordinary"), b"new").unwrap();
    // Act
    let result = finalize(set.clone()).await.unwrap();
    // Assert
    assert_unmounted(&set);
    assert_eq!(std::fs::read(source.join("a")).unwrap(), b"changed");
    assert_eq!(std::fs::read(source.join("b")).unwrap(), b"before");
    assert_eq!(std::fs::read(source.join("ordinary")).unwrap(), b"new");
    assert_eq!(
        result
            .warnings
            .iter()
            .filter(|warning| warning.contains("other hard-link names"))
            .count(),
        1
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn newly_created_hardlink_is_copied_as_separate_file() {
    // Arrange: hardlink added to a previously single-link lower file.
    let root = std::env::temp_dir().join(format!("overlay-new-hardlink-{}", std::process::id()));
    std::fs::create_dir_all(root.join("source/child")).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    std::fs::write(source.join("a"), b"before").unwrap();
    let artifacts = root.join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        bind_mounts: binds(&[plan(&artifacts, &source)]),
        artifacts,
        policy: Default::default(),
    })
    .await
    .unwrap();
    std::fs::hard_link(
        set.trees[0].merged.join("a"),
        set.trees[0].merged.join("alias"),
    )
    .unwrap();
    // Act
    let result = finalize(set.clone()).await.unwrap();
    // Assert
    assert_unmounted(&set);
    assert_eq!(std::fs::read(source.join("alias")).unwrap(), b"before");
    assert_ne!(
        std::fs::metadata(source.join("a")).unwrap().ino(),
        std::fs::metadata(source.join("alias")).unwrap().ino()
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("separate file"))
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn unsupported_upper_entries_reject_before_any_writeback() {
    // Arrange: each case uses real kernel OverlayFS and a valid change in a second tree.
    for feature in ["redirect", "metacopy"] {
        let root =
            std::env::temp_dir().join(format!("overlay-reject-{feature}-{}", std::process::id()));
        std::fs::create_dir_all(root.join("a/child")).unwrap();
        std::fs::create_dir_all(root.join("b/child")).unwrap();
        let _cleanup = MountCleanup(root.clone());
        let artifacts = root.join("run");
        let other = root.join("manifest-b");
        std::fs::create_dir(&artifacts).unwrap();
        std::fs::create_dir(&other).unwrap();
        let first = plan(&artifacts, &root.join("a"));
        let second = plan(&other, &root.join("b"));
        let set = prepare(PrepareInput {
            workspace: artifacts.join("views"),
            bind_mounts: binds(&[first, second]),
            artifacts,
            policy: Default::default(),
        })
        .await
        .unwrap();
        assert_eq!(set.trees.len(), 2);
        std::fs::write(set.trees[0].merged.join("ordinary"), b"must not apply").unwrap();
        let path = set.trees[1].merged.join("bad");
        if feature == "fifo" {
            let p = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
            // SAFETY: the path is a live C string inside the test overlay.
            assert_eq!(unsafe { libc::mkfifo(p.as_ptr(), 0o600) }, 0);
        } else {
            std::fs::write(&path, b"data").unwrap();
            // Install an unsupported marker in the actual upper after the writer exits.
            // The test does not access the overlay view again before strict unmount.
            let p = std::ffi::CString::new(
                set.trees[1]
                    .upper
                    .join("bad")
                    .as_os_str()
                    .as_encoded_bytes(),
            )
            .unwrap();
            let name = std::ffi::CString::new(format!("trusted.overlay.{feature}")).unwrap();
            // SAFETY: both C strings and the one-byte value remain valid.
            assert_eq!(
                unsafe { libc::setxattr(p.as_ptr(), name.as_ptr(), b"y".as_ptr().cast(), 1, 0) },
                0
            );
        }
        // Act
        let error = finalize(set.clone()).await.unwrap_err();
        // Assert: validation covers every tree before any apply.
        assert!(
            error.to_string().contains(if feature == "fifo" {
                "file type"
            } else {
                feature
            }),
            "{error}"
        );
        assert_unmounted(&set);
        assert!(!root.join("a/ordinary").exists());
        assert!(!root.join("b/bad").exists());
        assert!(set.trees[0].upper.join("ordinary").exists());
        assert!(!set.directory.join("results.json").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn xattr_whiteout_deletes_and_opaque_x_preserves_other_children() {
    // Arrange: actual upper files with the alternate whiteout and opaque=x markers.
    let root = std::env::temp_dir().join(format!("overlay-xwhiteout-{}", std::process::id()));
    std::fs::create_dir_all(root.join("source/child")).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    std::fs::write(source.join("child/gone"), b"delete").unwrap();
    std::fs::write(source.join("child/keep"), b"keep").unwrap();
    std::fs::write(source.join("child/ordinary"), b"old").unwrap();
    let artifacts = root.join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        bind_mounts: binds(&[plan(&artifacts, &source)]),
        artifacts,
        policy: Default::default(),
    })
    .await
    .unwrap();
    std::fs::write(set.trees[0].merged.join("child/gone"), b"").unwrap();
    std::fs::write(set.trees[0].merged.join("child/ordinary"), b"").unwrap();
    let upper = &set.trees[0].upper;
    let file =
        std::ffi::CString::new(upper.join("child/gone").as_os_str().as_encoded_bytes()).unwrap();
    let dir = std::ffi::CString::new(upper.join("child").as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: valid C strings and one-byte xattr values in this test's upper.
    assert_eq!(
        unsafe {
            libc::setxattr(
                file.as_ptr(),
                c"trusted.overlay.whiteout".as_ptr(),
                b"y".as_ptr().cast(),
                1,
                0,
            )
        },
        0
    );
    // SAFETY: valid C strings and one-byte xattr values in this test's upper.
    assert_eq!(
        unsafe {
            libc::setxattr(
                dir.as_ptr(),
                c"trusted.overlay.opaque".as_ptr(),
                b"x".as_ptr().cast(),
                1,
                0,
            )
        },
        0
    );
    let ordinary =
        std::ffi::CString::new(upper.join("child/ordinary").as_os_str().as_encoded_bytes())
            .unwrap();
    // SAFETY: valid C strings and one-byte xattr value on an ordinary upper file.
    assert_eq!(
        unsafe {
            libc::setxattr(
                ordinary.as_ptr(),
                c"user.overlay.whiteout".as_ptr(),
                b"y".as_ptr().cast(),
                1,
                0,
            )
        },
        0
    );
    // Act
    finalize(set.clone()).await.unwrap();
    // Assert: marker deletes only its named entry and does not clear the directory.
    assert_unmounted(&set);
    assert!(!source.join("child/gone").exists());
    assert_eq!(std::fs::read(source.join("child/keep")).unwrap(), b"keep");
    assert_eq!(std::fs::read(source.join("child/ordinary")).unwrap(), b"");
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn two_serial_writebacks_inherit_prior_results() {
    // Arrange
    let root = std::env::temp_dir().join(format!("overlay-serial-direct-{}", std::process::id()));
    std::fs::create_dir_all(root.join("source/child")).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    std::fs::write(source.join("state"), b"initial").unwrap();
    use std::os::unix::ffi::OsStringExt;
    let unusual = std::ffi::OsString::from_vec(b"input-\xff".to_vec());
    let target = std::ffi::OsString::from_vec(b"target-\xfe".to_vec());
    std::fs::write(source.join(&unusual), b"input").unwrap();
    std::os::unix::fs::symlink(&target, source.join("byte-link")).unwrap();
    for (index, before, after) in [
        (0, b"initial".as_slice(), b"first".as_slice()),
        (1, b"first".as_slice(), b"second".as_slice()),
    ] {
        let artifacts = root.join(format!("run-{index}"));
        std::fs::create_dir(&artifacts).unwrap();
        let set = prepare(PrepareInput {
            workspace: artifacts.join("views"),
            bind_mounts: binds(&[plan(&artifacts, &source)]),
            artifacts,
            policy: Default::default(),
        })
        .await
        .unwrap();
        // Act: the experiment sees the previous result and modifies its private view.
        assert_eq!(
            std::fs::read(set.trees[0].merged.join("state")).unwrap(),
            before
        );
        std::fs::write(set.trees[0].merged.join("state"), after).unwrap();
        // Assert: host remains unchanged until finalization.
        assert_eq!(std::fs::read(source.join("state")).unwrap(), before);
        finalize(set.clone()).await.unwrap();
        assert_unmounted(&set);
        assert_eq!(std::fs::read(source.join("state")).unwrap(), after);
        assert_eq!(std::fs::read(source.join(&unusual)).unwrap(), b"input");
        assert_eq!(
            std::fs::read_link(source.join("byte-link"))
                .unwrap()
                .into_os_string(),
            target
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn exhausted_host_filesystem_keeps_upper_and_reports_failure() {
    for disposition in [Disposition::WriteBack, Disposition::Keep] {
        exhausted_destination(disposition).await;
    }
}

async fn exhausted_destination(disposition: Disposition) {
    // Arrange: a test-owned 4 MiB tmpfs lower and a larger upper on the host fs.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();
    std::fs::create_dir_all(root.join("source")).unwrap();
    let artifacts = root.join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    let destination = if disposition == Disposition::Keep {
        &artifacts
    } else {
        &source
    };
    let mountpoint = std::ffi::CString::new(destination.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: live C strings, test-owned mountpoint and static tmpfs options.
    assert_eq!(
        unsafe {
            libc::mount(
                c"tmpfs".as_ptr(),
                mountpoint.as_ptr(),
                c"tmpfs".as_ptr(),
                0,
                c"size=4m".as_ptr().cast(),
            )
        },
        0
    );
    std::fs::create_dir(source.join("child")).unwrap();
    std::fs::write(source.join("existing"), b"before").unwrap();
    let set = prepare(PrepareInput {
        workspace: root.join("views"),
        bind_mounts: binds(&[plan(&artifacts, &source)]),
        artifacts,
        policy: OverlayPolicy { disposition },
    })
    .await
    .unwrap();
    std::fs::write(
        set.trees[0].merged.join("large"),
        vec![7u8; 8 * 1024 * 1024],
    )
    .unwrap();
    // Act: rsync cannot complete on the constrained destination filesystem.
    let error = finalize(set.clone()).await.unwrap_err();
    // Assert: no success is recorded, upper content remains, and the mount is gone.
    assert!(error.to_string().contains("rsync"), "{error}");
    assert_unmounted(&set);
    assert_eq!(
        std::fs::metadata(set.trees[0].upper.join("large"))
            .unwrap()
            .len(),
        8 * 1024 * 1024
    );
    assert!(!set.directory.join("results.json").exists());
    assert!(set.trees[0].records.join("apply.log").exists());
    // SAFETY: strict unmount of this test's own tmpfs after all handles close.
    assert_eq!(unsafe { libc::umount2(mountpoint.as_ptr(), 0) }, 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn opaque_upper_root_hides_lower_children_without_copying_internal_xattrs() {
    // Arrange: create a real mount and mark its upper root opaque after the writer exits.
    let root = std::env::temp_dir().join(format!("overlay-opaque-root-{}", std::process::id()));
    std::fs::create_dir_all(root.join("source/child")).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    std::fs::write(source.join("hidden"), b"old").unwrap();
    let artifacts = root.join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        bind_mounts: binds(&[plan(&artifacts, &source)]),
        artifacts,
        policy: Default::default(),
    })
    .await
    .unwrap();
    std::fs::write(set.trees[0].merged.join("visible"), b"new").unwrap();
    let upper = std::ffi::CString::new(set.trees[0].upper.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: valid test-owned upper root and one-byte attribute value.
    assert_eq!(
        unsafe {
            libc::setxattr(
                upper.as_ptr(),
                c"trusted.overlay.opaque".as_ptr(),
                b"y".as_ptr().cast(),
                1,
                0,
            )
        },
        0
    );
    // Act
    let result = finalize(set.clone()).await.unwrap();
    // Assert: hidden children are removed and internal overlay metadata is not copied.
    assert_unmounted(&set);
    assert!(!source.join("hidden").exists());
    assert_eq!(std::fs::read(source.join("visible")).unwrap(), b"new");
    assert_eq!(result.warnings.len(), 1);
    let target = std::ffi::CString::new(source.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: valid C strings and a null buffer to query the attribute's presence.
    assert_eq!(
        unsafe {
            libc::getxattr(
                target.as_ptr(),
                c"trusted.overlay.opaque".as_ptr(),
                std::ptr::null_mut(),
                0,
            )
        },
        -1
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ENODATA)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn direct_transfer_large_payload_without_staging_unchanged_data() {
    // Arrange: a large lower model and a fresh recording in the real overlay.
    let root = std::env::temp_dir().join(format!("overlay-large-{}", std::process::id()));
    std::fs::create_dir_all(root.join("source/child")).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    std::fs::write(source.join("model"), vec![3u8; 8 * 1024 * 1024]).unwrap();
    let artifacts = root.join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        bind_mounts: binds(&[plan(&artifacts, &source)]),
        artifacts,
        policy: Default::default(),
    })
    .await
    .unwrap();
    std::fs::write(
        set.trees[0].merged.join("recording"),
        vec![7u8; 8 * 1024 * 1024],
    )
    .unwrap();
    // Act
    finalize(set.clone()).await.unwrap();
    // Assert: only the recording is transferred, with no staged payload.
    assert_eq!(
        std::fs::read(source.join("recording")).unwrap(),
        vec![7u8; 8 * 1024 * 1024]
    );
    assert_eq!(
        std::fs::read(source.join("model")).unwrap(),
        vec![3u8; 8 * 1024 * 1024]
    );
    let log = std::fs::read_to_string(set.trees[0].records.as_path().join("apply.log")).unwrap();
    assert!(log.contains("recording"));
    assert!(!log.contains("model"));
    assert!(
        log.contains("8,388,608") || log.contains("8388608"),
        "{log}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "internal child for interrupted_copyback_discards_remaining_upper_data"]
async fn interrupted_copyback_child() {
    let Some(root) = std::env::var_os("DORAMCAP_OVERLAY_CRASH_ROOT") else {
        return;
    };
    // Arrange
    let root = PathBuf::from(root);
    let artifacts = root.join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let output = artifacts.join("output");
    std::fs::create_dir(&output).unwrap();
    let mut projects = Vec::new();
    for name in ["a", "b"] {
        let source = root.join(format!("source-{name}"));
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(source.join("child")).unwrap();
        std::fs::write(source.join("file"), b"before").unwrap();
        let input = root.join(format!("input-{name}"));
        std::fs::create_dir(&input).unwrap();
        let project = plan(&input, &source);
        projects.push(project);
    }
    projects.push(vec![BindMount {
        consumer: "app".into(),
        source: output.to_string_lossy().into(),
        target: "/output".into(),
        read_only: false,
    }]);
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        artifacts,
        bind_mounts: binds(&projects),
        policy: OverlayPolicy {
            disposition: if std::env::var_os("DORAMCAP_OVERLAY_KEEP").is_some() {
                Disposition::Keep
            } else {
                Disposition::WriteBack
            },
        },
    })
    .await
    .unwrap();
    for tree in &set.trees {
        std::fs::write(tree.merged.join("file"), b"after").unwrap();
    }
    let direct = set
        .mappings
        .iter()
        .find(|mapping| mapping.target == "/output")
        .unwrap();
    assert_eq!(direct.effective, output);
    std::fs::write(direct.effective.join("result"), b"survives interruption").unwrap();
    // Act: the test rsync wrapper pauses after applying the first tree.
    finalize(set).await.unwrap();
    panic!("copy-back should remain paused until the parent kills this child");
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn interrupted_copyback_discards_remaining_upper_data() {
    for keep in [false, true] {
        interrupted_copy(keep).await;
    }
}
async fn interrupted_copy(keep: bool) {
    use std::process::Command;
    // Arrange: pause real rsync after one of two host directories has changed.
    let root = tempfile::tempdir().unwrap();
    let _cleanup = MountCleanup(root.path().into());
    let bin = root.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let marker = root.path().join("paused");
    let wrapper = bin.join("rsync");
    std::fs::write(&wrapper, format!(
        "#!/bin/sh\n/usr/bin/rsync \"$@\" || exit \"$?\"\ncase \"$*\" in *changed.paths0*) printf '%s' \"$$\" > '{}'; kill -STOP \"$$\";; esac\n", marker.display())).unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    if keep {
        command.env("DORAMCAP_OVERLAY_KEEP", "1");
    }
    let mut child = command
        .args([
            "interrupted_copyback_child",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env("DORAMCAP_OVERLAY_CRASH_ROOT", root.path())
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !marker.exists()
        && child.try_wait().unwrap().is_none()
        && std::time::Instant::now() < deadline
    {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(marker.is_file(), "copy-back did not reach the pause");
    let wrapper_pid: i32 = std::fs::read_to_string(&marker).unwrap().parse().unwrap();
    // The orphaned stopped group may already have exited after its parent died.
    // SAFETY: this PID is the paused wrapper created by this isolated test.
    if unsafe { libc::kill(wrapper_pid, libc::SIGKILL) } != 0 {
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
    assert_eq!(
        std::fs::read(root.path().join("run/output/result")).unwrap(),
        b"survives interruption"
    );
    let first = root.path().join("source-a/file");
    let second = root.path().join("source-b/file");
    let expected: &[u8] = if keep { b"before" } else { b"after" };
    assert_eq!(std::fs::read(&first).unwrap(), expected);
    assert_eq!(std::fs::read(&second).unwrap(), b"before");
    if keep {
        assert_eq!(
            std::fs::read(root.path().join("run/kept/0/file")).unwrap(),
            b"after"
        );
    }
    let overlays = root.path().join("run/views");
    assert!(overlays.join("1/upper/file").is_file());
    // Act: reset the interrupted state twice through the pre-run discard primitive.
    for _ in 0..2 {
        discard(&overlays).await.unwrap();
        assert_eq!(
            std::fs::read(root.path().join("run/output/result")).unwrap(),
            b"survives interruption"
        );
        // Assert: copied changes remain; unapplied changes are discarded without replay.
        assert_eq!(std::fs::read(&first).unwrap(), expected);
        assert_eq!(std::fs::read(&second).unwrap(), b"before");
        for index in ["0", "1"] {
            assert!(!overlays.join(index).join("upper").exists());
            assert!(!overlays.join(index).join("work").exists());
        }
    }
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn upper_records_and_writeback_cover_real_kernel_entries() {
    use host_filesystem::filesystem_proto::{ChangeKind, StorageCategory};
    use std::os::unix::ffi::OsStringExt;
    // Arrange: a large unchanged directory and several lower transitions.
    let root = tempfile::tempdir().unwrap();
    let _cleanup = MountCleanup(root.path().into());
    let source = root.path().join("source");
    std::fs::create_dir_all(source.join("child")).unwrap();
    for i in 0..1500 {
        std::fs::write(source.join(format!("child/unchanged-{i}")), b"model").unwrap();
    }
    for name in ["child/edit", "opened", "permissions", "rename", "linked"] {
        std::fs::write(source.join(name), b"before").unwrap();
    }
    std::fs::hard_link(source.join("linked"), source.join("other-name")).unwrap();
    std::fs::create_dir_all(source.join("deleted/deep")).unwrap();
    std::fs::write(source.join("deleted/deep/file"), b"removed").unwrap();
    std::fs::create_dir_all(source.join("opaque/recreated")).unwrap();
    std::fs::write(source.join("opaque/recreated/hidden"), b"removed").unwrap();
    let artifacts = root.path().join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let mut mounts = plan(root.path(), &source);
    mounts.push(BindMount {
        consumer: "direct".into(),
        source: source.join("permissions").to_string_lossy().into(),
        target: "/file".into(),
        read_only: true,
    });
    let mut planned = resolve(PrepareInput {
        artifacts,
        workspace: root.path().join("views"),
        bind_mounts: mounts,
        policy: OverlayPolicy::default(),
    })
    .await
    .unwrap();
    planned
        .name_view(&source, source.file_name().unwrap())
        .unwrap();
    let set = host_filesystem::overlay::prepare(planned).await.unwrap();
    assert_eq!(
        set.mappings.last().unwrap().effective,
        set.trees[0].merged.join("permissions")
    );
    let tree = &set.trees[0];
    let view = &tree.merged;
    assert_eq!(view.file_name().unwrap(), source.file_name().unwrap());
    assert!(!tree.records.join("baseline.pb").exists());
    // Act: modify only an existing child so its parent's mtime remains unchanged.
    std::fs::write(view.join("child/edit"), b"changed").unwrap();
    drop(
        std::fs::OpenOptions::new()
            .write(true)
            .open(view.join("opened"))
            .unwrap(),
    );
    std::fs::set_permissions(
        view.join("permissions"),
        std::fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    std::fs::remove_dir_all(view.join("deleted")).unwrap();
    std::fs::rename(view.join("rename"), view.join("renamed")).unwrap();
    std::fs::write(view.join("linked"), b"run version").unwrap();
    std::fs::write(view.join("new-link"), b"new").unwrap();
    std::fs::hard_link(view.join("new-link"), view.join("new-alias")).unwrap();
    let unusual = std::ffi::OsString::from_vec(b"new-\xff".to_vec());
    std::fs::write(view.join(&unusual), b"new").unwrap();
    std::os::unix::fs::symlink(&unusual, view.join("symlink")).unwrap();
    let socket = std::os::unix::net::UnixListener::bind(view.join("socket")).unwrap();
    let fifo = std::ffi::CString::new(view.join("fifo").as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: live path in the test-owned overlay.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let device =
        std::ffi::CString::new(view.join("device").as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: test-owned char device, distinct from the OverlayFS 0:0 whiteout.
    assert_eq!(
        unsafe { libc::mknod(device.as_ptr(), libc::S_IFCHR | 0o600, libc::makedev(1, 3)) },
        0
    );
    drop(socket);
    std::fs::remove_dir_all(view.join("opaque")).unwrap();
    std::fs::create_dir_all(view.join("opaque/recreated")).unwrap();
    std::fs::write(view.join("opaque/recreated/new"), b"new").unwrap();
    std::fs::set_permissions(view, std::fs::Permissions::from_mode(0o750)).unwrap();
    // Assert: directory size differs but its copied-up metadata is otherwise unchanged.
    assert_ne!(
        std::fs::metadata(tree.upper.join("child")).unwrap().size(),
        std::fs::metadata(source.join("child")).unwrap().size()
    );
    let rows = changes(tree).await.unwrap();
    let find = |path: &str| rows.iter().find(|row| row.path == path.as_bytes()).unwrap();
    assert!(
        !rows
            .iter()
            .any(|row| row.path == b"child" || row.path == b"opened")
    );
    assert_eq!(find("child/edit").kind(), ChangeKind::Modified);
    assert_eq!(
        find("permissions").before.as_ref().unwrap().mode & 0o777,
        0o644
    );
    assert_eq!(
        find("permissions").after.as_ref().unwrap().mode & 0o777,
        0o640
    );
    for path in ["deleted", "deleted/deep", "deleted/deep/file", "rename"] {
        assert_eq!(find(path).kind(), ChangeKind::Deleted);
    }
    for path in [
        "renamed",
        "symlink",
        "socket",
        "fifo",
        "device",
        "new-link",
        "new-alias",
    ] {
        assert_eq!(find(path).kind(), ChangeKind::Added);
    }
    assert!(
        rows.iter()
            .any(|row| row.path == unusual.as_encoded_bytes())
    );
    assert!(
        rows.iter()
            .all(|row| row.category() == StorageCategory::HostBind)
    );
    assert_eq!(find("").after.as_ref().unwrap().mode & 0o777, 0o750);
    assert_eq!(find("opaque/recreated/hidden").kind(), ChangeKind::Deleted);
    let paths: std::collections::BTreeSet<_> = rows.iter().map(|row| &row.path).collect();
    assert_eq!(paths.len(), rows.len());
    // Act: reading needs no mounted view, and outside changes lose to the run's version.
    unmount(view).await.unwrap();
    let unmounted = changes(tree).await.unwrap();
    assert_eq!(rows.len(), unmounted.len());
    std::fs::write(source.join("linked"), b"external version").unwrap();
    // Re-mounting is unnecessary for finalization; finalize accepts already unmounted views.
    let result = finalize(set.clone()).await.unwrap();
    // Assert
    assert_eq!(
        std::fs::read(source.join("linked")).unwrap(),
        b"run version"
    );
    assert_eq!(
        std::fs::read(source.join("other-name")).unwrap(),
        b"external version"
    );
    assert_eq!(
        result
            .warnings
            .iter()
            .filter(|warning| warning.contains("skipped during copy"))
            .count(),
        3
    );
    assert_eq!(
        result
            .warnings
            .iter()
            .filter(|warning| warning.contains("separate file"))
            .count(),
        2
    );
    assert_eq!(
        result
            .warnings
            .iter()
            .filter(|warning| warning.contains("other hard-link names"))
            .count(),
        1
    );
    for path in ["socket", "fifo", "device"] {
        assert!(source.join(path).symlink_metadata().is_err());
    }
    assert_eq!(std::fs::metadata(&source).unwrap().mode() & 0o777, 0o750);
    assert_eq!(
        std::fs::read(source.join("opaque/recreated/new")).unwrap(),
        b"new"
    );
    assert!(!source.join("opaque/recreated/hidden").exists());
    assert!(!source.join("deleted").exists());
    assert_ne!(
        std::fs::metadata(source.join("new-link")).unwrap().ino(),
        std::fs::metadata(source.join("new-alias")).unwrap().ino()
    );
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn hugetlbfs_and_shm_aliases_outside_dev_keep_native_sharing_and_coverage() {
    use host_filesystem::changes::{self, CollectInput, SourcesInput};
    // Arrange: filesystem identity, rather than a /dev prefix, determines live shared memory.
    let root = tempfile::tempdir().unwrap();
    let _cleanup = MountCleanup(root.path().into());
    for backing in ["hugetlbfs", "shm-alias"] {
        let source = root.path().join(backing);
        std::fs::create_dir(&source).unwrap();
        let target = std::ffi::CString::new(source.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: live C strings and a fresh test-owned mountpoint; no persistent host configuration.
        let status = unsafe {
            if backing == "hugetlbfs" {
                libc::mount(
                    std::ptr::null(),
                    target.as_ptr(),
                    c"hugetlbfs".as_ptr(),
                    0,
                    std::ptr::null(),
                )
            } else {
                libc::mount(
                    c"/dev/shm".as_ptr(),
                    target.as_ptr(),
                    std::ptr::null(),
                    libc::MS_BIND,
                    std::ptr::null(),
                )
            }
        };
        assert_eq!(status, 0, "{}", std::io::Error::last_os_error());
        let artifacts = root.path().join(format!("run-{backing}"));
        std::fs::create_dir(&artifacts).unwrap();
        let binds = vec![BindMount {
            source: source.to_string_lossy().into(),
            consumer: "app".into(),
            target: "/memory".into(),
            read_only: false,
        }];
        // Act
        let notes = changes::coverage_notes(&binds, &[], &artifacts, true)
            .await
            .unwrap();
        let sources = changes::prepare_sources(SourcesInput {
            overlays: true,
            artifacts: artifacts.clone(),
            bind_mounts: binds.clone(),
            metadata_sources: vec![],
            excluded: vec![],
        })
        .await
        .unwrap();
        let set = prepare(PrepareInput {
            artifacts,
            workspace: root.path().join(format!("views-{backing}")),
            bind_mounts: binds,
            policy: Default::default(),
        })
        .await
        .unwrap();
        // Creating an empty hugetlbfs segment allocates no huge pages.
        let segment = format!("pr326-segment-{}", std::process::id());
        std::fs::write(set.mappings[0].effective.join(&segment), []).unwrap();
        let result = changes::collect(CollectInput {
            sources,
            overlays: Some(set.clone()),
            container_layers: None,
        })
        .await
        .unwrap();
        finalize(set.clone()).await.unwrap();
        // Assert
        assert!(set.trees.is_empty());
        assert_eq!(set.mappings[0].effective, source);
        assert!(source.join(&segment).exists());
        assert!(notes.is_empty());
        assert!(result.host_binds.coverage_complete);
        assert!(result.host_binds.warnings.is_empty());
        std::fs::remove_file(source.join(segment)).unwrap();
        unmount(&source).await.unwrap();
    }
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn writable_parent_and_child_use_one_view_and_write_back_once() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let _cleanup = MountCleanup(root.path().into());
    let source = root.path().join("source");
    std::fs::create_dir_all(source.join("child")).unwrap();
    std::fs::write(source.join("child/file"), b"before").unwrap();
    let artifacts = root.path().join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let mut bind_mounts = plan(root.path(), &source);
    bind_mounts[1].read_only = false;
    // Act
    let set = prepare(PrepareInput {
        artifacts,
        workspace: root.path().join("views"),
        bind_mounts,
        policy: Default::default(),
    })
    .await
    .unwrap();
    std::fs::write(set.mappings[1].effective.join("file"), b"child").unwrap();
    // Assert
    assert_eq!(set.trees.len(), 1);
    assert_eq!(
        std::fs::read(set.mappings[0].effective.join("child/file")).unwrap(),
        b"child"
    );
    assert_eq!(std::fs::read(source.join("child/file")).unwrap(), b"before");
    // Act
    std::fs::write(set.mappings[0].effective.join("child/file"), b"parent").unwrap();
    // Assert
    assert_eq!(
        std::fs::read(set.mappings[1].effective.join("file")).unwrap(),
        b"parent"
    );
    let result = finalize(set).await.unwrap();
    assert_eq!(result.paths.as_slice(), std::slice::from_ref(&source));
    assert_eq!(std::fs::read(source.join("child/file")).unwrap(), b"parent");
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn keep_runs_retain_files_and_leave_host_and_output_files_unchanged() {
    use host_filesystem::changes::{self, CollectInput, SourcesInput};
    use host_filesystem::filesystem_proto::ChangeKind;
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let _cleanup = MountCleanup(root.path().into());
    let source = root.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("edit"), b"before").unwrap();
    std::fs::write(source.join("gone"), b"original").unwrap();
    let cross_fs = tempfile::tempdir_in("/dev/shm").unwrap();
    for index in 0..2 {
        let artifacts = if index == 0 {
            root.path()
        } else {
            cross_fs.path()
        }
        .join(format!("run-{index}"));
        std::fs::create_dir(&artifacts).unwrap();
        let output = artifacts.join("output");
        std::fs::create_dir(&output).unwrap();
        let bind_mounts = vec![
            BindMount {
                source: source.to_str().unwrap().into(),
                target: "/work".into(),
                ..Default::default()
            },
            BindMount {
                source: output.to_str().unwrap().into(),
                target: "/output".into(),
                ..Default::default()
            },
        ];
        let sources = changes::prepare_sources(SourcesInput {
            artifacts: artifacts.clone(),
            overlays: true,
            bind_mounts: bind_mounts.clone(),
            metadata_sources: vec![],
            excluded: vec![],
        })
        .await
        .unwrap();
        let set = prepare(PrepareInput {
            artifacts: artifacts.clone(),
            workspace: root.path().join(format!("views-{index}")),
            bind_mounts,
            policy: OverlayPolicy {
                disposition: Disposition::Keep,
            },
        })
        .await
        .unwrap();
        let tree = &set.trees[0];
        assert_eq!(std::fs::read(tree.merged.join("edit")).unwrap(), b"before");
        // Act
        std::fs::write(tree.merged.join("edit"), b"after").unwrap();
        std::fs::write(tree.merged.join("new"), b"kept").unwrap();
        std::fs::create_dir(tree.merged.join("nested")).unwrap();
        std::fs::write(tree.merged.join("nested/file"), b"nested").unwrap();
        std::os::unix::fs::symlink("nested/file", tree.merged.join("link")).unwrap();
        std::fs::remove_file(tree.merged.join("gone")).unwrap();
        std::fs::write(output.join("result"), b"output").unwrap();
        let rows = host_filesystem::overlay::changes(tree).await.unwrap();
        changes::collect(CollectInput {
            sources,
            overlays: Some(set.clone()),
            container_layers: None,
        })
        .await
        .unwrap();
        use prost::Message;
        let report = host_filesystem::filesystem_proto::FileChangeReport::decode(
            std::fs::read(artifacts.join("filesystem/report.pb"))
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        assert_eq!(
            report.write_policy,
            host_filesystem::filesystem_proto::WritePolicy::Keep as i32
        );
        assert_eq!(
            report.overlay_sources[0].source,
            source.as_os_str().as_bytes()
        );
        let result = finalize(set.clone()).await.unwrap();
        // Assert
        assert_eq!(result.paths, vec![artifacts.join("kept/0")]);
        assert_eq!(
            std::fs::read(artifacts.join("kept/0/edit")).unwrap(),
            b"after"
        );
        assert_eq!(
            std::fs::read(artifacts.join("kept/0/new")).unwrap(),
            b"kept"
        );
        assert!(!artifacts.join("kept/0/gone").exists());
        assert_eq!(
            std::fs::read(artifacts.join("kept/0/nested/file")).unwrap(),
            b"nested"
        );
        assert_eq!(
            std::fs::read_link(artifacts.join("kept/0/link")).unwrap(),
            PathBuf::from("nested/file")
        );
        assert!(
            rows.iter()
                .any(|row| row.path == b"gone" && row.kind() == ChangeKind::Deleted)
        );
        assert_eq!(std::fs::read(source.join("edit")).unwrap(), b"before");
        assert_eq!(std::fs::read(source.join("gone")).unwrap(), b"original");
        assert!(!source.join("new").exists());
        assert_eq!(std::fs::read(output.join("result")).unwrap(), b"output");
        assert!(!set.workspace.exists());
    }
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn upper_records_match_walk_reference_and_measure_unchanged_tree_listing() {
    use host_filesystem::{
        changes::MetadataWalk,
        filesystem_proto::{ChangeKind, FileMetadata},
    };
    use std::collections::BTreeMap;
    async fn inventory(path: &Path) -> BTreeMap<PathBuf, FileMetadata> {
        let mut walk = MetadataWalk::open(path).await.unwrap();
        let mut entries = BTreeMap::new();
        while let Some((path, metadata)) = walk.next_entry().await.unwrap() {
            entries.insert(path, metadata);
        }
        entries
    }
    // Arrange: an independent before/after walk supplies the reference for ordinary transitions.
    let root = tempfile::tempdir().unwrap();
    let _cleanup = MountCleanup(root.path().into());
    let source = root.path().join("source");
    std::fs::create_dir_all(source.join("child")).unwrap();
    for index in 0..20_000 {
        std::fs::write(source.join(format!("child/model-{index}")), b"unchanged").unwrap();
    }
    for name in ["edit", "gone", "rename", "mode", "opened"] {
        std::fs::write(source.join(name), b"before").unwrap();
    }
    let before = inventory(&source).await;
    let artifacts = root.path().join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let start = std::time::Instant::now();
    let set = prepare(PrepareInput {
        artifacts,
        workspace: root.path().join("views"),
        bind_mounts: plan(root.path(), &source),
        policy: OverlayPolicy {
            disposition: Disposition::Discard,
        },
    })
    .await
    .unwrap();
    let preparation = start.elapsed();
    let tree = &set.trees[0];
    let start = std::time::Instant::now();
    let untouched = changes(tree).await.unwrap();
    assert!(
        untouched.iter().all(|row| row.path.is_empty()),
        "{untouched:#?}"
    );
    let unchanged_listing = start.elapsed();
    // Act
    std::fs::write(tree.merged.join("edit"), b"after-longer").unwrap();
    std::fs::write(tree.merged.join("added"), b"new").unwrap();
    std::fs::remove_file(tree.merged.join("gone")).unwrap();
    std::fs::rename(tree.merged.join("rename"), tree.merged.join("renamed")).unwrap();
    std::fs::set_permissions(
        tree.merged.join("mode"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    drop(
        std::fs::OpenOptions::new()
            .write(true)
            .open(tree.merged.join("opened"))
            .unwrap(),
    );
    let after = inventory(&tree.merged).await;
    let mut expected = BTreeMap::new();
    for (path, metadata) in &before {
        match after.get(path) {
            None => {
                expected.insert(path.clone(), ChangeKind::Deleted);
            }
            // Walk comparison retains the milestone's documented metadata fields, excluding ctime and directory size.
            Some(other)
                if metadata.mode != other.mode
                    || metadata.uid != other.uid
                    || metadata.gid != other.gid
                    || metadata.mtime_seconds != other.mtime_seconds
                    || metadata.mtime_nanoseconds != other.mtime_nanoseconds
                    || (metadata.mode & libc::S_IFMT != libc::S_IFDIR
                        && metadata.size != other.size) =>
            {
                expected.insert(path.clone(), ChangeKind::Modified);
            }
            _ => {}
        }
    }
    for path in after.keys().filter(|path| !before.contains_key(*path)) {
        expected.insert(path.clone(), ChangeKind::Added);
    }
    let start = std::time::Instant::now();
    let rows = changes(tree).await.unwrap();
    let listing = start.elapsed();
    let actual: BTreeMap<_, _> = rows
        .into_iter()
        .map(|row| {
            (
                PathBuf::from(std::ffi::OsStr::from_bytes(&row.path)),
                row.kind(),
            )
        })
        .collect();
    // Assert: every reference path and kind agrees; the measurement includes all upper inspection.
    assert_eq!(actual, expected);
    println!(
        "20,000 unchanged files: preparation={preparation:?}, unchanged-tree listing={unchanged_listing:?}, changed-upper listing={listing:?}, records={}",
        actual.len()
    );
    finalize(set).await.unwrap();
}
