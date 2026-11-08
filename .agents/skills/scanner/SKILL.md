---
name: scanner
description: Review a doramcap PR or branch against its implementation and design guides, and reconcile current AGENTS.md additions into those guides.
---

# Scanner

Use [implementation rules](../../../docs/impl-guide.md) and the
[design guide](../../../docs/design-guide.md) as the repository baseline.
Keep findings concise and evidence-backed.

## Establish scope

- Read current governing `AGENTS.md` files, including ancestor/nested instructions, and both guides. Preserve directory scope and instruction precedence.
- Resolve the requested PR's base/head or the branch and its comparison base; default to the repository's default branch when no base is supplied. Record commit IDs and merge-base; inspect the diff plus affected callers, contracts, and tests. Honor requested broader scope. For a PR, attach it to the chat when supported.
- Record relevant local edits separately. Compare target policy edits with the governing baseline; report weakened or conflicting rules explicitly. Do not silently use a PR's changed rules to excuse its code.

## Reconcile instructions

- Before scanning, follow [worktree-pool](../worktree-pool/SKILL.md) before guide/report edits. Absorb clear new root `AGENTS.md` obligations into the implementation guide and principles/preferences into the design guide; leave the entry-point links in `AGENTS.md`. Keep nested rules scoped to their directories.
- Preserve force, exceptions, scope, and existing guide IDs. Consolidate duplicates and remove absorbed root text. Unchanged sources produce no edits. Do not infer requirements from code or findings.
- Ask and wait before resolving conflicting policy or changing intent; continue independent review. Report the actual reconciliation diff and source. If reconciliation is explicitly prohibited, report drift without editing.

## Review and report

- Map every applicable guide item to the changed behavior and affected paths. Trace inputs, outputs, errors, cancellation, and cross-component ownership; names/comments alone are insufficient evidence.
- Keep a compact local report at the established `requirements-checklist.md` location or the user's requested path. Record target/base and guide revisions, reconciliation/local edits, inspected paths, exclusions, and remaining coverage; reuse it on later passes.
- Track each guide ID as **met**, **unmet**, **unverified**, **needs clarification**, or **not applicable**, with evidence or a reason. Split partially satisfied obligations. Recheck affected items when code or rules change; reading every file alone does not prove compliance.

| Guide ID | Status | Evidence / gap |
| --- | --- | --- |

- Lead with consequential findings: trigger, expected/actual behavior, impact, and file/line. Separate rule violations, design suggestions, and validation gaps. Preferences alone are not violations; missing approval evidence is unverified, not proof of historical noncompliance.
- Use static evidence and focused checks where useful; prefer existing `scripts/ci` tooling for mechanical checks. A review alone does not authorize broad CI, worker jobs, code fixes, commits, publication, or integration. Report required but unrun checks as gaps.
- Finish with coverage, checks run, unresolved questions, and a link to the local report. Mark partial scans explicitly; a clean result is valid. Keep only current evidence and open gaps.
