# Matched Dora runtime implementation

Created: 2026-11-10. [Architecture](111026-matched-runtime-versions.md).

## Milestone 1: Upgrade and qualify one resolved runtime

**Work items**

- [x] Centralize existing Dora requirements in workspace dependencies using `1.1`; retain disabled default features and current lockfile versions.
- [x] Require Hello's reported version to equal `dora_message::current_crate_version()` on every new connection.
- [x] Resolve the client's actual protocol dependency through locked, all-feature Cargo metadata; reject mixed resolved protocol versions.
- [x] Select the matching official Linux amd64/arm64 asset. Require its reviewed SHA-256 committed in `scripts/ci/dora-release.mts`, verify archive bytes and executable version, and install atomically.
- [x] Remove release/Dockerfile version literals. Check manual release-image inputs against the resolved version.
- [x] Require native authentication, registration, session/CLI compatibility and shutdown qualification against the downloaded artifact before publishing either architecture's images.

**Validation**

- [x] Reproduce minor/patch and reconnect mismatch failures before the handshake change; all 20 protocol tests pass afterward.
- [x] Test upgraded resolution, mixed versions, unpinned versions/architectures, unavailable archives, corrupt archives and wrong executable versions; existing output survives failure.
- [x] Download and qualify the exact current artifact on x86 and ARM separately.
- [x] Run focused locked checks, complete local integration CI, hosted checks and the ARM64 worker; final revision receipts are recorded in [PR #313](https://github.com/Cutuy/doramcap/pull/313).
- [x] Review rule coverage with Scanner and record remaining qualification.

The initial implementation (`eca7941f`) passed complete local and hosted x86/ARM
CI, ARM64 integration job 656 and native artifact qualification job 658. The same
downloader selected official Dora 1.1.0 on each architecture; both native fixtures
passed authentication, session/CLI compatibility, registration and shutdown.
The PR records subsequent checks after rebasing onto the merged backend changes.
Tag-triggered image publication is verified by workflow inspection and remains
unexecuted locally. Exact version matching establishes reported protocol-version
agreement; schema fingerprints and broader GPU/IPC qualification are outside scope.

Packaging supports published `major.minor.patch` releases on Linux amd64 and arm64.
Downloads require the committed SHA-256 for that version and architecture and use
a 120-second network deadline. Unsupported or unavailable selections fail explicitly. Archive
staging is removed after success or failure; existing output changes only after
checksum and executable-version checks pass.

Upgrade the shared requirements when needed, update Cargo.lock through Cargo and
add the new release's reviewed archive checksums to `ARCHIVE_SHA256`.
No runtime environment setting or new third-party dependency is introduced. Release
qualification uses the existing `DORAMCAP_TEST_DORA` test input. Publication itself
is outside local validation; the release workflow runs the same native fixture
before signing in and pushing images.
