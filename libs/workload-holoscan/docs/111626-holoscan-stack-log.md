# Holoscan runner stack

Created November 16, 2026.
Plan: [#264](https://github.com/Cutuy/doramcap/pull/264).

Implement the seven runner milestones as stacked pull requests. The web app gets
its implementation plan after the application-only run works. Nothing is merged
by this task.

## Work

| Milestone | Scope | PR |
| --- | --- | --- |
| 1 | Real CLI, GPU container and overlay probes | Pending |
| 2 | Crate, protobuf contract and discovery | Pending |
| 3 | Prepare and Review | Pending |
| 4 | Build request | Pending |
| 5 | Application run | Pending |
| 6 | System apps | Pending |
| 7 | Recovery and acceptance | Pending |

## Agents

| Agent | Model | Assignment |
| --- | --- | --- |
| `holoscan_probe_audit` | GPT-6.1 Sol, xhigh | Runtime prerequisites and milestone 1 feasibility |
| `holoscan_contract_audit` | GPT-6.1 Sol, xhigh | Current backend interfaces and milestone dependencies |

## Decisions

The agreed decisions in #264 remain the baseline. No new design decision has
been made for the owner.

## Events

- November 16: read #264 at `635cd8fd` and both repository guides. Verified the
  listed shared prerequisites, including #364, in `origin/main` at `e3fc8911`.
- November 16: started two independent read-only audits. Acquired a separate
  worktree lease for this log and the local review report.

## Left for the owner

- New important choices will be raised individually, with options and a diagram.
