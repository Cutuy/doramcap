# SPA design and implementation

- Keep explorer pages professional and restrained; avoid emojis and decorative clutter. The overview landing is the brand showcase and may use cinematic 3D and motion.
- Use short, natural labels: Compute, Platform, Data, Runtime, Embodiments, Existing middleware, Dora IPC, MCAP, OpenTelemetry. Avoid redundant prefixes and slash-separated synonyms.
- Remove explanations that repeat headings, links, or obvious interactions. Use one useful description; omit secondary caveats, disclaimers, graph captions, and repeated card calls to action.
- Link to each app/crate once per detail panel. Its page already contains its README; omit duplicate documentation/evidence links and audit links from the overview.
- Use small inline category glyphs before text. Keep them visible; avoid large icon tiles.
- The overview is a dark landing: a pinned Three.js platform stack with a scroll-driven story (hero, then one chapter per use case), followed by foundation, platform and workspace sections. Give doramcap the main visual space.
- Scrolling drives chapters; the chapter rail and `useCase` deep links jump to them. Prefer direct manipulation over plain buttons.
- Selecting a block opens its detail drawer; Escape, an empty-space click, a second click, browser back, or scrolling to another chapter closes it.
- Effects must not invent data: copy, counts and flows come from `overview.json` and the catalog.
- Keep the 2D schematic as the WebGL fallback and `view=2d`, and honor reduced motion.
- Keep Apps and Crates in separate galleries. Display readable names such as “My Name” for `my-name`, preserving technical acronyms.
- On app/crate pages, show direct dependencies with an Explore link. In Dependencies, show transitive dependents above the highlighted selection and dependencies below; omit the declarations list.
- Preserve the original React Flow graph appearance, connection handles, arrows, layout, and controls. When asked to restore a design, recover the actual implementation from Git history instead of approximating it with a new renderer.
- Generate content from manifests, READMEs, and repository docs; serve architecture SVGs statically. Keep use-case mappings in canonical, schema-validated JSON with auditable references. Mark placeholders accurately without inventing integrations.
- Preserve URL-based navigation, selection, filters, refresh, and browser history. Support local hosting and GitHub Pages subpaths.
- Apply copy corrections consistently across the SPA. Review substantial visual changes before implementation; verify responsive layout and relevant behavior, then refresh the local preview.

- Keep verification lightweight: production build, schema validation, and local visual checks. Do not add a JavaScript test suite or Playwright tooling.
