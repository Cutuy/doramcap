use tokio::process::Command;
use workload_holoscan::environment::configure_container;

#[tokio::test]
async fn container_execution_overrides_only_the_child_local_selector() {
    // Arrange
    let parent_selector = std::env::var_os("HOLOSCAN_CLI_BUILD_LOCAL");
    let cwd = std::env::current_dir().unwrap();
    let mut command = Command::new("/bin/sh");
    command
        .args([
            "-c",
            r#"printf '%s\n' "$0" "$1" "$PWD" "$HOLOSCAN_CLI_BUILD_LOCAL" "$DOCKER_HOST" "$DOCKER_CONTEXT" "$DOCKER_CONFIG" "$DOCKER_TLS" "$DOCKER_TLS_VERIFY" "${HOLOSCAN_CLI_RUN_LOCAL+present}"
printf preserved-stderr >&2"#,
            "literal program label",
            "argument with spaces and $literal",
        ])
        .current_dir(&cwd)
        .env("HOLOSCAN_CLI_BUILD_LOCAL", "1")
        .env("DOCKER_HOST", "unix:///preserved/socket")
        .env("DOCKER_CONTEXT", "preserved-context")
        .env("DOCKER_CONFIG", "/preserved/config")
        .env("DOCKER_TLS", "preserved-tls")
        .env("DOCKER_TLS_VERIFY", "preserved-verify")
        .env_remove("HOLOSCAN_CLI_RUN_LOCAL");

    // Act
    configure_container(&mut command);
    let output = command.output().await.unwrap();

    // Assert
    assert!(output.status.success());
    let expected = format!(
        "literal program label\nargument with spaces and $literal\n{}\n0\nunix:///preserved/socket\npreserved-context\n/preserved/config\npreserved-tls\npreserved-verify\n\n",
        cwd.display()
    );
    assert_eq!(output.stdout, expected.as_bytes());
    assert_eq!(output.stderr, b"preserved-stderr");
    assert_eq!(
        std::env::var_os("HOLOSCAN_CLI_BUILD_LOCAL"),
        parent_selector
    );
}
