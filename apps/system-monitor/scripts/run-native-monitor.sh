#!/bin/sh
# Elevate only the eBPF collector, keeping Dora and the HTTP server unprivileged.
set -eu
checkout=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
main_checkout=$(git -C "$checkout" worktree list --porcelain | sed -n '1s/^worktree //p')
[ -n "$main_checkout" ] || exit 1
credential="$main_checkout/.sudo-password"
if [ ! -r "$credential" ]; then
    echo "Provision $credential locally with mode 0600 as described in AGENTS.md." >&2
    exit 1
fi
if [ "$(stat -c '%a' "$credential")" != 600 ]; then
    echo ".sudo-password must have mode 0600" >&2
    exit 1
fi
git -C "$main_checkout" check-ignore -q .sudo-password || {
    echo "Main checkout .sudo-password must be gitignored" >&2
    exit 1
}
exec sudo -S -p '' --preserve-env=DORA_NODE_CONFIG -- \
    "$checkout/target/debug/system-monitor" "$@" < "$credential"
