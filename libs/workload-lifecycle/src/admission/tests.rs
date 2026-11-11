use super::*;
use std::process::{Command, Stdio};

struct Repository(PathBuf);
impl Repository {
    fn new() -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "doramcap-admission-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root.canonicalize().unwrap())
    }
}
impl Drop for Repository {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn aliases_are_busy_other_repositories_are_independent_and_drop_releases() {
    // Arrange
    let first = Repository::new();
    let other = Repository::new();
    let alias = other.0.join("alias");
    std::os::unix::fs::symlink(&first.0, &alias).unwrap();

    // Act
    let admission = Admission::acquire(&alias).unwrap();
    let independent = Admission::acquire(&other.0).unwrap();

    // Assert
    assert_eq!(admission.repository(), first.0);
    assert_eq!(independent.repository(), other.0);
    for path in [&first.0, &first.0.join("."), &alias] {
        assert!(matches!(Admission::acquire(path), Err(Refusal::Busy)));
    }
    assert_eq!(std::fs::read_dir(&first.0).unwrap().count(), 0);

    // Act / Assert
    drop(admission);
    assert!(Admission::acquire(&alias).is_ok());
}

#[test]
fn relative_missing_and_file_paths_are_invalid() {
    // Arrange
    let root = Repository::new();
    let file = root.0.join("file");
    std::fs::write(&file, b"").unwrap();

    // Act / Assert
    for path in [Path::new("."), &root.0.join("missing"), &file] {
        let error = Admission::acquire(path).unwrap_err();
        assert!(matches!(error, Refusal::Invalid(_)));
        assert!(error.to_string().contains(&path.display().to_string()));
    }
}

#[test]
fn exec_inheritance_is_opt_in_and_child_exit_releases_admission() {
    // Descriptor inheritance affects every concurrent fork; isolate this test.
    if !std::env::args().any(|arg| arg == "--test-threads=1") {
        let thread = std::thread::current();
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                thread.name().unwrap(),
                "--test-threads=1",
                "--nocapture",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("1 passed;"),
            "{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    for inherit in [false, true] {
        // Arrange: cat stays alive until its pipe closes or it is killed.
        let root = Repository::new();
        let admission = Admission::acquire(&root.0).unwrap();
        if inherit {
            admission.keep_across_exec().unwrap();
        }
        let mut child = Command::new("/bin/cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();

        // Act
        drop(admission);
        let next = Admission::acquire(&root.0);

        // Assert
        if inherit {
            assert!(matches!(next, Err(Refusal::Busy)));
        } else {
            assert!(next.is_ok());
        }
        drop(next);

        // Act / Assert: process death also closes the inherited holder.
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(Admission::acquire(&root.0).is_ok());
    }
}
