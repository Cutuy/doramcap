# Dora coordinator client implementation

Created: 2026-11-09. [Architecture](110926-coordinator-client.md).
Approved migration: [PR #210](https://github.com/Cutuy/doramcap/pull/210).

Use locked Dora 1.1.0 message types and tokio-tungstenite 0.29.0 on the caller's
Tokio runtime. A request temporarily owns the socket; cancellation drops it while
client-owned graph state remains. No receive task or executor context registry is
needed for the pinned control endpoint. Backend migration prepares the supported
dynamic-node descriptor and sends Start directly; CLI reuse and upstream changes
are optional approaches if a demonstrated parity gap requires them.

## Milestone 1: Shared protocol client

Evidence (2026-11-09): focused x86 WebSocket tests, strict Clippy and dependency
boundaries passed using the CI profile. The real Dora 1.1.0 coordinator/daemon test
passed registration, inspection, graph stop, Destroy and native process exit.
Backend migration, full CI and ARM64 qualification remain pending.

**Work items**

- [x] Add `dora-control` with explicit endpoint/token inputs and typed operations.
- [x] Keep Start intent and UUID through cancellation; reject duplicate starts.
- [x] Match reply IDs/types, negotiate version and never replay failed mutations.
- [x] Stop only a uniquely identified owned graph before Destroy.
- [x] Add the approved bounded startup registration sampling and its replacement TODO.

**Validation**

- [x] Exercise request/reply frames over actual WebSocket connections, including Ping/Pong and explicit authentication.
- [x] Exercise silent handshake/stop, disconnect, invalid replies and incompatible versions.
- [x] Exercise cancellation before/after UUID receipt and ambiguous/foreign graph identities.
- [x] Complete focused checks and library-boundary review on the published source revision.
- [x] Exercise the typed exchanges against a real Dora 1.1.0 coordinator and verify native process exit.

## Milestone 2: Backend migration and CLI parity

The [systemd architecture](../../workload-lifecycle/docs/110726-launch-forms.md#systemd-mode)
supports dynamic nodes launched by the backend. Console expands and merges those
descriptors before execution. The migration must preserve that input, runtime-visible
paths, session identity and required artifacts. General CLI cached-build/Hub
preparation has not been established as a dependency of this supported path.

Dora's Start request accepts an optional build ID and an explicit session ID. The
real-coordinator test already starts a dynamic graph with a new session and no
build ID, using direct requests. Keep CLI behavior as a parity reference; no new
CLI dependency or upstream extraction is required merely to send Start.

**Work items**

- [ ] Preserve supported dynamic-node descriptor preparation, native execution contexts, session identity and required artifacts before replacing registration.
- [ ] Replace Compose and systemd control calls; inject shared references and retain backend cleanup ownership.
- [ ] Pass one absolute startup control deadline and a fresh shutdown control deadline after cancellation.
- [ ] Continue native teardown and evidence reporting after control errors.
- [ ] Remove unused transient control processes, files and CLI-output parsing.

**Validation**

- [ ] Compare session artifacts and prepared descriptors with existing CLI behavior.
- [ ] Run each backend with real Dora startup, inspection, stop and failed startup.
- [ ] Cancel during startup and hang/disconnect the coordinator; verify actual process cleanup and evidence retention.
- [ ] Verify concurrent repositories remain isolated and control launches no transient Dora CLI processes.

## Milestone 3: Integration qualification

Cloud validation is tracked in [PR #251 checks](https://github.com/Cutuy/doramcap/pull/251/checks).
Full local CI and the ARM64 worker remain deferred to backend integration.

**Work items**

- [x] Publish the client foundation in draft [PR #251](https://github.com/Cutuy/doramcap/pull/251), separately from the plan.
- [ ] Update runtime documents after backend adoption.
- [ ] Finish Scanner review and record current validation gaps.

**Validation**

- [ ] Run cloud CI and the required final-source local checks.
- [ ] Run ARM64; record native backend and real-Dora results separately.

## TODO: Replace startup readiness polling

Owner: `dora-control` integration. The initial migration retains this follow-up.

**Work items**

- [ ] Add/adopt an upstream wait-for-daemon operation or readiness subscription.
- [ ] Replace timed daemon-registration checks in both backends and remove `DAEMON_REGISTRATION_CHECK_INTERVAL` and its approved polling exception.

**Validation**

- [ ] Cover registration before/after subscription, disconnect, cancellation and deadline expiry.
- [ ] Verify both backends preserve startup ordering and native teardown without readiness polling.
