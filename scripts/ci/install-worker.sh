#!/usr/bin/env bash
# Run as root on Ubuntu ARM64. Argument: authorized public-key file.
set -euo pipefail
[[ $(uname -m) == aarch64 && $EUID == 0 && $# == 1 ]]
worker_tmp=$(mktemp -d)
trap 'rm -rf -- "$worker_tmp"' EXIT
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y --no-install-recommends clang-18 libclang-18-dev llvm-18-dev build-essential pkg-config libelf-dev zlib1g-dev curl ca-certificates python3 python3-protobuf git rsync util-linux xz-utils
id doramcap-ci >/dev/null 2>&1 || useradd --create-home --shell /bin/bash doramcap-ci
install -d -m 700 -o doramcap-ci -g doramcap-ci /home/doramcap-ci/.ssh
install -m 600 -o doramcap-ci -g doramcap-ci "$1" /home/doramcap-ci/.ssh/authorized_keys
# Install the repository-pinned Node runtime for unprivileged CI jobs.
node_version=$(cat "$(dirname "$0")/../../.node-version")
sudo -u doramcap-ci -H bash -s -- "$node_version" <<'NODE'
set -euo pipefail
node_archive="node-v$1-linux-arm64.tar.xz"
node_stage=$(mktemp -d)
trap 'rm -rf -- "$node_stage"' EXIT
cd "$node_stage"
curl -fsSLO "https://nodejs.org/dist/v$1/$node_archive"
curl -fsSLO "https://nodejs.org/dist/v$1/SHASUMS256.txt"
sha256sum --check --ignore-missing SHASUMS256.txt
mkdir -p "$HOME/.local/lib" "$HOME/.local/bin"
tar -xJf "$node_archive" -C "$HOME/.local/lib"
for binary in node npm npx; do
  ln -sfn "$HOME/.local/lib/node-v$1-linux-arm64/bin/$binary" "$HOME/.local/bin/$binary"
done
NODE
for binary in pueue pueued; do
  curl -fsSL "https://github.com/Nukesor/pueue/releases/download/v4.0.4/${binary}-aarch64-unknown-linux-musl" -o "$worker_tmp/${binary}"
done
echo "759bf5100a51024997111c6913aaf3330a0cdfd893ff552dcf429ae9b5e01e09  $worker_tmp/pueue" | sha256sum -c -
echo "332c5ef74270b64aeaf04894c8c04826f3422eb7d50dbd1a8e0706d74a42f653  $worker_tmp/pueued" | sha256sum -c -
install -m 755 "$worker_tmp/pueue" /usr/local/bin/pueue
install -m 755 "$worker_tmp/pueued" /usr/local/bin/pueued
install -d -m 755 /usr/local/libexec
cat > /usr/local/libexec/doramcap-ci-native-tests <<'LAUNCHER'
#!/usr/bin/env bash
# Trusted CI code executes as root. Path checks prevent accidental misuse only.
set -euo pipefail
[[ $# == 1 ]]
job=$(realpath -- "$1")
[[ $job == /home/doramcap-ci/jobs/* && -d $job ]]
export CARGO_TARGET_DIR=/home/doramcap-ci/target
cd "$job"
python3 apps/system-monitor/tests/validate_native.py --monitor "$CARGO_TARGET_DIR/debug/system-monitor"
"$CARGO_TARGET_DIR/debug/examples/native_allocations"
if [[ -f apps/system-monitor/tests/validate_recording_dora.py && -x /home/doramcap-ci/.local/bin/dora ]]; then
    python3 apps/system-monitor/tests/validate_recording_dora.py --dora /home/doramcap-ci/.local/bin/dora --bin-dir "$CARGO_TARGET_DIR/debug" --backend bpf
fi
LAUNCHER
chmod 755 /usr/local/libexec/doramcap-ci-native-tests
printf '%s\n' 'doramcap-ci ALL=(root) NOPASSWD: /usr/local/libexec/doramcap-ci-native-tests' > /etc/sudoers.d/doramcap-ci
chmod 440 /etc/sudoers.d/doramcap-ci
visudo -cf /etc/sudoers.d/doramcap-ci
cat > /etc/systemd/system/doramcap-pueued.service <<'SERVICE'
[Unit]
Description=Doramcap ARM64 test queue
After=network.target
[Service]
User=doramcap-ci
Group=doramcap-ci
Environment=HOME=/home/doramcap-ci
WorkingDirectory=/home/doramcap-ci
ExecStart=/usr/local/bin/pueued
Restart=on-failure
RestartSec=3
[Install]
WantedBy=multi-user.target
SERVICE
install -d -m 700 -o doramcap-ci -g doramcap-ci /home/doramcap-ci/.config/pueue /home/doramcap-ci/.local/share/pueue
cat > /home/doramcap-ci/.config/pueue/pueue.yml <<'CONFIG'
shared:
  use_unix_socket: true
  unix_socket_path: /home/doramcap-ci/.local/share/pueue/pueue.socket
CONFIG
chown doramcap-ci:doramcap-ci /home/doramcap-ci/.config/pueue/pueue.yml
systemctl daemon-reload
systemctl enable doramcap-pueued.service
systemctl restart doramcap-pueued.service

for attempt in {1..20}; do
  sudo -u doramcap-ci -H pueue parallel 1 && break
  sleep 0.2
done
sudo -u doramcap-ci -H pueue status
# Install the pinned Rust toolchain under the unprivileged account.
sudo -u doramcap-ci -H bash -c 'curl -fsSL https://sh.rustup.rs -o "$HOME/rustup-init.sh"; sh "$HOME/rustup-init.sh" -y --profile minimal --default-toolchain 1.99.1'
