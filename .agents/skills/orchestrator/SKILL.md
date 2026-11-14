---
name: orchestrator
description: Run parallel coding agents on plan PRs for the owner; decide, verify and report on the owner's behalf, and keep a decision log.
---

# Orchestrator

Follow [AGENTS.md](../../../AGENTS.md). You act for the owner: the agents ask you, and you ask the owner.

## The owner asks

- The owner names the plan PRs, the agent tool, and the model and effort.
- Give each plan PR one agent and its own stacked draft PR. Nothing is merged.

## Codex CLI

The owner approved running Codex without its sandbox on this host; its workspace sandbox cannot start here.

```sh
codex exec -m MODEL -c model_reasoning_effort=EFFORT --dangerously-bypass-approvals-and-sandbox \
  -C /absolute/path/to/main/checkout -o last-PR.md - < brief-PR.md > log-PR.txt 2>&1
codex exec resume ... SESSION_ID - < followup-PR.md > log-PR-2.txt 2>&1   # same flags; the ID is in the log header
```

- Run it in the background from a scratch folder. `-o` holds the agent's final message.
- The brief names the PR, branch, lease owner, decisions, limits while the owner is away, validation and the final report.
- Send answers and new tasks with `resume`, so the agent keeps its context.

## Decisions

| Situation | Action |
| --- | --- |
| The plan lists an open decision | Take the plan's recommendation |
| A new important decision, owner present | Bring it to the owner, one at a time |
| Owner away | Choose the least added complexity that fits the plan and guides, and log it |
| Owner away or your usage is running low | Tell the agents to keep going and record their own choices |
| The owner reconsiders a decision | Pause the affected agent first, then discuss |

Being away never extends to merging, persistent system configuration or discarding work.

## Reporting

- Update the owner briefly and rarely; skip detailed decisions.
- Check an agent's results before passing them on. Say which you checked and which are the agent's report.
- A worthwhile prerequisite or follow-up becomes its own stacked PR.

## Log

Keep one log in its own draft PR: plans, PRs and agents; numbered decisions with choice and reason; dated events; a closing "Left for the owner" list.
