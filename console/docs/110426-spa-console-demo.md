![Shared Console UI and SPA demo](110426-spa-console-demo.svg)

# Console demo in the project explorer

Created: 2026-11-04. Status: design agreed; implementation in progress.

Add a Console section to the project explorer SPA using the production React
screens. A browser-owned demo client supplies an Activity lab capture walkthrough,
one built-in run and downloadable examples. The SPA remains statically hosted.

Visitor flow: History starts with one built-in run. The visitor picks settings and
runs an experiment, compares any two runs and may keep creating runs. A page refresh
clears their runs and restores the built-in one.

## Accepted design

Agreed with the project owner on 2026-11-04.

| Area | Decision |
| --- | --- |
| Source | Keep `console/frontend` in place as a private local package exporting TypeScript source. |
| Entries | `./ui` exports the existing `App` and loads scoped CSS; `./demo` exports `createDemoClient`. |
| Client | Infer the shared client shape from `ReturnType<typeof createClient>`; use one internal context for the client and base path. |
| Internals | Keep protobuf constructors, fixtures and simulation helpers inside the package. |
| Navigation | `App` keeps its hash listener and hash writes and takes one `basePath` prop; link helpers build under it. Standalone passes an empty base. |
| Shell | The SPA header contains Console; embedded `PageFrame` retains Run, History and workflow navigation. |
| Lifetime | One demo client lives as long as the page, so a run continues during SPA navigation. Refresh and Reset restore the built-in run. |
| Reset | Reset remounts `App` with a new `key`; the client stays fixed for each mount. |
| Scenario | Activity lab with CPU monitoring, recording and the existing workflow/metrics panels. |
| History | One built-in successful run; any two runs compare; Run again works from any completed run. |
| Comparison | A small diff inside the demo turns per-run named facts into real comparison rows. The server is unchanged. |
| Artifacts | The built-in run offers its matching MCAP file and report. Visitor runs offer a matching JSON report and report no recording file or size. |
| Theme | Console stays light inside the SPA for now. |
| Limits | Runs last 30 s unless stopped. Visitor runs have no count limit; refresh clears them. |

The package uses the existing React, TypeScript, Vite and protobuf dependencies.
Each host compiles the shared source in its own build. The production import graph
reaches `./ui`; the SPA additionally imports `./demo`.

## Decisions awaiting owner review

Made during implementation while the owner was away. Each stays easy to revise.

| Topic | Decision | Reason |
| --- | --- | --- |
| Startup failure | No failure scenario; drop its checks. | Follows from one built-in run; visitors see no failed run. |
| Report links | Visitor reports use `data:` URLs. | Nothing to create or revoke, which removes reset and disposal cleanup. |
| `current()` | Keep it on the client. | The browser contract test and server route use it. |

## Production change boundary

| Location | Intended change |
| --- | --- |
| `frontend/src/App.tsx`, `History.tsx`, `Comparison.tsx` | Receive the supplied client and base path through one internal context; retain existing screen and workflow logic. |
| `frontend/src/api.ts`, `Result.tsx` | Add `artifactUrl(runId, id)` returning `string \| undefined`; links need availability and a URL, and carry a `download` filename. |
| `frontend/src/model.ts` | Route parsing and link helpers take the base path. |
| `frontend/src/ui/PageFrame.tsx`, `tokens.css` | Add the embedded layout, scope styles beneath a Console root, reset SPA globals that reach it and pin the light color scheme. |
| `frontend/package.json`, entry modules, build scripts | Expose the two source entries and prepare generated code before consumers compile. |

Use local inferred types for component props and client wiring. Public exports are
the two entry values above; generated schemas and screen internals stay behind
them. Demo behavior belongs to the demo client and SPA wrapper.

## Runtime behavior

- Subscribe before publishing initial state. A returning screen receives the current status and replays the current run's frames to rebuild its live panels; CPU dictionary frames are always kept.
- `live` remains subscribed through idle and completed states until aborted. Detaching a screen removes its subscription while the page-owned scenario continues.
- Use event callbacks for commands and state changes. Schedule finite sample events with named durations; disposal cancels pending events.
- Preserve current prepare/start/stop behavior, including request-key reuse, stale reviews, cancellation during preparation and one active run. Gracefully stopping an accepted run follows the production Completed outcome.
- Fixtures use generated protobuf messages, valid run identifiers and consistent timestamps, counters, comparison facts and recording metadata. Mark the SPA experience as an interactive demo.
- Keep the built-in run stable. Visitor-created runs are memory-only; a cleared or unknown run link offers the existing route back to History. Recording-disabled selections produce recording-disabled outcomes.

SPA routes: `#/console`, `#/console/history`, `#/console/history/<id>`,
`#/console/history/compare/<a>` and `#/console/history/compare/<a>/<b>`. Standalone
URLs retain their existing forms.

Give every fixed value a descriptive name.

## Milestone 1: Source package and reproducible builds

**Work items**
- [ ] Add `./ui` and `./demo` source exports to the existing private package; the SPA consumes them and standalone keeps its relative entry.
- [ ] Declare the SPA's local package dependency and update its lockfile. Make installation of Console dependencies and protobuf generation explicit in the clean-checkout workflow.
- [ ] Generate protocol code before either consumer type-checks or bundles. Preserve independent standalone and SPA build commands through a small explicit preparation step.
- [ ] Resolve React/ReactDOM to one instance per host bundle; retain CSS side effects and relative static-asset URLs.

**Validation**
- [ ] From a checkout without generated modules, install locked dependencies, then build both frontends successfully.
- [ ] Verify public entry imports work and ordinary package imports of unexported internals fail.
- [ ] Inspect bundle imports: production excludes demo fixtures/scheduling; the SPA contains no Node/native server modules.
- [ ] Confirm one React runtime per application and verify the dev server refreshes shared TS/CSS edits.

Evidence: pending implementation.

## Milestone 2: Shared UI integration

**Work items**
- [ ] Inject the client and base path once; replace singleton-client use in App, History and Comparison with the internal context.
- [ ] Give route and link helpers the base path, covering `PageFrame`, History, Comparison and the Run again hash write.
- [ ] Add embedded `PageFrame` behavior and scoped styles with a small reset for SPA globals, preserving the standalone shell, focus behavior and semantic landmarks. Pin the Console root to the light color scheme.
- [ ] Add `artifactUrl` to `createClient`, retaining production endpoint generation. Require both artifact availability and a URL before rendering a download link, and give links a `download` filename.

**Validation**
- [ ] Existing Console tests pass for prepare, run, stop, history, comparison and artifact links; add focused contract checks in that existing suite.
- [ ] Unmount/remount, a new `key` and React StrictMode do not duplicate active subscriptions.
- [ ] Standalone session establishment and its original URLs still work. No session request is issued by importing the shared UI.
- [ ] Host-prefixed links, comparison selection, Run again, leaving Console and browser back/forward preserve the correct view.
- [ ] Review standalone and embedded rendering at desktop/mobile widths; check SPA heading, paragraph, link and `.detail-grid` styles do not disturb Console, plus focus and absence of nested main landmarks.

Evidence: pending implementation.

## Milestone 3: Interactive capture client

**Work items**
- [ ] Implement `createDemoClient` with the inferred production client shape, generated message constructors and internal scenario state.
- [ ] Support prepare, start, stop, current, history, historyRun, historyCompare and live. Model the configured sample project's real options, including live-only versus recording.
- [ ] Seed one successful capture. Build Review and History comparison rows with a demo-local diff over per-run named facts.
- [ ] Deliver workflow statuses and real-format CPU telemetry packets through the existing UI models and reducers; use deterministic sample data.
- [ ] Run for 30 s through scheduled events, retain the current run's frames and keep state across screen subscriptions. Provide an inferred disposal helper for the SPA owner.

**Validation**
- [ ] Existing Console test infrastructure covers successful completion, early Stop, preparation cancellation, duplicate Start, stale review, recording disabled and Run again.
- [ ] Filters, paging, run lookup and comparisons agree with the same stored run facts; missing IDs return the existing Missing error behavior.
- [ ] Comparison rows cover Same, Changed and Unavailable, including Review's current-values-only machine condition.
- [ ] Abort stops delivery to that subscriber; re-entry reconstructs current status and live panels without restarting or duplicating the run.
- [ ] Disposal cancels pending work and prevents old callbacks from modifying new state.
- [ ] The demo completes the walkthrough with all Console-server network requests blocked.

Evidence: pending implementation.

## Milestone 4: Example files and per-run reports

**Work items**
- [ ] Bundle a small, valid MCAP file and matching report for the built-in run; document its reproducible generation.
- [ ] Keep the built-in status, schema/channel data, sample timestamps, recording size and artifact metadata aligned with those files.
- [ ] Generate visitor-run JSON reports from terminal outcomes using the production report fields, returned as `data:` URLs through `artifactUrl`.
- [ ] Report no file or size for visitor recordings. Show recording-disabled selections using existing states.

**Validation**
- [ ] Decode the bundled MCAP with existing reader tooling and check schemas, samples, duration and exact file size against fixture metadata; confirm it holds no host-identifying values.
- [ ] Compare JSON report fields with production-format expectations for completion, early Stop and disabled recording.
- [ ] Verify download filenames, media types and availability on current results and History; unavailable artifacts create no broken links or "left on host" text.
- [ ] Verify sample URLs under `/` and `/doramcap/`.

Evidence: pending implementation.

## Milestone 5: SPA section and integration checks

**Work items**
- [ ] Add Console navigation with its own highlight and lazy-load the local package entries on first use. Keep the demo client at page lifetime while Console views may unmount.
- [ ] Add the interactive-demo label, a link to the install page and a Reset action to the SPA wrapper; Reset also returns navigation to the initial Console view.
- [ ] Connect the base path, unknown-run handling and embedded layout. Preserve other SPA sections, selection URLs and static hosting.
- [ ] Update `scripts/ci/test-node.mts` to build the SPA explicitly after shared prerequisites; it currently skips packages without a test script.
- [ ] Update minimal build/hosting documentation and record the latest validation in this milestone.

**Validation**
- [ ] Console frontend tests/build and the SPA production build/schema validation pass on the final revision; run report/server checks if their serialization code changed.
- [ ] Walk through prepare, profile changes, start, early Stop, completion, History, comparison, Run again, downloads and Reset in a local browser preview.
- [ ] Navigate to another SPA section during a run and return; verify continued progression, retained history and restored live state. Refresh restores the built-in run and handles a visitor-created deep link clearly.
- [ ] Check direct URLs, browser history, keyboard use, responsive layout, the light Console in a dark SPA and static hosting at both root and repository subpaths. Refresh the local preview after final visual changes.
- [ ] Review how "Recording: Saved" beside "cpu recording unavailable" reads, and whether 30 s feels right.
- [ ] Verify production Console against its real server for session entry, review, live updates and downloads; elevate only capability-requiring test executables under repository rules.
- [ ] At PR integration, verify x86 and ARM64 Node CI and submit the repository's required ARM64 validation unless waived. Record local, ARM64 and blocked checks separately.

Keep new automated behavior checks in the existing Console suite. SPA verification
uses its production build, schema validation and local visual checks. Full local
Rust CI is reserved for an integration point or changes that warrant it under
[AGENTS.md](../../AGENTS.md).

Evidence: pending implementation.

## Plan validation

- Documentation: milestone structure, local links and whitespace checks passed; SVG parsed and labels updated.
- Application checks and implementation milestones: pending implementation.
