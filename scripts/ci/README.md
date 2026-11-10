# CI

GitHub Actions runs these suites on ARM64 and x86_64 Ubuntu runners:

| Suite | Coverage |
| --- | --- |
| [Rust](../../.github/workflows/ci.yml) | Library boundaries, formatting, default/all-feature Clippy and tests, storage without default features, snapshot synchronization. |
| [Node](../../.github/workflows/node.yml) | App and Console tests, Console builds/type checks and HTTP integration, Project Explorer build. |
| [MCAP](../../.github/workflows/mcap.yml) | Real-writer recordings checked by Rust protobuf and TypeScript streaming readers. |

The workflows define path filters and exact commands. Rust uses the `ci` Cargo
profile; runtimes are pinned in [rust-toolchain.toml](../../rust-toolchain.toml)
and [.node-version](../../.node-version). CI tools run directly as TypeScript using
the CI npm dependencies, Git, GNU tar, SSH/SCP, and rsync.
The worker also uses system tar to extract submitted archives.

## Workers

| Use case | Submit command | Instructions |
| --- | --- | --- |
| Native WSL2 CI | `node scripts/ci/wsl2/submit.mts --source /path/to/checkout` | [WSL2](wsl2/README.md) |
| Mac Linux run-only CI | `node scripts/ci/macos-linux/submit.mts --source /path/to/checkout` | [Mac Linux](macos-linux/README.md) |

Worker-specific code, tests and records live with each use case. Shared snapshot,
process, SSH, hashing and runtime-installation utilities stay in this directory.

## Local checks

Use the pinned Node runtime, Git, GNU tar and rsync. Boundary rules use standard,
case-sensitive minimatch syntax.

```sh
npm --prefix scripts/ci ci --ignore-scripts --no-audit --no-fund
npm --prefix scripts/ci test
npm --prefix scripts/ci run typecheck
```

See [AGENTS.md](../../AGENTS.md#testing-and-ci) for validation policy.
