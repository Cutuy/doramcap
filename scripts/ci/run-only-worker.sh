#!/usr/bin/env bash
# Both hosts use these paths because some tests embed source and executable paths.
set -euo pipefail
[[ $# == 1 && ( $1 == build || $1 == run ) && $(uname -m) == aarch64 ]]
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
worker_job=$(pwd -P)
worker_cache=/tmp/doramcap-ci-run-only
umask 077
mkdir -p "$worker_cache"
[[ -O $worker_cache && ! -L $worker_cache ]]
chmod 700 "$worker_cache"
exec 9>"$worker_cache/worker.lock"
flock 9
if [[ $1 == build ]]; then
    uname -a
    node "$worker_job/source/scripts/ci/prepare-build.mts" "$worker_job/source" "$worker_cache/source"
    cd "$worker_cache/source"
    bash scripts/ci/install-nextest.sh
    export CARGO_TARGET_DIR="$worker_cache/target"
    export CARGO_PROFILE_DEV_DEBUG=line-tables-only
    export CLANG=clang-18
    test "$(node --version)" = "v$(cat .node-version)"
    rustc --version
    cargo --version
    mkdir -p "$worker_job/artifacts"
    for suite in default all-features storage-no-default; do
        case $suite in
            default) options=(--workspace) ;;
            all-features) options=(--workspace --all-features) ;;
            storage-no-default) options=(-p storage-persistence --no-default-features) ;;
        esac
        cargo nextest archive --locked "${options[@]}" --archive-file "$worker_job/artifacts/$suite.tar.zst"
    done
    cp "$worker_job/source/source.tar.gz" "$worker_job/artifacts/source.tar.gz"
    cp "$HOME/.local/bin/cargo-nextest" "$worker_job/artifacts/cargo-nextest"
    cd "$worker_job/artifacts"
    sha256sum source.tar.gz cargo-nextest default.tar.zst all-features.tar.zst storage-no-default.tar.zst > SHA256SUMS
else
    exec > "$worker_job/run.log" 2>&1
    trap 'worker_result=$?; printf "%s\n" "$worker_result" > "$worker_job/exit-code"' EXIT
    rm -f -- "$worker_job/exit-code" "$worker_job/results.tsv"
    uname -a
    cd "$worker_job/artifacts"
    sha256sum -c SHA256SUMS
    rm -rf -- "$worker_cache/source" "$worker_cache/target"
    mkdir "$worker_cache/source"
    tar -xzpf source.tar.gz -C "$worker_cache/source"
    cd "$worker_cache/source"
    mkdir -p "$HOME/.local/bin"
    install -m 755 "$worker_job/artifacts/cargo-nextest" "$HOME/.local/bin/cargo-nextest"
    cargo-nextest --version
    printf 'Source manifest: %s\n' "$worker_job/source/SOURCE.json"
    sha256sum SOURCE.json
    printf '\nMemory limit: '
    if [[ -r /sys/fs/cgroup/memory.max ]]; then cat /sys/fs/cgroup/memory.max; else echo unavailable; fi
    worker_failed=0
    : > "$worker_job/results.tsv"
    for suite in default all-features storage-no-default; do
        rm -rf -- "$worker_cache/target"
        printf '\nSuite: %s\n' "$suite"
        if cargo-nextest nextest run --archive-file "$worker_job/artifacts/$suite.tar.zst" \
            --extract-to "$worker_cache" --test-threads 1 --no-fail-fast; then
            printf '%s\t0\n' "$suite" >> "$worker_job/results.tsv"
        else
            printf '%s\t%s\n' "$suite" "$?" >> "$worker_job/results.tsv"
            worker_failed=1
        fi
    done
    printf '\nPrivileged native tests and doctests remain on the WSL worker.\n'
    exit "$worker_failed"
fi
