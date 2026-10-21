# Validation

Validated on 2026-10-21:

- Production Vite build and TypeScript checks passed.
- Nineteen data and schema tests passed, including negative mutations for invalid
  mappings, evidence references, duplicate IDs and capability membership.
- Ten Chromium tests passed against production output at `/doramcap/`:
  deep links, refresh/history, new tabs, Markdown/SVGs, anchors, direct sidebar dependencies and full transitive DAGs, overview selection, local JSON/schema audit and mobile overflow.
- DAG desktop layout inspected; mobile graph scrolling stays within the page.
- Compact overview and software layers inspected at desktop and mobile widths.
- Browser checks cover text-only selectors, detail-before-stack order, and Linux/CUDA deep links.
- Generated repository-link report: zero missing targets.
- ARM64 Pueue job **127** passed the repository suite. The native suite passed for the dependency DAG snapshot.

Overview mappings are illustrative compositions with explicit evidence links.
Schema validation checks structure and referential integrity; it does not establish
end-to-end integration or platform qualification. Publication is a separate step.
