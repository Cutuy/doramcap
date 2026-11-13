# Orchestration log: implementing plans #353, #354, #355

Created 2026-11-13. Kept by the orchestrator (Claude) while the owner is away.

## Setup

- Three Codex agents, `gpt-6.1-sol` at `xhigh`, full access, one per plan, each on its own pool lease.
- Each opens a new draft PR based on its plan branch. Plan PRs stay untouched. Nothing is merged.

| Plan | Impl PR | Codex session |
| --- | --- | --- |
| #353 systemd overlays | #357 | `01a1d4a4-106d-72b1-9b13-e17ab44efb37` |
| #354 keep files with the run | #358 | `01a1d4a4-106d-7501-bdd2-4c89f14dada7` |
| #355 journal export | #356 | `01a1d4a4-1075-7dd3-aeb2-842b85f45a0b` |

## Decisions made by the orchestrator for the owner

| # | Decision | Choice | Why |
| --- | --- | --- | --- |
| 1 | #353 shared end-of-run step names | Rename to neutral IDs, as its own commit | Plan's recommendation; raise if IDs are persisted or in a contract |
| 2 | #353 own unit files | Console drop-in that resets `BindPaths=` | Plan's recommendation; only if the host probe shows it works |
| 3 | #354 download naming | Position in the saved change list | Plan's recommendation; never a path |
| 4 | #354 kept-copy check | Type and size | Plan's recommendation; least complexity |
| 5 | #355 after a Console crash | Export during the next run's cleanup | Plan's recommendation; skip is the fallback if it needs much more machinery |
| 6 | #355 journal reader | `journalctl` child process | Agreed in the run-logs plan; no new crate |
| 7 | PR shape | New stacked draft PRs on the plan branches | Keeps plans reviewable on their own |
| 8 | #355 log downloads | Logs list up to the download link; attaching is a follow-up | The route belongs to #354 |
| 9 | Agent mode | Agents ask only for new important decisions and log their own choices in the PR description | Owner's instruction |

| 10 | #355 where start identities are saved | `logs/index.pb` holds identities at start in a pending state, then export results; no second format | Agent's recommendation; one persisted format instead of two. Differs from the plan's "with the run's ownership records" |
| 11 | #355 agent mode | Told to keep going without answers and log its own choices | It stopped before any implementation; owner is away |
| 12 | #353 drop-in and read-only binds | Drop-in restores both `BindPaths=` and `BindReadOnlyPaths=` from manager properties | Probe: systemd 255 clears `BindReadOnlyPaths=` when `BindPaths=` is reset. Stays with the drop-in approach; patching Console's copy is the fallback |
| 13 | #353 agent mode | Told to keep going without answers and log its own choices | Owner is away |
| 14 | #354 size check for directories | Type for every entry; size for regular files and symlinks only | Probe: the same directory reports 4096 bytes on disk and 60 on tmpfs |
| 15 | #354 agent mode | Told to keep going without answers and log its own choices | Owner is away |
| 16 | Prerequisites | Agents may split a worthwhile prerequisite into its own PR and stack on it | Owner's instruction; passed to #354 now, to the others at their next stop |
| 17 | Log downloads follow-up | New PR on `codex/journal-log-downloads`, based on #358 and carrying #356's commits by cherry-pick | Needs both pieces; no merges. Also shows how the two pieces fit together |
| 18 | systemd runs and the shared write policy | New PR on `codex/systemd-keep-policy`, based on #358 and carrying #357's commits by cherry-pick; replaces #357's `discard_writes` flag with #358's shared policy and removes the temporary rejection | #358 asked for it; needs both pieces |

## Events

- 2026-11-13: agents launched.
- 2026-11-13: owner went AFK; orchestrator decides alone from here and records it in this file.
- 2026-11-13: #355 agent opened draft PR #356 (slot 2), stopped with a question on the index contract; answered (decision 10) and resumed in keep-going mode.
- 2026-11-13: #353 agent opened draft PR #357 (slot 7). Host probe passed: both unit kinds see Console's overlay and writes stay in the upper. It asked about read-only binds; answered (decision 12) and resumed in keep-going mode.
- 2026-11-13: #354 agent opened draft PR #358 (slot 6), milestone 1 partial, privileged OverlayFS test passed. Asked about directory sizes; answered (decision 14) and resumed in keep-going mode.
- 2026-11-13: owner allowed prerequisite PRs and asked for brief progress updates only.
- 2026-11-13: #355 agent finished. PR #356 marked ready, lease released. Agent reports full local CI passed; ARM64 worker job 691 confirmed `Success` by the orchestrator. GitHub CI still running at that point. Its ten own decisions are in the PR description. Log downloads remain a follow-up once #358 lands.
- 2026-11-13: this log published as a draft PR.
- 2026-11-13: GitHub CI on #356 passed (Rust and Node, x86_64 and ARM64).
- 2026-11-13: #354 agent finished. PR #358 marked ready, lease released. Agent reports full local CI and 19 privileged OverlayFS cases passed; ARM64 worker job 696. Download route: `/api/artifacts/<run>/run-file.download?change=<position>`. Its twelve own decisions are in the PR description. Left for others: systemd runs reject keep/discard at Review until #357 consumes the shared policy.
- 2026-11-13: journal agent resumed to attach log downloads (decision 17).
- 2026-11-13: GitHub CI on #358 passed on both architectures.
- 2026-11-13: #353 agent finished. PR #357 marked ready, lease released, all four milestones ticked. Agent reports full local CI plus 29 systemd, 5 cgroup and 17 OverlayFS scenarios passed; ARM64 worker job 698 confirmed `Success`; GitHub CI passed. The step-ID rename is its own commit, `d1002e6d`. Its fourteen own decisions are in the PR description.
- 2026-11-13: systemd agent resumed for the write-policy integration (decision 18).
- 2026-11-13: Bugbot flagged one medium issue on #356 ("Reset leaves runner blocked", `libs/workload-systemd/src/runner.rs`). Queued for the journal agent once its current turn ends. No Bugbot findings on #357 or #358 at that point.
- 2026-11-13: log downloads done. PR #360 marked ready (base #358, carries #355/#356; only `d774715a` is new). Agent reports full local CI and Chromium download checks passed; ARM64 worker job 699 confirmed `Success`. One conflict between the two pieces, in `workload-lifecycle/build.rs`. Its four own decisions are in the PR description.
- 2026-11-13: journal agent resumed to check and fix the Bugbot finding on #356 and carry it into #360.
- 2026-11-13: Bugbot finding on #356 confirmed real and fixed, with a native regression test; same fix carried into #360. New heads: #356 `39c6febc`, #360 `f8a153e8`. Focused checks only; full local CI and the ARM64 worker were not rerun on these heads. GitHub CI rerunning.
- 2026-11-13: GitHub CI passed on the new heads of #356 and #360.
- 2026-11-13: systemd write-policy integration done. PR #361 marked ready (base #358, carries #353/#357; only `b923d4c5` and `1ec8213c` are new). systemd runs take all three policies; the `discard_writes` flag and the temporary rejection are gone. Agent reports full local CI and privileged keep/discard scenarios passed; ARM64 worker job 700 confirmed `Success`; GitHub CI passed. Its own decisions are in the PR description.
- 2026-11-13: Bugbot flagged one medium issue on #357 ("Overlay drop-in resolves binds before SHM drop-in exists", `libs/workload-systemd/src/runtime.rs`). Systemd agent resumed to check and fix it and carry it into #361.
