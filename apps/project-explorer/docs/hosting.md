# Build and hosting

```sh
npm --prefix apps/project-explorer/frontend ci
npm --prefix apps/project-explorer/frontend run build
npm --prefix apps/project-explorer/frontend run preview
```

`frontend/dist/` is self-contained. Serve it with any static HTTP server that
answers unknown paths with `index.html`; the gallery server and `vite preview` do.
Building uses `cargo metadata --offline --no-deps`; it does not compile Rust or require
system headers. Regenerate after editing repository documentation or manifests.

## GitHub Pages and subpaths

Assets use absolute paths. For a subpath, build with its base:

```sh
npm --prefix apps/project-explorer/frontend run build -- --base /doramcap/
```

The build also writes `404.html`, which Pages serves for deep links. Upload the
contents of `frontend/dist/` as the Pages artifact using the repository’s chosen
deployment workflow; this app does not deploy automatically.

Clean URLs support refresh, browser history, copied links and new tabs. Links in
the old `#/path` form redirect to the same route.

- `/apps?q=activity`
- `/crates/activity-tracing`
- `/dependencies?focus=apps/dataflow-tracing-demo`
- `/?useCase=perception`
- `/?capability=recording`
- `/?integration=mcap`
- `/?useCase=latency&view=2d`
- `/docs?path=docs/repository-layout.md&heading=component-ownership`

The overview is the default route. Capability and integration links retain the
selected use case. Dependencies use `focus`; the explorer shows full transitive chains. `build=1` and `dev=1` include
those declaration types. Component sidebars remain direct. Component documentation headings use `heading` in the query string.

## Validation

```sh
npm --prefix apps/project-explorer/frontend run build
```

The build validates TypeScript and canonical mappings. Check deep links,
refresh/history, documentation assets, and mobile layout in the local preview.

[Recorded validation](validation.md).

## On-demand apps

See the [gallery hosting server guide](../server/docs/hosting.md) for local launch,
sessions, recording retention and recovery.
