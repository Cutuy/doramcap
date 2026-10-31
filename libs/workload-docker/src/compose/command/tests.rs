use super::*;

fn shell(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", script]);
    command
}

#[tokio::test]
async fn successful_commands_log_exact_arguments_and_exclude_environment_and_output() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let mut command = shell("printf '%s' \"$TEST_VALUE\"");
    command
        .arg("argument with spaces\nand a newline")
        .current_dir(root.path())
        .env("TEST_VALUE", "private-sentinel");
    let mut log = Vec::new();
    // Act
    let (stdout, _) = execute(command, &[], Duration::from_secs(5), &mut log)
        .await
        .unwrap();
    // Assert
    let log = String::from_utf8(log).unwrap();
    assert_eq!(stdout, "private-sentinel");
    assert_eq!(log.lines().count(), 2);
    assert!(log.contains("program=\"/bin/sh\""));
    assert!(log.contains("argument with spaces\\nand a newline"));
    assert!(log.contains(root.path().to_str().unwrap()));
    assert!(log.contains("timeout_ms=5000"));
    assert!(log.contains("outcome=success exit_code=Some(0)"));
    assert!(!log.contains("private-sentinel"));
    let ids: Vec<_> = log
        .lines()
        .map(|line| {
            line.split_whitespace()
                .find(|field| field.starts_with("id="))
                .unwrap()
        })
        .collect();
    assert_eq!(ids[0], ids[1]);
}

#[tokio::test]
async fn failed_commands_log_exit_status_without_dumping_captured_stderr() {
    // Arrange
    let mut command = shell("printf '%s' \"$TEST_VALUE\" >&2; exit 7");
    command.env("TEST_VALUE", "private-error");
    let mut log = Vec::new();
    // Act
    let result = execute(command, &[], Duration::from_secs(5), &mut log).await;
    // Assert
    assert!(
        matches!(result, Err(Error::Command { status: Some(7), ref stderr, .. }) if stderr == "private-error")
    );
    let log = String::from_utf8(log).unwrap();
    assert!(log.contains("outcome=failed exit_code=Some(7)"));
    assert!(!log.contains("private-error"));
}

#[tokio::test]
async fn spawn_failures_and_signals_have_terminal_records() {
    // Arrange
    let command = Command::new("/nonexistent/doramcap-cli-test");
    let mut log = Vec::new();
    // Act
    let result = execute(command, &[], Duration::from_secs(5), &mut log).await;
    // Assert
    assert!(matches!(result, Err(Error::Io(_))));
    assert!(
        String::from_utf8(log)
            .unwrap()
            .contains("outcome=start_failed")
    );

    // Arrange
    let command = shell("kill -TERM $$");
    let mut log = Vec::new();
    // Act
    let result = execute(command, &[], Duration::from_secs(5), &mut log).await;
    // Assert
    assert!(matches!(result, Err(Error::Command { status: None, .. })));
    assert!(
        String::from_utf8(log)
            .unwrap()
            .contains("outcome=failed exit_code=None signal=Some(15)")
    );
}

#[tokio::test]
async fn timeouts_and_cancellation_have_terminal_records() {
    // Arrange
    let mut command = Command::new("/bin/sleep");
    command.arg("10");
    let mut log = Vec::new();
    // Act
    let result = execute(command, &[], Duration::from_millis(20), &mut log).await;
    // Assert
    assert!(matches!(result, Err(Error::Timeout { .. })));
    assert!(
        String::from_utf8(log)
            .unwrap()
            .contains("outcome=timed_out")
    );

    // Arrange
    let mut command = Command::new("/bin/sleep");
    command.arg("10");
    let mut log = Vec::new();
    let mut pending = Box::pin(execute(command, &[], Duration::from_secs(5), &mut log));
    // Act: cancellation drops the pending process future.
    assert!(futures_util::poll!(&mut pending).is_pending());
    drop(pending);
    // Assert
    assert!(
        String::from_utf8(log)
            .unwrap()
            .contains("outcome=cancelled")
    );
}

#[tokio::test]
async fn output_limits_have_terminal_records() {
    // Arrange
    let command = shell("head -c 16777217 /dev/zero");
    let mut log = Vec::new();
    // Act
    let result = execute(command, &[], Duration::from_secs(5), &mut log).await;
    // Assert
    assert!(matches!(result, Err(Error::OutputLimit { .. })));
    assert!(
        String::from_utf8(log)
            .unwrap()
            .contains("outcome=output_limit")
    );
}
