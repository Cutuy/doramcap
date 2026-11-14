---
name: orchestrator
description: Run Codex CLI agents in parallel, one per plan PR; answer their questions for the owner, verify their results and keep a decision log.
---

# Orchestrator

Follow [AGENTS.md](../../../AGENTS.md) and [worktree-pool](../worktree-pool/SKILL.md). Requires `codex` and `gh`.
You act for the owner: the agents ask you, and you ask the owner.

## Launch

- Give each plan PR one agent. Fill [brief.md](brief.md) for each and keep briefs, answers and final messages in your scratch directory.
- Use the model and effort the owner names; the last choice was `gpt-6.1-sol` at `xhigh`. Run each agent in the background with full access.
- Record each agent's `session id` from the top of its output.
- Each agent takes its own lease and opens a new draft PR based on its plan branch. Take a lease yourself only for your own edits.
- Cap each agent's parallel build jobs so all of them fit on the machine.

```sh
codex exec -m MODEL -c model_reasoning_effort=EFFORT --dangerously-bypass-approvals-and-sandbox \
  -C REPO -o last-NAME.md - < brief-NAME.md > log-NAME.txt 2>&1          # start
codex exec resume -m MODEL -c model_reasoning_effort=EFFORT --dangerously-bypass-approvals-and-sandbox \
  -o last-NAME.md SESSION_ID - < followup-NAME.md > log-NAME.txt 2>&1    # answer or redirect, from REPO
```

## Decide

| Situation | Action |
| --- | --- |
| The plan lists an open decision | Answer it in the brief with the plan's recommendation |
| An agent ends with `QUESTION`, owner present | Take important decisions to the owner as AGENTS.md requires; answer routine ones yourself |
| Owner away | Choose the least added complexity that fits the plan and guides, answer, and log it |
| Owner away or your usage is running low | Switch every agent to keep-going mode: no more questions, own choices recorded in the PR description |
| A probe contradicts a decided option | Reopen it; log the evidence |

Being away never extends to merging, new persistent system configuration or discarding work.

## Steer

- Send follow-ups to the session that holds the context. One agent writes to a branch at a time.
- To change instructions mid-task, stop the background process, then resume the session with the update. Work in its leased checkout survives.
- When the owner reconsiders, stop the affected agent first, then discuss.
- A useful prerequisite becomes its own PR with the dependent PR stacked on it. Combine sibling work in a stacked integration PR by cherry-pick.
- After the owner merges a plan, rebase its implementation PR onto main and restack every PR above it. Confirm each new tree differs from the old one only by what main added.

## Verify

An agent's final message is a report. Before passing it on:

- Check the PR head, base, draft state and the diff for the claimed change.
- Read GitHub CI and the ARM64 job result yourself.
- Confirm each review-bot finding against the diff, then send real ones to the agent.
- Look at screenshots for interface changes.
- Tell the owner which results you checked and which are the agent's report.

## Log

- Keep one log document in its own draft PR: a setup table of plans, PRs and session IDs; a numbered decision table with choice and reason; dated events; a closing "Left for the owner" list.
- Log each decision when you make it, and each withdrawn decision in place.
- Update the owner briefly and rarely; the log holds the detail.

## Pitfalls

- A lease refuses a branch another checkout holds. Use a new local branch and `git push origin HEAD:REMOTE_BRANCH`.
- Release can fail right after a push while GitHub catches up. Retry it.
- Lease output and `state.json` contain tokens. Read your token into a variable; print neither.
- When `gh pr edit` fails on the Projects query, update the body with `gh api -X PATCH repos/OWNER/REPO/pulls/N -F body=@FILE`.
