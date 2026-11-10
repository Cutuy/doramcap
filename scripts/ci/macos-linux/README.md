# Mac Linux run-only CI

WSL `.20` builds native ARM64 archives; `mac-mini-ubuntu24@100.64.0.22` runs
them serially with 1 GiB RAM and 64 MiB soft/hard memlock.

```sh
npm --prefix scripts/ci ci --ignore-scripts --no-audit --no-fund
node scripts/ci/macos-linux/submit.mts --source /absolute/path/to/checkout
```

Requires SSH access and the pinned Node runtime on both hosts. Provision Node with
`node scripts/ci/install-node.mts --host USER@HOST`; it installs under `~/.local`.
Keep submission connected until completion. The printed job directories retain
`run.log`, `results.tsv` and `exit-code` until manually removed.

[The worker](worker.mts) builds archives on WSL and executes them on the Mac.
[Artifact verification](artifacts.mts) checks the complete inventory before cache
replacement or tool installation. Native BPF checks stay on WSL.

See the [plan and validation](docs/110826-run-only-worker.md) and
[shared CI instructions](../README.md).
