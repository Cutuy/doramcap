---
name: sane-engineer
description: Audit a codebase against documented requirements. Use for requirements-driven reviews and implementation gap analysis.
---

# Sane Engineer

Read the existing [engineering requirements](requirements.md) as the reusable
baseline, inspect the governing repository requirements and instructions, and trace
the audit checklist through code and validation. Record findings in a separate
checklist file.

## Read requirements without expanding them

Read the bundled `requirements.md`, the existing governing repository
`requirements.md`, and applicable `AGENTS.md` files, including inherited and nested
instructions for the components in scope. Respect instruction precedence and
directory scope. Use the user's specified requirements path or the established
repository location.

**Keep `requirements.md` read-only unless the user explicitly requests creating or
updating it.** Invoking this skill or requesting a review does not authorize adding,
rewriting, removing or automatically absorbing requirements from `AGENTS.md`, plans,
chat history or the bundled baseline. Apply governing instructions during the work
and cite relevant source documents in the checklist without promoting them into
new repository requirements.

If the repository requirements file is missing, use the bundled baseline for the
audit and state that product requirements may be unspecified. Do not create a
requirements file automatically or infer product requirements from implementation.
Record ambiguities and proposed requirement changes separately as observations;
ask for clarification only when needed to complete the requested review.

When the user explicitly requests requirement edits, follow the repository worktree
process and keep changes within that request. Preserve unchanged wording, order,
IDs, formatting and independently authored constraints. Retain source references;
record audit status and supersession history in the checklist. Unchanged sources
must leave `requirements.md` byte-for-byte unchanged. Report the actual edits, or
state that the requirements were left unchanged. This workflow does not authorize
changes to `AGENTS.md` or the skill itself.

## Apply governing engineering rules

Include applicable `AGENTS.md` obligations in the audit checklist with their own
source references. Keep product behavior, engineering preferences, and workflow or
approval requirements distinguishable. Preserve qualifiers such as “favor,” “when
practical,” and “unless waived”; a preference alone does not establish a violation.
For documented exceptions, record the rationale or user approval required by the
source. Missing evidence is **unverified**; ask only when it blocks the review or
an action that requires approval.

Assess implementation rules against the code in scope and workflow rules against
the current work and available history. Do not infer past approval failures from
code alone or treat new process rules as proof of historical noncompliance.
Read current repository instructions for worktree leases, integration, test
profiles, capability handling, CI coverage, worker submission and documentation
formats; keep repository-specific paths, hosts and commands in those sources.

Choose focused validation for incremental reviews. Run full CI and additional
architecture or hardware checks when governing instructions require them; report
each environment and skipped or blocked checks separately. For plan-driven work,
record completed and deferred validation evidence in the relevant milestone when
maintaining that plan is authorized. Share promising findings as provisional while
validation continues. An audit does not authorize dependency adoption, configuration
contract extensions, publication or other separately gated actions.

## Establish the concrete checklist

Read the existing governing requirements. If competing requirements files make scope
unclear, ask which governs while continuing independent inspection.

Before auditing the implementation, write the derived requirements to a separate,
persistent file. Default to `requirements-checklist.md` beside the source
`requirements.md`; honor a user-specified path or an established repository audit
location. Markdown is the default; use YAML when requested or already established.
Treat the checklist as a local working file: do not stage or commit it unless the
user explicitly requests that. Follow the repository worktree process before
creating or updating it.

Turn the applicable existing requirements into the checklist file:

- Give each requirement a stable ID and retain its source item or line reference.
- Split compound items into independently verifiable obligations, preserving their
  conditions, exceptions and scope.
- State the trigger/input, expected behavior or invariant, and observable acceptance
  criteria where applicable. Include failure behavior when the source specifies it.
- Distinguish explicit requirements from necessary interpretations. Mark ambiguous,
  conflicting or underspecified items for clarification; do not silently decide
  product policy or invent numeric thresholds.
- Keep concrete audit decomposition separate from the read-only `requirements.md`.
  The checklist must be a separate file, even when the final response summarizes it.
  Reuse and update that file on later passes rather than creating competing copies.
- Record the source path, reviewed revision when available, scope, concrete
  requirements, acceptance criteria, and source references in the checklist file.

Keep derived detail proportional to the source. A simple requirement can remain a
single checkable sentence. Implementation choices are evidence, not new requirements.

## Check piece by piece

Map the checklist to components, entry points, public contracts and relevant tests.
Identify cross-component requirements so checking individual modules does not miss
integration behavior. Cover every in-scope requirement; track remaining items
explicitly if the review spans multiple passes.

For each item:

1. Trace actual behavior from inputs and callers through implementation and outputs.
   Inspect relevant error paths and state transitions. Names and comments alone do
   not establish compliance.
2. Compare behavior with the acceptance criteria. For configuration constraints,
   check when validation occurs and whether invalid input fails before dependent
   side effects. Assess limits against the stated supported workload and exhaustion
   contract; a large constant alone is not evidence of a defect.
3. Inspect existing tests and run focused checks when useful. Reproduce a suspected
   mismatch where practical. Record what was executed separately from static evidence;
   passing unrelated tests does not verify an item.
4. Update each item in the checklist file with its status and concise evidence
   with code/test locations. Use **met**,
   **unmet**, **unverified**, or **needs clarification**. State any partially satisfied
   sub-obligations explicitly rather than passing the whole item.

A requirement is met only to the extent supported by the evidence. State validation
limits, including unavailable environments or untested integration paths. Separate
optional engineering suggestions from requirement failures and prioritize findings
by their demonstrated impact.

## Deliver the audit

Keep the audit results in the separate checklist file. For Markdown, use a compact
traceability table; YAML should retain equivalent fields:

| ID / source | Concrete requirement | Status | Evidence / gap |
| --- | --- | --- | --- |

Lead with consequential mismatches, including the triggering condition, expected
and actual behavior, and affected code. Follow with checklist coverage, validation
results and unresolved questions. Link the checklist file in the final response.
A fully satisfied checklist is a valid outcome.
Do not manufacture a refactor to make the review appear productive.

Review requests update only the checklist file and end with findings. Edit
`requirements.md` only when the user explicitly requests requirement changes.
If implementation fixes are requested, follow the repository's
worktree process, address the agreed requirements, and update relevant callers,
docs and behavior tests together. Preserve unrelated behavior and compatibility.
Recheck affected items and report remaining gaps. Commit, publish and integrate
only within the user's authorization.

For later passes, reuse requirement IDs and prior evidence. Inspect changes to both
the requirements and implementation since the recorded baseline; recheck affected
consumers and previously unresolved items. Record the reviewed revision and scope
in the existing audit location when maintaining a repository audit is requested.

## Optional Cargo dependency checks

When a requirement specifies crate dependency direction, prefer existing repository
checks. If the bundled checker is useful, read its
[contract](references/cargo-boundaries.md) and run
`scripts/check_boundaries.py` with explicit requirement-backed rules. It checks direct
edges and transitive workspace paths; semantic ownership still requires code review.
Use repository-owned tooling for CI. Dependency checks do not establish behavioral
compliance with the rest of the checklist.
