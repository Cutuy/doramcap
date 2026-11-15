# PR 386 fixes: orchestration log

Created 11/15/26. Scope: findings 1, 4, 6, 7, 8 and 10 from
[the rc7 report](111426-rc7-ux-report.md), [PR 386](https://github.com/Cutuy/doramcap/pull/386).
Each finding has a separate agent and fix PR based on the report branch. Nothing is merged.

## Assignments

Agents use GPT-6.1 Sol with xhigh reasoning. The launcher exposes no speed setting
and permits three active subagents. Assignments continue in the same agent after decisions.

| Finding | Agent | Fix PR | State |
| --- | --- | --- | --- |
| 1: service commands in Compare | `finding_1` | Pending | Investigating |
| 4: system-app choice on New run | `finding_4` | Pending | Design proposed; owner decision queued |
| 6: ownership of file changes | `finding_6` | Pending | Investigating |
| 7: service exits and CPU in Compare | `finding_7` | Pending | Investigating |
| 8: rounded machine values | `finding_8` | Pending | Queued |
| 10: file-change warning noise | `finding_10` | Pending | Queued |

Report head: `fe886a0841ce2bcb27a5876b9aa35fb2c2f77ea8`.
Implementation baseline: `9e18b12a6f70a79919f4dfbc6e2811d6ce0a4674`.
The governing guides are `docs/impl-guide.md` and `docs/design-guide.md` at that baseline.

## Decisions

No product decisions have been made for the owner. Important decisions are raised one at a time.

### 1. Worktree capacity — awaiting owner

Context: four of seven pool slots were leased by existing tasks when work began.
Problem: six isolated fixes plus this log require seven additional leases if retained together.

```mermaid
flowchart LR
    A[Three free slots] --> B[Grow pool to eleven slots]
    A --> C[Keep seven slots and queue leases]
```

- Options: grow to eleven slots, or keep seven and wait for releases.
- Recommendation: grow to eleven so each fix and the log can retain its own checkout.
- Choice: pending. The worktree-pool skill requires approval before pool initialization or growth.

## Events

- 11/15/26: Read the report, both guides, and the orchestrator, worktree-pool and Scanner skills.
- 11/15/26: Started findings 1, 4 and 6 independently; finding 4 returned its design proposal and finding 7 started.
- 11/15/26: Requested pool growth. Acquired an existing free slot for this log and stacked it on PR 386.

## Left for the owner

- Decide pool capacity, followed by the queued product decisions.
- Review the individual fix PRs after implementation and validation.
- Choose integration after the fixes are ready; this task does not merge them.
