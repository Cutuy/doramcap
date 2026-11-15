# Console server source layout implementation

Created November 15, 2026. [Architecture](111526-server-source-layout.md).

## Milestone 1: Group modules by responsibility

**Work items**
- [x] Move modules into deployment, run, records and HTTP groups.
- [x] Place native request builders and image handling under run/backends.
- [x] Update direct imports, dynamic imports and release tooling.
- [x] Link the source layout from the server README.

**Validation**
- [x] TypeScript compilation.
- [ ] Production build and compiled imports.
- [ ] Server and browser/server contract tests.
- [ ] Scanner review and import equivalence.
