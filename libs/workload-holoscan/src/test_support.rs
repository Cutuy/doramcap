/// Rerun a lock-sensitive test alone; concurrent forks briefly inherit CLOEXEC locks.
/// Returns true in the parent after checking the isolated test's result.
pub(crate) fn ran_in_isolated_process() -> bool {
    if std::env::args().any(|arg| arg == "--test-threads=1") {
        return false;
    }
    let thread = std::thread::current();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
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
    true
}
