![Deployment architecture](103126-arm64-deployment.svg)

# ARM64 gallery deployment

Deploy the SPA and Activity Lab from remote main, based on
`e056fc114302a9c2f77711166bad0aabd183b67c`. Build on the existing ARM worker;
ship prebuilt artifacts to the small EC2 host over SSH. Connection details and
credentials stay outside Git. No GitHub deploy key or registry is required.

## M1 — Prepare release

**Work items**

- [x] Acquire a worktree lease and base it on fetched remote main.
- [x] Build release manager and Activity Lab with locked Rust dependencies.
- [ ] Build gallery and Activity Lab frontends with lockfile installs.
- [ ] Package the ARM64 app image, Caddy image, runtime tree and checksums.

**Validation**

- [ ] Record source revision, build results and image architectures/digests.
- [ ] Verify checksum manifest and runtime files before transfer.

Evidence: worker has Docker 29.9.2, Node 22.23.4, npm 10.9.9 and Cargo 1.99.1.
Its serial default queue is paused; inspect before scheduling a single build.
Only 6.5 GiB worker disk space was available at initial inspection.
Job 548 built both release binaries in 1m39s and passed 36 gallery library tests.
The SPA generator requires Git metadata. Job 549 then exposed a catalog bug:
the new `console-native` workspace member was discovered but `console/` documents
were excluded. Include that directory in the existing document filter and record
the resulting release commit. Existing worker caches were preserved.

## M2 — Provision and deploy

**Work items**

- [x] Install Docker/Compose and create a dedicated gallery service account.
- [x] Configure a small swap file, private persistent results and release paths.
- [ ] Load prebuilt images and configure manager/Caddy systemd services.
- [ ] Bind gallery access to loopback and retain SSH-only external access.

**Validation**

- [ ] Verify services, image architecture, permissions and loopback listeners.
- [ ] Verify deployed source revision and SPA/hosting API responses.
- [ ] Record memory/disk use and service restart behavior.

Evidence: EC2 has Docker 29.1.3, Compose 2.40.3, a dedicated `gallery` account,
mode-0700 results storage and a mode-0600 1 GiB swap file. No application
environment configuration inputs were added.

## M3 — Exercise hosted app

**Work items**

- [ ] Launch Activity Lab through the gallery and exercise its routed API.
- [ ] Verify shared viewers, heartbeat, expiry and recording publication.
- [ ] Verify a second generation and restart recovery.
- [ ] Document operator access, update and rollback commands.

**Validation**

- [ ] Record HTTP, stream and MCAP results plus browser verification.
- [ ] Check public changes for connection details and credential material.
- [ ] Record focused checks and deferred broader/other-app qualification.

Full workspace CI, other demos, GPU execution and Kubernetes are outside this
initial deployment. ARM release and runtime results will be reported separately.
