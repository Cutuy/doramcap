//! Opt-in CAP_SYS_ADMIN tests; build normally, elevate only this executable.
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};
use workload_docker::compose::{overlay::*, *};
fn plan(root: &Path, source: &Path) -> ResolvedComposeProject {
    let manifest = root.join("original.json");
    let model = serde_json::json!({"services":{"a":{"image":"unused","volumes":[{"type":"bind","source":source,"target":"/work"},{"type":"bind","source":source.join("child"),"target":"/child","read_only":true}]},"b":{"image":"unused","volumes":[{"type":"bind","source":source,"target":"/shared"}]}}});
    let bytes = serde_json::to_vec(&model).unwrap();
    std::fs::write(&manifest, &bytes).unwrap();
    ResolvedComposeProject {
        run: "overlay-test".into(),
        key: "foreign".into(),
        name: "dc-overlay-test".into(),
        manifest,
        sha256: hex::encode(Sha256::digest(&bytes)),
        services: BTreeMap::new(),
        diagnostics: vec![],
        remove_volumes: false,
    }
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
        let original_bytes = std::fs::read(&original.manifest).unwrap();
        // Act: prepare shared views; writes through the same source have one upper tree.
        let set = prepare(PrepareInput {
            workspace: artifacts.join("views"),
            artifacts,
            projects: vec![original.clone()],
            policy: OverlayPolicy {
                disposition: disposition.clone(),
            },
        })
        .await
        .unwrap();
        assert_eq!(set.trees.len(), 1);
        let view = &set.trees[0].merged;
        // Assert: no unchanged payload copied up; original manifest untouched.
        assert!(!set.trees[0].upper.join("unchanged").exists());
        assert_eq!(std::fs::read(&original.manifest).unwrap(), original_bytes);
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
            projects: vec![plan(&artifacts, &source)],
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
        let error = finalize(set.clone()).await.unwrap_err();
        // Assert: no automatic write-back after conflict or busy unmount.
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
    let mut second = plan(&other, &invalid_source);
    second.key = "second".into();
    let _cleanup = MountCleanup(root.clone());
    // Act
    let error = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        projects: vec![first, second],
        artifacts: artifacts.clone(),
        policy: Default::default(),
    })
    .await
    .unwrap_err();
    // Assert: partial preparation unwinds its earlier successful mount.
    assert!(error.to_string().contains("option characters"));
    assert!(
        !std::fs::read_to_string("/proc/self/mountinfo")
            .unwrap()
            .contains(&format!("{}/overlays/", artifacts.display()))
    );
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
    let mut project = plan(&artifacts, &source);
    let model = serde_json::json!({"services":{"writer":{"image":"unused","volumes":[
        {"type":"bind","source":source.join("child"),"target":"/child"},
        {"type":"bind","source":source,"target":"/parent","read_only":true}]}}});
    let bytes = serde_json::to_vec(&model).unwrap();
    std::fs::write(&project.manifest, &bytes).unwrap();
    project.sha256 = hex::encode(Sha256::digest(&bytes));
    // Act
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        artifacts,
        projects: vec![project],
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
    let error = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        projects: vec![plan(&artifacts, &source)],
        artifacts: artifacts.clone(),
        policy: Default::default(),
    })
    .await
    .unwrap_err();
    // Assert
    assert!(error.to_string().contains("hardlink group escapes"));
    assert!(!artifacts.join("overlays").exists());
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
    let error = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        projects: vec![plan(&artifacts, &source)],
        artifacts: artifacts.clone(),
        policy: Default::default(),
    })
    .await
    .unwrap_err();
    // Assert
    assert!(error.to_string().contains("nested mount"), "{error}");
    assert!(!artifacts.join("overlays").exists());
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
        projects: vec![plan(&artifacts, &source)],
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
    assert!(!tree.baseline.parent().unwrap().join("stage.log").exists());
    use prost::Message;
    let action_bytes =
        std::fs::read(tree.baseline.parent().unwrap().join("apply-plan.pb")).unwrap();
    let mut action_stream = action_bytes.as_slice();
    let mut actions = Vec::new();
    while !action_stream.is_empty() {
        actions.push(
            workload_docker::filesystem_proto::FileAction::decode_length_delimited(
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
    let log = std::fs::read_to_string(tree.baseline.parent().unwrap().join("apply.log")).unwrap();
    assert!(log.contains("same-time"));
    assert!(!log.contains("unchanged"));
    assert!(tree.upper.join("same-time").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn hardlinks_fail_before_any_source_is_modified() {
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
        projects: vec![plan(&artifacts, &source)],
        artifacts,
        policy: Default::default(),
    })
    .await
    .unwrap();
    std::fs::write(set.trees[0].merged.join("a"), b"changed").unwrap();
    std::fs::write(set.trees[0].merged.join("ordinary"), b"new").unwrap();
    // Act
    let error = finalize(set.clone()).await.unwrap_err();
    // Assert: source remains unchanged after the mount is gone; upper is retained.
    assert!(error.to_string().contains("hardlink"), "{error}");
    assert_unmounted(&set);
    assert_eq!(std::fs::read(source.join("a")).unwrap(), b"before");
    assert_eq!(std::fs::read(source.join("b")).unwrap(), b"before");
    assert!(!source.join("ordinary").exists());
    assert!(set.trees[0].upper.join("ordinary").exists());
    assert!(!set.directory.join("results.json").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn newly_created_hardlink_is_rejected() {
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
        projects: vec![plan(&artifacts, &source)],
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
    let error = finalize(set.clone()).await.unwrap_err();
    // Assert
    assert!(error.to_string().contains("hardlink"), "{error}");
    assert_unmounted(&set);
    assert!(!source.join("alias").exists());
    assert_eq!(std::fs::read(source.join("a")).unwrap(), b"before");
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires CAP_SYS_ADMIN and rsync"]
async fn unsupported_upper_entries_reject_before_any_writeback() {
    // Arrange: each case uses real kernel OverlayFS and a valid change in a second tree.
    for feature in ["fifo", "redirect", "metacopy"] {
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
        let mut second = plan(&other, &root.join("b"));
        second.key = "second".into();
        let set = prepare(PrepareInput {
            workspace: artifacts.join("views"),
            projects: vec![first, second],
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
        projects: vec![plan(&artifacts, &source)],
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
            projects: vec![plan(&artifacts, &source)],
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
    // Arrange: a test-owned 4 MiB tmpfs lower and a larger upper on the host fs.
    let root = std::env::temp_dir().join(format!("overlay-enospc-{}", std::process::id()));
    std::fs::create_dir_all(root.join("source")).unwrap();
    let _cleanup = MountCleanup(root.clone());
    let source = root.join("source");
    let mountpoint = std::ffi::CString::new(source.as_os_str().as_encoded_bytes()).unwrap();
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
    let artifacts = root.join("run");
    std::fs::create_dir(&artifacts).unwrap();
    let set = prepare(PrepareInput {
        workspace: artifacts.join("views"),
        projects: vec![plan(&artifacts, &source)],
        artifacts,
        policy: Default::default(),
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
    assert!(
        set.trees[0]
            .baseline
            .parent()
            .unwrap()
            .join("apply.log")
            .exists()
    );
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
        projects: vec![plan(&artifacts, &source)],
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
        projects: vec![plan(&artifacts, &source)],
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
    let log =
        std::fs::read_to_string(set.trees[0].baseline.parent().unwrap().join("apply.log")).unwrap();
    assert!(log.contains("recording"));
    assert!(!log.contains("model"));
    assert!(
        log.contains("8,388,608") || log.contains("8388608"),
        "{log}"
    );
    std::fs::remove_dir_all(root).unwrap();
}
