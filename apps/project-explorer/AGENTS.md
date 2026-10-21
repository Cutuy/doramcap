# SPA design and implementation

- Keep the presentation professional and restrained; avoid emojis and decorative clutter.
- Use short, natural labels: Compute, Platform, Data, Runtime, Embodiments, Existing middleware, Dora IPC, MCAP, OpenTelemetry. Avoid redundant prefixes and slash-separated synonyms.
- Remove explanations that repeat headings, links, or obvious interactions. Use one useful description; omit secondary caveats, disclaimers, graph captions, and repeated card calls to action.
- Link to each app/crate once per detail panel. Its page already contains its README; omit duplicate documentation/evidence links and audit links from the overview.
- Use small inline category glyphs before text. Keep them visible; avoid large icon tiles.
- Present doramcap as selectable foundation components alongside existing middleware. Use a narrow vertical middleware label on desktop and a compact horizontal label on mobile.
- Frame the complete platform stack. Keep application, platform, and compute layers compact; give doramcap the main visual space.
- Open the overview with one page title, followed by “Explore use cases” and the tabs. Omit extra introductory copy and links, and omit “Illustrative use case” from the detail panel.
- Keep use-case selectors text-only, default to the first case, and place its detail panel immediately before the stack. Use color for highlights, without underlines.
- Keep Apps and Crates in separate galleries. Display readable names such as “My Name” for `my-name`, preserving technical acronyms.
- On app/crate pages, show direct dependencies with an Explore link. In Dependencies, show transitive dependents above the highlighted selection and dependencies below; omit the declarations list.
- Preserve the original React Flow graph appearance, connection handles, arrows, layout, and controls. When asked to restore a design, recover the actual implementation from Git history instead of approximating it with a new renderer.
- Generate content from manifests, READMEs, and repository docs; serve architecture SVGs statically. Keep use-case mappings in canonical, schema-validated JSON with auditable references. Mark placeholders accurately without inventing integrations.
- Preserve URL-based navigation, selection, filters, refresh, and browser history. Support local hosting and GitHub Pages subpaths.
- Apply copy corrections consistently across the SPA. Review substantial visual changes before implementation; verify responsive layout and relevant behavior, then refresh the local preview.

- Keep verification lightweight: production build, schema validation, and local visual checks. Do not add a JavaScript test suite or Playwright tooling.
