#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
export CARGO_TARGET_DIR="$HOME/target"
export CLANG=clang-18
export RUSTUP_TOOLCHAIN=1.99.1
# Hold the lock until all consumers of this source/target pair have finished.
worker_snapshot=$(pwd -P)
worker_cache="$HOME/jobs/.build-cache"
mkdir -p "$worker_cache"
exec 9>"$worker_cache/worker.lock"
flock 9
printf 'Submitted snapshot: %s\n' "$worker_snapshot"
python3 "$worker_snapshot/prepare-build.py" "$worker_snapshot" "$worker_cache/source"
cd "$worker_cache/source"
uname -a
rustc --version
cargo --version
cat SOURCE.json
rustup component add rustfmt clippy --toolchain 1.99.1
python3 scripts/ci/check_boundaries.py --rules scripts/ci/boundaries.json
python3 scripts/ci/test_boundaries.py
cargo fmt --all -- --check
cargo clippy --workspace --no-deps --all-targets --locked -- -D warnings
cargo clippy --workspace --no-deps --all-targets --all-features --locked -- -D warnings
cargo test --locked -p storage-persistence --no-default-features
cargo test --locked --workspace
cargo test --locked --workspace --all-features
cargo build --locked --workspace --all-features --examples --bins
"$CARGO_TARGET_DIR/debug/examples/policy_overhead"
sudo -n /usr/local/libexec/doramcap-ci-native-tests "$PWD"

"$CARGO_TARGET_DIR/debug/examples/storage_benchmark"
python3 apps/storage-demo/validate.py --bin-dir "$CARGO_TARGET_DIR/debug"
python3 apps/storage-demo/tests/activity_capture.py --bin-dir "$CARGO_TARGET_DIR/debug"
if [[ -x "$HOME/.local/bin/dora" ]]; then
    python3 apps/system-monitor/tests/validate_recording_dora.py --dora "$HOME/.local/bin/dora" --bin-dir "$CARGO_TARGET_DIR/debug"
fi
