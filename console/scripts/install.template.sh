#!/bin/sh
set -eu
if [ "$(uname -s)" != Linux ]; then
  echo 'Run this installer on the Linux Docker host.' >&2
  exit 1
fi
case "$(uname -m)" in
  x86_64) arch=amd64; checksum='@@AMD64@@' ;;
  aarch64|arm64) arch=arm64; checksum='@@ARM64@@' ;;
  *) echo 'Supported architectures: x86_64 and ARM64.' >&2; exit 1 ;;
esac
for command in curl tar sha256sum sudo; do
  command -v "$command" >/dev/null || { echo "Required command: $command" >&2; exit 1; }
done
if [ "$(id -u)" = 0 ]; then
  echo 'Run the installer as your normal user; it requests sudo after download verification.' >&2
  exit 1
fi
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT HUP INT TERM
archive='doramcap-console-@@VERSION@@-linux-'"$arch"'.tar.gz'
echo "Downloading Console @@VERSION@@ for linux-$arch..."
curl --fail --location --proto '=https' --proto-redir '=https' --tlsv1.2 --retry 3 \
  '@@BASE@@/'"$archive" -o "$work/release.tar.gz"
printf '%s  %s\n' "$checksum" "$work/release.tar.gz" | sha256sum -c -
tar -xzf "$work/release.tar.gz" -C "$work" --no-same-owner --no-same-permissions
echo 'Verified. Installing under /opt/doramcap-console (sudo required)...'
sudo -- "$work/package/runtime/node" "$work/package/server/dist/cli.js" install
