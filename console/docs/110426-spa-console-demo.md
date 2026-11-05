![Shared Console UI and SPA demo](110426-spa-console-demo.svg)

# Console demo in the project explorer

Created: 2026-11-04. Status: accepted design; implementation pending.

Add a Console section to the project explorer SPA using the production React
screens. A browser-owned demo client supplies an Activity lab capture walkthrough,
seeded history and downloadable examples. The SPA remains statically hosted.

## Accepted design

| Area | Decision |
| --- | --- |
| Source | Keep `console/frontend` in place as a private local package exporting TypeScript source. |
| Entries | `./ui` exports the existing `App` and loads scoped CSS; `./demo` exports `createDemoClient`. |
| Client | Infer the shared client shape from `ReturnType<typeof createClient>`; use one internal context. |
| Internals | Keep protobuf constructors, fixtures and simulation helpers inside the package. |
| Navigation | Hosts own hash observation and navigation. Shared helpers construct links under the supplied base path. |
| Shell | The SPA header contains Console; embedded `PageFrame` retains Run, History and workflow navigation. |
| Lifetime | One demo client survives SPA section navigation. Refresh and Reset restore the seeded demo. |
| Scenario | Activity lab with CPU monitoring, recording and the existing workflow/metrics panels. |
| History | Two successful examples with different settings and one startup failure; support comparison and Run again. |
| Artifacts | Seeded successful runs offer matching MCAP files and reports. New runs offer matching JSON reports; their MCAP downloads are unavailable. |

The package uses the existing React, TypeScript, Vite and protobuf dependencies.
Each host compiles the shared source in its own build. The production import graph
reaches `./ui`; the SPA additionally imports `./demo`.

## Production change boundary

| Location | Intended change |
| --- | --- |
| `frontend/src/App.tsx`, `History.tsx`, `Comparison.tsx` | Receive the supplied client through one internal context; retain existing screen and workflow logic. |
| `frontend/src/api.ts`, `Result.tsx` | Add `artifactUrl(runId, id)` to the existing client and use it with artifact availability. |
| `frontend/src/model.ts`, host entry points | Reuse route parsing/link helpers with a host-supplied hash and base path; route imperative actions through a small host callback. |
| `frontend/src/ui/PageFrame.tsx`, `tokens.css` | Add the embedded layout and scope existing styles beneath a Console root. |
| `frontend/package.json`, entry modules, build scripts | Expose the two source entries and prepare generated code before consumers compile. |
| Existing report serialization | Share the pure formatter internally if needed to preserve the production JSON report shape. |

Use local inferred types for component props and client wiring. Public exports are
the two entry values above; generated schemas and screen internals stay behind
them. Demo behavior belongs to the demo client and SPA wrapper.

## Runtime behavior

- Subscribe before publishing initial state. A returning screen receives the current status and enough retained stream data to rebuild its live panels.
- `live` remains subscribed through idle and completed states until aborted. Detaching a screen removes its subscription while the page-owned scenario continues.
- Use event callbacks for commands and state changes. Schedule finite sample events with named durations; reset/disposal cancels pending events and revokes report URLs.
- Preserve current prepare/start/stop behavior, including request-key reuse, stale reviews, cancellation during preparation and one active run. Gracefully stopping an accepted run follows the production Completed outcome.
- Fixtures use generated protobuf messages, valid run identifiers and consistent timestamps, counters, comparison facts and recording metadata. Mark the SPA experience as an interactive demo.
- Keep seeded history stable. Visitor-created runs are memory-only; a cleared or unknown run link offers a route back to History. Recording-disabled selections produce recording-disabled outcomes.

SPA routes: `#/console`, `#/console/history`, `#/console/history/<id>` and
`#/console/history/compare/<a>/<b>`. Standalone URLs retain their existing forms.

Before implementing the scheduler, agree the user-visible run duration and
history-retention limit. Define finite event/report byte bounds and reuse existing
protocol bounds where applicable. Give every fixed value a descriptive name.

## Milestone 1: Source package and reproducible builds

**Work items**
- [ ] Add `./ui` and `./demo` source exports to the existing private package; have both hosts consume `./ui`.
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
- [ ] Inject the existing client shape once; replace singleton-client use in App, History and Comparison with the internal context.
- [ ] Move hash observation into host entry points. Pass current hash/base path and the host navigation callback; adapt existing parsers, links and Run again actions.
- [ ] Add embedded `PageFrame` behavior and scoped styles, preserving the standalone shell, focus behavior and semantic landmarks.
- [ ] Add `artifactUrl` to `createClient`, retaining production endpoint generation. Require both artifact availability and a usable URL before rendering a download link.

**Validation**
- [ ] Existing Console tests pass for prepare, run, stop, history, comparison and artifact links; add focused contract checks in that existing suite.
- [ ] Changing the injected client cleans up the prior subscription; unmount/remount and React StrictMode do not duplicate active subscriptions.
- [ ] Standalone session establishment and its original URLs still work. No session request is issued by importing the shared UI.
- [ ] Host-prefixed links, comparison selection, Run again and browser back/forward preserve the correct Console view.
- [ ] Review standalone and embedded rendering at desktop/mobile widths; check scoped typography, controls, focus and absence of nested main landmarks.

Evidence: pending implementation.

## Milestone 3: Interactive capture client

**Work items**
- [ ] Implement `createDemoClient` with the inferred production client shape, generated message constructors and internal scenario state.
- [ ] Support prepare, start, stop, current, history, historyRun, historyCompare and live. Model the configured sample project's real options, including live-only versus recording.
- [ ] Seed two successful captures and one startup failure. Supply consistent Review/History comparison messages using existing pure helpers where browser-compatible.
- [ ] Deliver workflow statuses and real-format CPU telemetry packets through the existing UI models and reducers; use deterministic sample data.
- [ ] Implement bounded run/event retention and finite scheduled progression. Provide inferred reset/disposal helpers for the SPA owner; retain state across screen subscriptions.

**Validation**
- [ ] Existing Console test infrastructure covers successful completion, early Stop, preparation cancellation, duplicate Start, stale review, recording disabled and Run again.
- [ ] Filters, paging, run lookup and comparisons agree with the same stored run facts; missing/expired IDs return the existing Missing error behavior.
- [ ] Abort stops delivery to that subscriber; re-entry reconstructs current status and live panels without restarting or duplicating the run.
- [ ] Reset restores fixtures, cancels pending work and prevents old callbacks from modifying new state. Retention and payload limits have observable boundary checks.
- [ ] The demo completes the walkthrough with all Console-server network requests blocked.

Evidence: pending implementation.

## Milestone 4: Example files and per-run reports

**Work items**
- [ ] Bundle small, valid MCAP files and matching reports for seeded successful runs; document reproducible fixture generation using existing repository tooling.
- [ ] Keep seeded statuses, schema/channel data, sample timestamps, recording sizes and artifact metadata aligned with those files.
- [ ] Generate new-run JSON reports from terminal outcomes using the production report format. Return object URLs through `artifactUrl` and revoke them on reset, eviction and disposal.
- [ ] Keep new-run MCAP URLs unavailable. Show recording-disabled selections and unavailable startup-failure artifacts using existing states.

**Validation**
- [ ] Decode bundled MCAP examples with existing reader tooling and check schemas, samples, duration and exact file sizes against fixture metadata.
- [ ] Compare JSON report fields with production-format expectations for completion, early Stop, startup failure where a report exists, and disabled recording.
- [ ] Verify download filenames, media types and availability on current results and History; unavailable artifacts create no broken links.
- [ ] Verify sample URLs under `/` and `/doramcap/`; confirm reset/eviction/disposal releases only URLs whose artifacts are no longer available.

Evidence: pending implementation.

## Milestone 5: SPA section and integration checks

**Work items**
- [ ] Add Console navigation and lazy-load the local package entries on first use. Keep the demo client at SPA lifetime while Console views may unmount.
- [ ] Add the interactive-demo label and Reset action to the SPA wrapper; Reset also returns navigation to the initial Console view.
- [ ] Connect host hash/base-path inputs, unknown-run handling, and embedded layout. Preserve other SPA sections, selection URLs and static hosting.
- [ ] Update `scripts/ci/test-node.mts` to build the SPA explicitly after shared prerequisites; it currently skips packages without a test script.
- [ ] Update minimal build/hosting documentation and record the latest validation in this milestone.

**Validation**
- [ ] Console frontend tests/build and the SPA production build/schema validation pass on the final revision; run report/server checks if their serialization code changed.
- [ ] Walk through prepare, profile changes, start, early Stop, completion, History, comparison, Run again, downloads and Reset in a local browser preview.
- [ ] Navigate to another SPA section during a run and return; verify continued progression, retained history and restored live state. Refresh restores seeds and handles a visitor-created deep link clearly.
- [ ] Check direct URLs, browser history, keyboard use, responsive layout and static hosting at both root and repository subpaths. Refresh the local preview after final visual changes.
- [ ] Verify production Console against its real server for session entry, review, live updates and downloads; elevate only capability-requiring test executables under repository rules.
- [ ] At PR integration, verify x86 and ARM64 Node CI and submit the repository's required ARM64 validation unless waived. Record local, ARM64 and blocked checks separately.

Keep new automated behavior checks in the existing Console suite. SPA verification
uses its production build, schema validation and local visual checks. Full local
Rust CI is reserved for an integration point or changes that warrant it under
[AGENTS.md](../../AGENTS.md).

Evidence: pending implementation.

## Plan validation

- Documentation: milestone structure, local links and whitespace checks passed; SVG parsed, rendered and visually reviewed.
- Application checks and implementation milestones: pending implementation.
