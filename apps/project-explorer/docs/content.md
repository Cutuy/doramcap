# Content conventions

The generator reads Cargo workspace members and README files under `apps/` and
`console/`. Library packages get individual crate pages; packages below
`apps/<name>/` or `console/` belong to that logical app. Frontend-only apps appear with no Cargo dependency edges.

- Titles humanize package names and preserve common acronyms.
- Summaries use the first nonempty README paragraph.
- `catalog.config.json` supplies category overrides by longest directory prefix.
  New components default to **Other** until categorized.
- Markdown and images under `apps/`, `libs/`, `docs/`, and `scripts/` are bundled
  from Git’s tracked and nonignored inventory. Agent instructions are excluded.
- Existing SVGs are served as image assets. Click a diagram to open it at full size.
- Relative Markdown and component links stay within the explorer. Source-code
  links open the repository at the generated catalog’s commit. Raw Markdown HTML
  is not executed.

The generated link report at `frontend/src/.generated/link-report.json` identifies
missing repository targets. Existing documentation issues are reported without
blocking the site build. Missing component READMEs fail the build.

Component pages list direct internal Cargo dependencies with an Explore link.
The Dependencies tab shows transitive dependents above the highlighted selection and dependencies below it;
shared nodes appear once. Optional/target-specific edges are dashed, and build/dev
edges are opt-in. Selecting a node opens its inspector and component link. The original React Flow
renderer provides connection handles, fitted layout, and pan/zoom controls. Legacy
depth and category parameters are ignored.

The overview uses the versioned [mapping contract](overview-mappings.md). The build
validates editorial mappings against the generated repository inventory.

Generated data and copied assets are ignored. Regenerate before development or
production builds; the frontend runs without a backend or runtime GitHub API calls.
