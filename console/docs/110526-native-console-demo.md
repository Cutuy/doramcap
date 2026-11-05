![Project explorer composes Console](110526-native-console-demo.svg)

# Native Console demo

Created: 2026-11-05. Follows [the Console demo plan](110426-spa-console-demo.md).

Make the Console section read as part of the project explorer, and give the real
Console a matching light and dark look.

## Accepted design

Agreed with the project owner on 2026-11-05.

| Area | Decision |
| --- | --- |
| Composition | `App` takes a layout prop and keeps its run state. The site places Run, History and Reset on the page. |
| Theming | Named `--console-*` tokens; the site maps its theme to them in one CSS block. |
| Light and dark | `light-dark()` follows the inherited `color-scheme`, so the site's switch reaches the demo. |
| Look | The site's colours, font, corners, padding and button sizes become Console's defaults. |
| Standalone theme | Follows the browser or OS setting, with no switch. |

## Milestone 1: site content

**Work items**

- [x] Move the Console tab after Overview.
- [x] Replace the demo bar with a standard page heading; keep Reset demo.
- [x] Show Upcoming features as a section below the demo; remove its sub-route.
- [x] Rewrite the home panel: where Console runs, the SSH tunnel, install steps, Try Console.

**Validation**

- [x] Production build of the project explorer.
- [x] Visual check: home at desktop and phone width, Console at desktop width.

## Milestone 2: Console tokens

**Work items**

- [ ] Replace hardcoded colours in styles and charts with `--console-*` tokens.
- [ ] Add tokens for font, corners, panel padding, heading weight and button size, with the site's values as defaults.
- [ ] Inherit `color-scheme`; the standalone page follows the browser or OS.
- [ ] Map the site's theme to the tokens in one block.

**Validation**

- [ ] Console frontend tests and build; project explorer build.
- [ ] Visual check of every Console phase, History and comparison, light and dark, in the demo.
- [ ] Standalone Console renders in light and dark.
- [ ] Touch targets on phone width.

## Milestone 3: layout prop

**Work items**

- [ ] Add a frame prop to `App` with tab data, phase rail and screens; the current layout becomes the default frame.
- [ ] Remove the embedded branch from `PageFrame`.
- [ ] Render Run, History and Reset on the page; keep the rail and screens in the window.

**Validation**

- [ ] Console frontend tests and build; project explorer build.
- [ ] Demo: tabs follow the URL, a run continues across tabs and sections, Reset works.
- [ ] Standalone Console layout is unchanged.
