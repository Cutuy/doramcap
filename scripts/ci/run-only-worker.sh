#!/usr/bin/env bash
set -euo pipefail
worker_script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
worker_cache_directory=/tmp/doramcap-ci-run-only
worker_failure_exit=1
worker_usage_exit=2
worker_hangup_exit=129
worker_interrupt_exit=130
worker_terminate_exit=143
worker_test_threads=1
compose_plugin=/usr/libexec/docker/cli-plugins/docker-compose
sha256_entry_pattern='^[[:xdigit:]]{64}  ([a-z][a-z0-9.-]*)$'

worker_fail() {
    printf 'run-only-worker: %s\n' "$1" >&2
    exit "${2:-$worker_failure_exit}"
}

# Require every expected artifact exactly once before checking any content.
verify_artifacts() {
    local line artifact
    local -A remaining=()
    for artifact in "${worker_artifacts[@]}"; do remaining[$artifact]=1; done
    [[ -r SHA256SUMS ]] || worker_fail 'Missing checksum manifest: SHA256SUMS'
    while IFS= read -r line || [[ -n $line ]]; do
        [[ $line =~ $sha256_entry_pattern ]] || worker_fail "Malformed checksum entry: $line"
        artifact=${BASH_REMATCH[1]}
        [[ ${remaining[$artifact]+present} ]] || worker_fail "Unexpected or duplicate artifact: $artifact"
        unset 'remaining[$artifact]'
    done < SHA256SUMS
    [[ ${#remaining[@]} == 0 ]] || worker_fail "Missing checksum entries: ${!remaining[*]}"
    sha256sum -c SHA256SUMS
}

worker_finish() {
    local result=$?
    trap - EXIT HUP INT TERM
    if [[ -n $worker_lock_waiter ]]; then
        kill "$worker_lock_waiter" 2>/dev/null || true
        wait "$worker_lock_waiter" 2>/dev/null || true
    fi
    if [[ $worker_mode == run ]]; then printf '%s\n' "$result" > "$worker_job/exit-code"; fi
}

# Execute one job; main supplies the fixed cache, while tests use isolated paths.
run_worker() {
    worker_mode=$1
    worker_job=$2
    worker_cache=$3
    worker_lock_waiter=''
    umask 077
    trap worker_finish EXIT
    trap 'exit "$worker_hangup_exit"' HUP
    trap 'exit "$worker_interrupt_exit"' INT
    trap 'exit "$worker_terminate_exit"' TERM
    if [[ $worker_mode == run ]]; then
        rm -f -- "$worker_job/exit-code" "$worker_job/results.tsv"
        exec > "$worker_job/run.log" 2>&1
    fi
    mkdir -p "$worker_cache"
    [[ -O $worker_cache && ! -L $worker_cache ]] || worker_fail "Cache must be an owned, real directory: $worker_cache"
    chmod 700 "$worker_cache"
    exec {worker_lock_fd}>"$worker_cache/worker.lock"
    flock "$worker_lock_fd" &
    worker_lock_waiter=$!
    wait "$worker_lock_waiter"
    worker_lock_waiter=''
    mapfile -t worker_artifacts < "$worker_script_directory/run-only-artifacts.txt"
    uname -a
    if [[ $worker_mode == build ]]; then
        local expected_node actual_node tool
        for tool in node rustc cargo docker; do
            command -v "$tool" >/dev/null || worker_fail "Missing build tool: $tool"
        done
        expected_node="v$(cat "$worker_job/source/.node-version")"
        actual_node=$(node --version)
        [[ $actual_node == "$expected_node" ]] || worker_fail "Node version $actual_node; expected $expected_node"
        [[ -x $compose_plugin ]] || worker_fail "Missing Docker Compose plugin: $compose_plugin"
        node "$worker_job/source/scripts/ci/prepare-build.mts" "$worker_job/source" "$worker_cache/source"
        cd "$worker_cache/source"
        bash scripts/ci/install-nextest.sh
        export CARGO_TARGET_DIR="$worker_cache/target"
        export CARGO_PROFILE_DEV_DEBUG=line-tables-only
        export CLANG=clang-18
        rustc --version
        cargo --version
        docker --version
        docker compose version
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
        cp "$(command -v docker)" "$worker_job/artifacts/docker"
        cp "$compose_plugin" "$worker_job/artifacts/docker-compose"
        cd "$worker_job/artifacts"
        sha256sum "${worker_artifacts[@]}" > SHA256SUMS
    else
        cd "$worker_job/artifacts"
        verify_artifacts
        rm -rf -- "$worker_cache/source" "$worker_cache/target"
        mkdir "$worker_cache/source"
        tar -xzpf source.tar.gz -C "$worker_cache/source"
        cd "$worker_cache/source"
        mkdir -p "$HOME/.local/bin" "$HOME/.docker/cli-plugins"
        install -m 755 "$worker_job/artifacts/cargo-nextest" "$HOME/.local/bin/cargo-nextest"
        install -m 755 "$worker_job/artifacts/docker" "$HOME/.local/bin/docker"
        install -m 755 "$worker_job/artifacts/docker-compose" "$HOME/.docker/cli-plugins/docker-compose"
        cargo-nextest --version
        docker --version
        docker compose version
        printf 'Memlock soft/hard (KiB): '; ulimit -l; ulimit -Hl
        printf 'Source manifest: %s\n' "$worker_job/source/SOURCE.json"
        sha256sum SOURCE.json
        printf '\nMemory limit: '
        if [[ -r /sys/fs/cgroup/memory.max ]]; then cat /sys/fs/cgroup/memory.max; else echo unavailable; fi
        worker_result=0
        : > "$worker_job/results.tsv"
        for suite in default all-features storage-no-default; do
            rm -rf -- "$worker_cache/target"
            printf '\nSuite: %s\n' "$suite"
            if cargo-nextest nextest run --archive-file "$worker_job/artifacts/$suite.tar.zst" \
                --extract-to "$worker_cache" --test-threads "$worker_test_threads" --no-fail-fast; then
                printf '%s\t0\n' "$suite" >> "$worker_job/results.tsv"
            else
                suite_result=$?
                printf '%s\t%s\n' "$suite" "$suite_result" >> "$worker_job/results.tsv"
                worker_result=$worker_failure_exit
            fi
        done
        printf '\nPrivileged native tests and doctests remain on the WSL worker.\n'
        return "$worker_result"
    fi
}

main() {
    [[ $# == 1 && ( $1 == build || $1 == run ) ]] || worker_fail 'Usage: run-only-worker.sh build|run (from a staged job directory)' "$worker_usage_exit"
    [[ $(uname -s) == Linux && $(uname -m) == aarch64 ]] || worker_fail 'Worker requires Linux aarch64'
    export PATH="$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
    run_worker "$1" "$(pwd -P)" "$worker_cache_directory"
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then main "$@"; fi
