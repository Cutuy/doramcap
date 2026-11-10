# MCAP repair decision

Astra/xhigh returned `needs_decision` after 148.4 seconds. The controller acquired
and retains the lease on `codex/code-scan-mcap-fix`. Production source is unchanged.

[FixTask](FixTask.json) → [invocation](invocation.json) → [PatchProposal](PatchProposal.json) → [measured result](result.json).

| Option | Change | Tradeoff |
|---|---|---|
| Deadline barriers — recommended | Two sink files; wait for actual batch/flush deadlines and use the existing blocking completion barrier | Aged-batch flushes can block earlier and increase queue pressure; idle errors surface at the next write/barrier, potentially the configured deadline of up to 60 seconds |
| Storage notifications | Expand scope to storage APIs and tests; signal completion/capacity and select with packets/shutdown/deadlines | Larger lifecycle/concurrency change and validation scope; preserves asynchronous progress and prompt failure observation |

`AGENTS.md:20` requires the user to choose important behavior changes. No dependent
implementation, compilation, capability test, full CI, or ARM64 job has started.

- Native schema, task identity, unchanged source scope, and all nine exact source citations passed.
- One model session ran with concurrency one; its tool reads could run concurrently. One earlier CLI invocation failed argument parsing before starting a model session.
- Usage: 295,736 input tokens, including 232,576 cached; 5,585 output tokens, including reported reasoning usage of 1,706.
- Estimated API-equivalent cost: **$1.143426** for this stage, **$1.515101** across screening, confirmation, and this repair attempt. Actual billing and served model are unknown; the supervising chat is excluded. [Astra rates](https://developers.openai.com/api/docs/models/gpt-6-astra) checked November 8, 2026.
- Complete options, source quotes, and proposed validation are in `PatchProposal.json`; machine-readable totals are in [summary.json](summary.json).
