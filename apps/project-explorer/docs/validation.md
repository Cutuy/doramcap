# Validation

Run `npm --prefix apps/project-explorer/frontend run build` for TypeScript,
JSON Schema and mapping-reference validation, and the production bundle.

Check navigation, deep links, dependency selection, and responsive layout in the
local preview. The SPA has no JavaScript test suite or Playwright tooling.

## Console overview panel

- TypeScript, overview schema/references, and Vite bundle pass with a temporary
  `libs/fault/dora/README.md` fixture, removed after the build. A clean build remains
  blocked by that existing missing README.
- Latest Console release and installer links resolve; published installer passes
  shell syntax validation. Local preview serves the built bundle.
- Desktop/mobile rendering and clipboard interaction remain pending: this session
  has no connected browser. Responsive styles and HTTP copy fallback are included.
