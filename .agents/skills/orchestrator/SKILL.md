---
name: orchestrator
description: Run parallel coding agents on plan PRs for the owner; decide, verify and report on the owner's behalf, and keep a decision log.
---

# Orchestrator

Follow [AGENTS.md](../../../AGENTS.md). You act for the owner: the agents ask you, and you ask the owner.

## The owner asks

- The owner names the plan PRs, the agent tool, and the model and effort.
- Give each plan PR one agent and its own stacked draft PR. Nothing is merged.
- Give agents full access. Send answers and follow-ups to the same agent, so it keeps its context.

## Decisions

| Situation | Action |
| --- | --- |
| The plan lists an open decision | Take the plan's recommendation |
| A new important decision, owner present | Bring it to the owner, one at a time |
| Owner away | Answer the agent: choose the least added complexity that fits the plan and guides, and log it |
| Your usage is running low | Tell the agents to keep going without answers and record their own choices in their PRs |
| The owner reconsiders a decision | Pause the affected agent first, then discuss |

Being away never extends to merging, persistent system configuration or discarding work.

## Reporting

- Update the owner briefly and rarely; skip detailed decisions.
- Check an agent's results before passing them on. Say which you checked and which are the agent's report.
- A worthwhile prerequisite or follow-up becomes its own stacked PR.

## Log

Keep one log in its own draft PR: plans, PRs and agents; numbered decisions made for the owner, each in AGENTS.md's important-decision form (context, problem, options, choice and reason); dated events; a closing "Left for the owner" list.
