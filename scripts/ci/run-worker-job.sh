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
cargo test --locked -p storage-persistence --no-default-features
cargo test --locked --workspace
cargo test --locked --workspace --all-features
cargo build --locked --workspace --all-features --examples --bins
sudo -n /usr/local/libexec/doramcap-ci-native-tests "$PWD"

"$CARGO_TARGET_DIR/debug/examples/storage_benchmark"
if [[ -x "$HOME/.local/bin/dora" ]]; then
    python3 apps/system-monitor/tests/validate_recording_dora.py --dora "$HOME/.local/bin/dora" --bin-dir "$CARGO_TARGET_DIR/debug"
fi

rustup component add clippy --toolchain 1.99.1
cargo clippy -p storage-persistence -p telemetry-mcap -p system-monitor -p system-monitor-dora --no-deps --all-targets --all-features --locked -- -D warnings
