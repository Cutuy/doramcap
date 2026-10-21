# Build and hosting

```sh
npm --prefix apps/project-explorer/frontend ci
npm --prefix apps/project-explorer/frontend run build
npm --prefix apps/project-explorer/frontend run preview
```

`frontend/dist/` is self-contained. Serve it with any static HTTP server. Building
uses `cargo metadata --offline --no-deps`; it does not compile Rust or require
system headers. Regenerate after editing repository documentation or manifests.

## GitHub Pages

Upload the contents of `frontend/dist/` as the Pages artifact using the repository’s
chosen deployment workflow. Vite emits relative asset paths, so the same build
works at `/` or `/doramcap/`. No server-side rewrite rules are required.
Publication is separate from building; this app does not deploy automatically.

Hash routes support refresh, browser history, copied links and new tabs:

- `#/apps?category=Observability&q=activity`
- `#/crates/activity-tracing`
- `#/dependencies?focus=apps/provenance-demo`
- `#/overview?useCase=perception`
- `#/overview?capability=recording`
- `#/overview?integration=mcap`
- `#/docs?path=docs/repository-layout.md&heading=component-ownership`

The overview is the default route. Capability and integration links retain the
selected use case. Dependencies use `focus` and `category`; the explorer shows full transitive chains. `build=1` and `dev=1` include
those declaration types. Component sidebars remain direct. Component documentation headings use `heading` in the query string.

## Validation

```sh
npm --prefix apps/project-explorer/frontend test
npm --prefix apps/project-explorer/frontend run build
cd apps/project-explorer/frontend
npx playwright install chromium
npm run test:browser
```

Browser tests serve the production build at a project subpath and check deep links,
refresh/history, documentation assets, overview mappings, direct dependency links and mobile layout.

[Recorded validation](validation.md).
