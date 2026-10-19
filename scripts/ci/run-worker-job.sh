#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
export CARGO_TARGET_DIR="$HOME/target"
export CLANG=clang-18
export RUSTUP_TOOLCHAIN=1.99.1
uname -a
rustc --version
cargo --version
cat SOURCE.json
cargo test --locked --workspace
cargo test --locked --workspace --all-features
cargo build --locked --workspace --all-features --examples --bins
sudo -n /usr/local/libexec/doramcap-ci-native-tests "$PWD"
