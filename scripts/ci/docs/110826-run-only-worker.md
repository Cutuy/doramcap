# ARM64 build and execution workers

![ARM64 source, build and execution flow](110826-run-only-worker.svg)

Created: 2026-11-08. [Implementation](110826-run-only-worker-impl.md).

The submitter sends an exact source snapshot to WSL ARM64 `.20`. Its serial
Pueue queue builds nextest archives for default workspace tests, all-feature
workspace tests and storage tests without default features. The submitter copies
the archives and matching source to Mac mini Linux `.22`, then runs one test at
a time there. The submitter may be x86; compilation happens natively on ARM64.

Both hosts use `/tmp/doramcap-ci-run-only/source` and `target` under a worker
lock. Keeping the same paths preserves tests that embed source fixtures or
executable paths. The private build cache is separate from the existing native
worker cache. Nextest owns executable discovery, packaging and execution.

The Mac mini container has a 1 GiB memory limit, no swap, and Linux 6.18.35.
The observed kernel/capability restrictions prevent privileged BPF and cgroup
validation. Those tests, doctests, Node checks and linting remain with the
existing WSL/GitHub workflows. Ignored tests retain their normal ignore status.
The new worker installs only a pinned user-level nextest executable; it needs
neither a compiler nor a queue daemon.

Each job retains its source snapshot, archive checksums and logs in `~/jobs`.
A failed queued build stops transfer. Execution attempts all three suites,
records each exit status and returns failure if any suite fails. A runner lock
serializes overlapping submissions. SSH must stay connected during execution;
retained logs and exit status distinguish completion from interrupted work.
Rerunning the retained runner job repeats execution without rebuilding.
