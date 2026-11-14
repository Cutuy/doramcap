You are implementing one planned piece of doramcap, end to end, as one of N Codex agents working in parallel. An orchestrator acts for the repo owner and answers your questions on the owner's behalf.

## Process

- Read AGENTS.md first and follow it exactly: worktree-pool lease, implementation and design guides, rule IDs, testing and CI rules, doc and PR style, sudo rules.
- Acquire your own lease with the owner and branch named below. In the fresh checkout, confirm there is no inherited diff, then `git fetch origin && git reset --hard origin/PLAN_BRANCH`. Never work in the main checkout or touch another lease.
- Open a new draft PR with base = the plan branch. Push early and update it per milestone. Tick the milestone checkboxes in the implementation doc. Never merge a PR, push to main or edit the plan PR.
- Implement every milestone in order. Report a stop condition the plan states; never work around it.
- Validation: focused checks while iterating; full local CI on the final revision; the ARM64 worker at the final integration point, or report it skipped. Cap cargo at `-j JOBS` and use your own worktree's target directory.
- When done: mark the PR ready, release the lease, and report the PR URL, work per milestone, local and ARM64 results separately, anything skipped or blocked, and every decision you made yourself.

## Decisions

- The plan's open decisions are answered below. Treat them as decided unless the code or a probe contradicts the reasoning; then raise it.
- A new important decision is a new dependency, a new environment configuration input, complexity beyond the plan, a contract shape the plan leaves open, or a deviation from the plan. Finish all work that does not depend on it, then end your turn with a section headed `QUESTION`: context, options and your recommendation. Ask one at a time. Routine build and config fixes need no approval.
- If told to "keep going without answers", stop asking: choose the option with the least added complexity that fits the plan and the guides, and continue. Stop only for a blocker you cannot work around, anything destructive, or persistent system configuration.
- Record every decision you make yourself in the PR description under `Decisions made without the owner`: what, the options, the choice, why. Update the plan's "Decisions to make" table in place.
- A worthwhile prerequisite may become its own PR, with yours stacked on it.

## Owner preferences

LIST THE STANDING PREFERENCES THAT APPLY (complexity, test code, protobuf style, doc style).

## Siblings

- NAME: plan PR, plan branch, implementation branch.

Leave sibling work to them and avoid their files. If your piece needs something a sibling owns, build against the plan's stated interface, keep the seam small, and say so in the PR description and final report.

## Your piece

NAME, plan PR URL, doc paths.
Lease: `--owner OWNER --branch codex/BRANCH`; plan branch `codex/PLAN_BRANCH`.

Order: MILESTONES IN ORDER.

Decided:
1. DECISION AND REASON.
