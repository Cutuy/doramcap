#!/usr/bin/env bash
# User-level tool installation for both ARM64 build and execution workers.
set -euo pipefail
[[ $(uname -s) == Linux && $(uname -m) == aarch64 ]]
nextest_version=0.9.151
nextest_sha256=3e35e8f6913af92558ba09d7155f74e35dd1f0024ef3cd891637433ae4eacbdd
nextest_binary="$HOME/.local/bin/cargo-nextest"
if [[ -x $nextest_binary ]] && "$nextest_binary" --version | grep -q "^cargo-nextest $nextest_version "; then
    exit 0
fi
nextest_stage=$(mktemp -d)
trap 'rm -rf -- "$nextest_stage"' EXIT
curl --fail --location --show-error --connect-timeout 10 --max-time 120 \
    "https://get.nexte.st/$nextest_version/linux-arm" -o "$nextest_stage/nextest.tar.gz"
printf '%s  %s\n' "$nextest_sha256" "$nextest_stage/nextest.tar.gz" | sha256sum -c -
tar -xzf "$nextest_stage/nextest.tar.gz" -C "$nextest_stage"
mkdir -p "$HOME/.local/bin"
install -m 755 "$nextest_stage/cargo-nextest" "$nextest_binary"
"$nextest_binary" --version
