# Project explorer

A foundation overview and use-case guide, galleries for workspace apps and crates,
linked documentation, architecture SVGs, and direct Cargo dependencies. Content is generated from the
checkout’s manifests and READMEs.

![Architecture](docs/architecture.svg)

## Run

Requires Node.js 22+, npm, and the repository’s Rust toolchain. From the repository root:

```sh
npm --prefix apps/project-explorer/frontend ci
npm --prefix apps/project-explorer/frontend run dev
```

Open the local URL printed by Vite. See [build and hosting](docs/hosting.md)
[content conventions](docs/content.md), and [use-case mapping contract](docs/overview-mappings.md).

Planned: [experiment preparation and onboarding](server/docs/102726-experiment-preparation.md).
