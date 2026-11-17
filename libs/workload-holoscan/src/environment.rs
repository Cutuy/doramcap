//! Approved environment contract for native container Build and Run commands.
use tokio::process::Command;

const BUILD_LOCAL: &str = "HOLOSCAN_CLI_BUILD_LOCAL";
const CONTAINER: &str = "0";

/// Set child-only `HOLOSCAN_CLI_BUILD_LOCAL=0` for native Build, Run and their dry-runs.
/// CLI 5 uses this key for both operations; callers must omit its overriding `--local` flag.
/// Preserves the parent environment, other child variables, program, arguments, cwd and I/O.
pub fn configure_container(command: &mut Command) {
    command.env(BUILD_LOCAL, CONTAINER);
}
