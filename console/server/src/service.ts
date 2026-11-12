import { runPlan } from './run-plan.js';
import { ensureImages } from './images.js';
import { PREPARING_RUN, FILE_CHANGES_ARTIFACT } from './generated/transport.js';
import Long from 'long';
import { mkdir, rm } from 'node:fs/promises';
import { closeSync, fstatSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { randomBytes, randomUUID } from 'node:crypto';
import {
  capabilities,
  wire,
  execution,
  browserError,
  composePlan,
  launch,
  objectOptions,
  type Config,
  type NativeExecution,
  type ConsoleBackend,
  type Project,
  type App,
  type Run,
  type Snapshot,
} from './native.js';
import type { doramcap } from './generated/console.js';
import { loadConfig } from './config.js';
import { discover, selectApps, appOptions, appProfiles, outputs, bindOutputs, type SelectedApp } from './system-apps.js';
import { Graphs } from './graphs.js';
import { systemdPlan, systemdProject } from './systemd.js';
import { writeOnce } from './files.js';
import { Store } from './store.js';
import { History } from './history.js';
import { compareReview, compareRuns, machineChanges, readManifest, type Side } from './compare.js';
import { sourceState } from './source.js';
import { fileChangePreview, fileChangeAvailability, exportFileChanges, filesystem } from './file-changes.js';
import { Progress, MAX_PROGRESS_ROWS } from './progress.js';
import { LiveRelay, bindRelay, liveStreams } from './live.js';
import { availablePort } from './launch.js';

type Inspection = doramcap.console.execution.v1.Inspection;
type ExecutionResult = doramcap.console.execution.v1.IExecutionResult;
type Prepared = {
  review: typeof wire.Review.prototype;
  project: Project;
  apps: SelectedApp[];
  workspace: string;
  admission: doramcap.console.execution.v1.StartRequest;
};
const { Phase } = wire;
const active = (phase: number) => [Phase.STARTING, Phase.RUNNING, Phase.STOPPING].includes(phase);
const MAX_METRICS = 1200;
/** Compose system app that keeps the run IPC namespace available until shutdown. */
const RUN_SHM_APP = 'doramcap-shm';
const integer = (value: unknown) => BigInt(String(value ?? 0));
const relayApp = (apps: App[]) => apps.find((app) => app.liveRelay);
const errorCode = (error: unknown) => (error as Error)?.message ?? 'Internal';
/** A browser error code that keeps its cause, when the cause says more than the code. */
const failure = (code: string, cause: unknown) =>
  Object.assign(new Error(code), { detail: errorCode(cause) === code ? '' : browserError(errorCode(cause)) });

/** The TypeScript application owns request coordination, browser state and persistence. */
export class ConsoleService implements ConsoleBackend {
  private snapshot: Snapshot = wire.Status.create({ phase: Phase.IDLE });
  private marker: typeof wire.Marker.prototype | null = null;
  private prepared?: Prepared;
  private progress = new Progress();
  private graphs: Graphs;
  private relay?: LiveRelay;
  private timer: ReturnType<typeof setInterval>;
  private ticking?: Promise<void>;
  private preparing?: Promise<Buffer>;
  private preparation = new AbortController();
  private startKey = '';
  private startId = '';
  private startPromise?: Promise<Buffer>;
  private admissionPending = false;
  private accepted = false;
  private stopped = false;
  private completed = true;
  private pendingResultAt?: number;
  private metadataFailed = false;
  private executionStarted = false;
  private begun = 0n;
  private lastSample = 0;
  private previous?: { bytes: bigint; time: number };
  private closing?: Promise<void>;
  private closed = false;
  private startedAt = 0;
  private machineBefore: typeof execution.RunCapture.prototype | null = null;
  private machineAfter = false;
  private recorded = Phase.PHASE_UNSPECIFIED;
  private runIndex: History;
  constructor(
    readonly config: Config,
    private engine: NativeExecution,
    private store: Store,
  ) {
    this.runIndex = new History(store);
    this.graphs = new Graphs(capabilities, config.apps, config.dataflowFile ?? '');
    const relay = relayApp(config.apps);
    if (relay) this.relay = new LiveRelay(`ws://127.0.0.1:${relay.liveRelay!.port}/stream`);
    this.timer = setInterval(
      () =>
        void this.tick().catch(() => {
          this.snapshot.metricsIncomplete = true;
        }),
      100,
    );
    this.timer.unref();
  }
  static async open(path: string) {
    const { config, secret } = await loadConfig(path);
    const store = await Store.open(config.dataRoot);
    config.dataRoot = store.root;
    try {
      // One port for this service's life keeps Review and Start on the same bridge inputs.
      const relay = relayApp(config.apps);
      if (relay && !relay.liveRelay!.port) relay.liveRelay!.port = await availablePort();
      // A systemd Console has no Docker socket, so the addon never connects to Docker.
      const engine = await capabilities.open(config.dockerSocket, 1);
      await new History(store).closeInterrupted();
      const backend = new ConsoleService(config, engine, store);
      await backend.restore();
      return { backend, config, secret };
    } catch (error) {
      store.close();
      throw error;
    }
  }
  private async restore() {
    const marker = this.store.marker();
    if (
      !marker ||
      marker.version !== 1 ||
      !/^[a-f0-9]{32}$/.test(marker.runId) ||
      marker.artifactReference !== marker.runId
    )
      return;
    this.marker = marker;
    if (marker.phase !== Phase.FINISHED || marker.outcome?.boundaryClean !== true) return;
    this.snapshot = wire.Status.create({
      phase: marker.phase,
      runId: marker.runId,
      outcome: marker.outcome,
      profiles: marker.profiles,
      systemApps: marker.systemApps,
      recordings: marker.recordings,
    });
    const saved = this.store.activity(marker.runId);
    if (
      saved &&
      saved.runId === marker.runId &&
      saved.phase === marker.phase &&
      saved.steps.length <= MAX_PROGRESS_ROWS &&
      saved.metrics.length <= MAX_METRICS
    ) {
      for (const row of saved.steps) this.progress.apply(row);
      saved.steps = this.progress.rows;
      this.snapshot = saved;
    }
    if (marker.startAccepted) {
      this.startKey = marker.requestKey;
      this.startId = marker.runId;
      this.startPromise = Promise.resolve(
        Buffer.from(wire.StartResponse.encode({ runId: marker.runId }).finish()),
      );
    }
  }
  prepare(input: Buffer): Promise<Buffer> {
    if (this.closing || this.preparing || active(this.snapshot.phase) || this.admissionPending)
      return Promise.reject(new Error('Conflict'));
    const request = wire.PrepareRequest.decode(input);
    this.preparing = this.prepareRequest(request).finally(() => {
      this.preparing = undefined;
    });
    return this.preparing;
  }
  private async prepareRequest(request: typeof wire.PrepareRequest.prototype) {
    try {
      await this.ticking;
      this.reconcileResult();
    } catch (error) {
      throw failure('Internal', error);
    }
    if (this.closing) throw new Error('Closing');
    this.preparation = new AbortController();
    this.snapshot = wire.Status.create({ phase: Phase.PREPARING });
    this.startPromise = undefined;
    this.startKey = '';
    if (this.prepared) {
      await rm(this.prepared.workspace, { recursive: true, force: true });
      this.prepared = undefined;
    }
    const id = randomBytes(16).toString('hex');
    let workspace = '';
    try {
      const ids = request.systemApps?.ids ?? undefined;
      const project = await this.project(request.profiles),
        chosen = selectApps(this.config.apps.filter((app) => app.id !== RUN_SHM_APP && !this.config.doraApps.includes(app.id!)), ids, request.systemApps?.profiles ?? {}),
        // The relay runs only when a selected app declares a live output.
        apps = chosen.filter((app) => !app.liveRelay || chosen.some((other) => other.liveOutputs?.length));
      // Compose profiles select services; systemd projects have none.
      const profiles = this.systemd() ? [] : (
        await this.engine.composeProfiles(
          execution.ComposeProfilesRequest.create({
            runId: id,
            repositoryRoot: project.directory,
            project,
          }),
        )
      ).names;
      if (
        new Set(request.profiles).size !== request.profiles.length ||
        request.profiles.some((profile) => !profiles.includes(profile))
      )
        throw new Error('unknown or duplicate profile');
      workspace = join(this.config.dataRoot, 'reviews', id);
      await mkdir(dirname(workspace), { recursive: true, mode: 0o700 });
      await mkdir(workspace, { mode: 0o700 });
      const runRequest = await this.runRequest(id, project, apps, workspace, join(workspace, 'resolved'), 'review');
      if (this.config.downloadMissingImages && runRequest.compose)
        await ensureImages(this.config, (runRequest.compose.projects ?? []) as Project[], this.preparation.signal);
      let inspected: Inspection;
      try {
        inspected = await this.engine.inspect(runRequest);
      } catch (error) {
        if (errorCode(error) === 'EmptySelection' && profiles.length && !request.profiles.length) {
          await rm(workspace, { recursive: true, force: true });
          this.snapshot = wire.Status.create({ phase: Phase.IDLE });
          return Buffer.from(
            wire.Review.encode({
              availableProfiles: profiles,
              systemApps: appOptions(this.config.apps.filter((app) => app.id !== RUN_SHM_APP), chosen, this.config.doraApps),
              warnings: ['Select a profile to enable at least one service.'],
            }).finish(),
          );
        }
        throw error;
      }
      if (this.preparation.signal.aborted) throw new Error('Stopped');
      const review = wire.Review.create({
        preparedId: id,
        runId: id,
        availableProfiles: profiles,
        liveStreams: liveStreams(apps),
        selectedProfiles: request.profiles,
        services: Object.entries(inspected.plan?.services ?? {}).map(([name, service]) => ({
          name,
          imageId: service.imageId,
        })),
        systemApps: appOptions(this.config.apps.filter((app) => app.id !== RUN_SHM_APP), chosen, this.config.doraApps),
        recordings: outputs(apps),
        warnings: [...(inspected.plan?.warnings ?? []), ...inspected.filesystemNotes],
        comparison: await this.reviewComparison(request.baselineRunId, id, project, apps, inspected),
      });
      this.prepared = {
        review,
        project,
        apps,
        workspace,
        admission: execution.StartRequest.create({
          composeExpected: inspected.composeSnapshot,
          systemdExpected: inspected.systemdSnapshot,
        }),
      };
      this.snapshot = wire.Status.create({
        phase: Phase.REVIEWED,
        runId: id,
        profiles: request.profiles,
        systemApps: { ids: apps.map((app) => app.id!), profiles: appProfiles(apps) },
        recordings: outputs(apps),
        liveStreams: review.liveStreams,
      });
      return Buffer.from(wire.Review.encode(review).finish());
    } catch (error) {
      this.snapshot = wire.Status.create({ phase: Phase.IDLE });
      if (workspace) await rm(workspace, { recursive: true, force: true });
      await this.diagnostic(error);
      const code = errorCode(error);
      throw failure(['Busy', 'Closing', 'ImageDownload', 'Stopped'].includes(code) ? code : 'InvalidInput', error);
    }
  }
  private async runRequest(
    id: string,
    project: Project,
    apps: SelectedApp[],
    workspace: string,
    artifacts: string,
    stage: string,
  ): Promise<Run> {
    const relay = relayApp(apps);
    const bound = relay && (await bindRelay(relay, apps, workspace, stage));
    const graph = await this.graphs.build(apps.map((app) => app.id!), bound ? [bound.graph] : [], appProfiles(apps));
    let dataflowFile = '';
    if (graph.length) {
      const path = join(workspace, `${stage}.graph.json`);
      await writeOnce(path, graph);
      dataflowFile = path;
    }
    if (this.systemd())
      return systemdPlan(this.config, {
        id, project: project.directory!, artifacts, workspace, stage, apps,
        graph: dataflowFile, relaySettings: bound?.settings,
      });
    // The configured IPC placeholder belongs to the run, independent of optional app selection.
    const composeApps = [...apps, ...this.config.apps.filter((app) => app.id === RUN_SHM_APP)];
    const request = execution.RunRequest.create({
      run: runPlan(this.config, { id, project: project.directory!, artifacts, apps: composeApps }, dataflowFile),
      compose: composePlan.ComposePlan.create({ projects: [project, ...this.config.apps.filter((app) => this.config.doraApps.includes(app.id!)).map((app) => ({ ...app.input!, key: app.id! }))] }),
    });
    await bindOutputs(composeApps, request, workspace, stage);
    if (bound) request.compose!.projects!.find((input) => input.key === relay!.id)!.files!.push(bound.overlay);
    return request;
  }
  /** Whether this Console launches runs as systemd units. */
  private systemd() {
    return this.config.form === launch.Form.SYSTEMD;
  }
  /** The project in the configured project root: Compose files, or a directory of units. */
  private project(profiles: string[]) {
    return this.systemd() ? systemdProject(this.config.projectRoot) : discover(this.config.projectRoot, profiles);
  }
  start(input: Buffer): Promise<Buffer> {
    const request = wire.StartRequest.decode(input);
    if (!request.requestKey || request.requestKey.length > 128)
      return Promise.reject(new Error('InvalidInput'));
    if (this.startKey === request.requestKey) {
      if (this.startId !== request.preparedId) return Promise.reject(new Error('Conflict'));
      return this.startPromise!;
    }
    if (
      this.closing ||
      this.preparing ||
      this.snapshot.phase !== Phase.REVIEWED ||
      !this.prepared ||
      request.preparedId !== this.prepared.review.preparedId
    )
      return Promise.reject(new Error('Conflict'));
    const prepared = this.prepared;
    this.prepared = undefined;
    this.startKey = request.requestKey;
    this.startId = request.preparedId;
    this.admissionPending = true;
    this.accepted = false;
    this.stopped = false;
    this.completed = false;
    this.executionStarted = false;
    this.metadataFailed = false;
    this.progress = new Progress();
    this.previous = undefined;
    this.begun = process.hrtime.bigint();
    this.lastSample = 0;
    this.marker = wire.Marker.create({
      version: 1,
      runId: request.preparedId,
      requestKey: request.requestKey,
      phase: Phase.STARTING,
      artifactReference: request.preparedId,
      profiles: prepared.project.profiles,
      systemApps: { ids: prepared.apps.map((app) => app.id!), profiles: appProfiles(prepared.apps) },
      recordings: outputs(prepared.apps),
      startAccepted: false,
    });
    this.snapshot = wire.Status.create({
      ...this.marker,
      phase: Phase.STARTING,
      metrics: [],
      steps: [],
      liveStreams: prepared.review.liveStreams,
    });
    this.startedAt = Date.now();
    this.machineBefore = null;
    this.machineAfter = false;
    this.recordTransition();
    this.startPromise = this.startRequest(prepared);
    return this.startPromise;
  }
  private async startRequest(prepared: Prepared): Promise<Buffer> {
    try {
      // Persist intent before execution; interrupted requests are never replayed on restart.
      this.store.write(this.marker!);
      this.recorded = this.snapshot.phase;
      this.runIndex.record(this.enriched(), this.startedAt);
      this.saveCapture({ setup: await this.setup(prepared) });
      const fresh = await this.project(prepared.project.profiles ?? []).catch(() => {
        throw new Error('StaleReview');
      });
      if (JSON.stringify(fresh) !== JSON.stringify(prepared.project)) throw new Error('StaleReview');
      const runs = join(this.config.dataRoot, 'runs');
      await mkdir(runs, { recursive: true, mode: 0o700 });
      let runRequest: Run;
      try {
        runRequest = await this.runRequest(
          this.startId,
          fresh,
          prepared.apps,
          prepared.workspace,
          join(runs, this.startId),
          'execution',
        );
      } catch {
        throw new Error('StaleReview');
      }
      if (this.stopped) throw new Error('Stopped');
      this.executionStarted = true;
      await this.engine.start(execution.StartRequest.create({ ...prepared.admission, run: runRequest }));
      this.accepted = true;
      this.marker!.startAccepted = true;
      try {
        this.store.write(this.marker!);
      } catch (error) {
        this.metadataFailed = true;
        await this.engine.stop(this.startId);
        throw error;
      }
      return Buffer.from(wire.StartResponse.encode({ runId: this.startId }).finish());
    } catch (error) {
      await this.diagnostic(error);
      let preflightFailed = false, preflightError = '';
      if (this.executionStarted && !this.accepted) {
        const update = this.engine.poll();
        preflightFailed = update.runId === this.startId && update.result?.error != null;
        if (preflightFailed) preflightError = update.result!.error!;
        await this.consume(update);
        if (update.runId !== this.startId) this.executionStarted = false;
      }
      if (!this.executionStarted) await this.finish({ blocked: false, error: errorCode(error) });
      const code = errorCode(error) === 'ConfigurationChanged' ? 'StaleReview' : errorCode(error);
      throw failure(
        ['StaleReview', 'Stopped', 'Conflict', 'Busy', 'Closing'].includes(code)
          ? code
          : preflightFailed
            ? 'PreflightRejected'
            : 'Internal',
        preflightError ? new Error(preflightError) : error,
      );
    } finally {
      this.admissionPending = false;
      await this.ticking;
      await this.tick();
    }
  }
  async stop(input: Buffer) {
    const request = wire.StopRequest.decode(input);
    if (request.runId === PREPARING_RUN && this.preparing) {
      this.preparation.abort();
      return Buffer.from(wire.StopResponse.encode({ accepted: true }).finish());
    }
    if (!this.marker || this.marker.runId !== request.runId || this.preparing) throw new Error('Conflict');
    if (active(this.snapshot.phase)) {
      this.stopped = true;
      this.snapshot.phase = Phase.STOPPING;
      if (this.executionStarted) await this.engine.stop(request.runId);
    }
    return Buffer.from(wire.StopResponse.encode({ accepted: true }).finish());
  }
  private tick(): Promise<void> {
    if (this.closed) return Promise.resolve();
    if (!this.ticking)
      this.ticking = this.tickInner().finally(() => {
        this.ticking = undefined;
      });
    return this.ticking;
  }
  private async tickInner() {
    // A finished run has nothing left to read, and the addon would re-encode its whole result.
    if (!this.completed) await this.consume(this.engine.poll());
  }
  private async setup(prepared: Prepared) {
    const images = prepared.review.services.map(({ name, imageId }) => ({ service: name, imageId }));
    return execution.ExperimentSetup.create({ ...(await sourceState(prepared.project.directory!)), images });
  }
  /** Save capture parts; observation and storage failures remain visible while the run continues. */
  private saveCapture(parts: Partial<typeof execution.RunCapture.prototype>) {
    try {
      return this.runIndex.saveCapture(this.snapshot.runId, parts);
    } catch (error) {
      console.error('Could not save run capture', error);
      return execution.RunCapture.create({ ...parts, before: null,
        beforeError: `Machine capture could not be saved: ${browserError(errorCode(error))}`,
      });
    }
  }
  private saveMachineStates(update: typeof execution.ExecutionUpdate.prototype) {
    if ((update.machineBefore || update.machineBeforeError) && !this.machineBefore) {
      this.machineBefore = this.saveCapture({ before: update.machineBefore, beforeError: browserError(update.machineBeforeError) });
      this.snapshot.machineChanges = machineChanges(this.machineBefore.before, null, this.machineBefore.beforeError);
    }
    if ((update.machineAfter || update.machineAfterError) && !this.machineAfter) {
      this.machineAfter = true;
      this.snapshot.machineChanges = machineChanges(this.machineBefore?.before, update.machineAfter, this.machineBefore?.beforeError, browserError(update.machineAfterError));
    }
  }
  private async consume(update: typeof execution.ExecutionUpdate.prototype) {
    if (!this.completed && update.runId === this.snapshot.runId) {
      this.saveMachineStates(update);
      for (const event of update.events) this.progress.apply(event);
      this.snapshot.steps = this.progress.rows;
      this.snapshot.sequence = Long.fromString(this.progress.sequence.toString(), true);
      this.snapshot.lostEvents = update.droppedEvents;
      this.snapshot.truncatedSteps = this.progress.truncated;
      this.snapshot.overflowSteps = this.progress.overflow;
      this.snapshot.progressIncomplete = this.progress.incomplete || integer(update.droppedEvents) > 0n;
      this.snapshot.activityAvailable = this.progress.rows.length > 0;
      if (update.teardownStarted) this.snapshot.phase = Phase.STOPPING;
      else if (update.ready && this.snapshot.phase === Phase.STARTING) this.snapshot.phase = Phase.RUNNING;
      const now = Date.now();
      this.snapshot.stoppingServices = update.stoppingServices.map(({ service, forceStopUnixMs }) =>
        wire.StoppingService.create({
          service,
          forceStopInSeconds: Math.max(0, Math.ceil((Number(forceStopUnixMs) - now) / 1000)),
        }),
      );
      await this.sample();
      if (this.snapshot.phase === Phase.RUNNING && this.snapshot.liveStreams.length) this.relay?.open(this.snapshot.runId);
      else this.relay?.close();
      if (update.result && !this.admissionPending) await this.finish(update.result);
      else this.recordTransition();
    }
  }
  /** Save the run on each phase change. A failed save is retried at the next change and at the end. */
  private recordTransition() {
    if (this.snapshot.phase === this.recorded) return;
    try {
      this.runIndex.record(this.enriched(), this.startedAt);
      this.recorded = this.snapshot.phase;
    } catch (error) {
      console.error('Could not save run progress', error);
    }
  }
  private async sample() {
    const now = performance.now();
    if (now - this.lastSample < this.config.runtime!.sampleIntervalMs!) return;
    this.lastSample = now;
    const recordings = this.marker?.recordings ?? [];
    if (!recordings.length) return;
    let bytes: bigint | undefined = 0n;
    for (const output of recordings) {
      try {
        const fd = capabilities.openFile(
          join(this.config.dataRoot, 'runs'),
          `${this.snapshot.runId}/${output.path}`,
        );
        try {
          bytes += BigInt(fstatSync(fd).size);
        } finally {
          closeSync(fd);
        }
      } catch {
        bytes = undefined;
        break;
      }
    }
    const rate =
      bytes != null && this.previous && bytes >= this.previous.bytes
        ? Number(bytes - this.previous.bytes) / Math.max((now - this.previous.time) / 1000, 0.001)
        : null;
    if (bytes != null) this.previous = { bytes, time: now };
    this.snapshot.metrics.push(
      wire.RunMetric.fromObject({
        elapsedNs: String(process.hrtime.bigint() - this.begun),
        recordingBytes: bytes?.toString() ?? null,
        writeBytesPerSecond: rate,
      }),
    );
    if (this.snapshot.metrics.length > MAX_METRICS) this.snapshot.metrics.shift();
  }
  private async finish(result: ExecutionResult) {
    this.snapshot.stoppingServices = [];
    const clean = result.boundaryClean ?? !result.blocked;
    this.snapshot.phase =
      this.metadataFailed || result.blocked || !clean
        ? Phase.BLOCKED
        : !this.accepted && !this.stopped
          ? Phase.IDLE
          : Phase.FINISHED;
    this.snapshot.outcome = wire.Outcome.create({
      execution: !this.accepted && this.stopped ? 'Cancelled' : result.error ? 'Failed' : 'Completed',
      error: browserError(result.error ?? ''),
      boundaryClean: !!clean,
      writersStopped: this.accepted ? result.writersStopped : null,
      recordingFinalized: this.marker!.recordings.length ? result.recordingFinalized : null,
      serviceExits: result.serviceExits ?? [],
      eventStreamHealthy: this.accepted ? result.eventStreamHealthy : null,
    });
    const categories = result.fileChanges?.categories ?? [];
    this.snapshot.fileChanges = result.fileChanges ? wire.FileChanges.create({
      report: result.fileChanges, state: filesystem.ReportState.REPORT_COMPLETE, downloadable: true,
      previewIncomplete: categories.some(c => integer(c.counts?.added) + integer(c.counts?.modified) + integer(c.counts?.deleted) > 0n),
      notesIncomplete: categories.some(c => integer(c.coverageNoteCount) > 0n),
    }) : null;
    this.marker!.fileChanges = result.fileChanges ?? null;
    this.marker!.phase = this.snapshot.phase;
    this.marker!.outcome = this.snapshot.outcome;
    this.pendingResultAt = Date.now();
    this.completed = true;
    this.snapshot.fileChanges = await fileChangePreview(
      join(this.config.dataRoot, 'runs', this.snapshot.runId), result.fileChanges, result.fileChangesState ?? filesystem.ReportState.REPORT_UNAVAILABLE,
    );
    this.marker!.fileChanges = this.snapshot.fileChanges?.report ?? null;
    this.marker!.phase = this.snapshot.phase;
    this.marker!.outcome = this.snapshot.outcome;
    if (result.error) await this.diagnostic(new Error(result.error));
    {
      const report = join(this.config.dataRoot, 'runs', this.snapshot.runId, 'console-report.json');
      if (this.executionStarted) await mkdir(dirname(report), { recursive: true, mode: 0o700 });
      if (this.executionStarted)
        await writeOnce(
          report,
          Buffer.from(
            JSON.stringify({
              execution: this.snapshot.outcome.execution,
              error: result.error ?? null,
              writers_stopped: this.snapshot.outcome.writersStopped,
              recording_finalized: this.snapshot.outcome.recordingFinalized,
              service_exits: (this.snapshot.outcome.serviceExits ?? []).map(exit => ({
                service: exit.service,
                exit_code: exit.exitCode,
                oom_killed: exit.oomKilled,
                killed: exit.killed,
              })),
              boundary_clean: this.snapshot.outcome.boundaryClean,
              event_stream_healthy: this.snapshot.outcome.eventStreamHealthy,
              system_apps: this.snapshot.systemApps?.ids ?? [],
              file_changes: this.snapshot.fileChanges ? wire.FileChanges.toObject(wire.FileChanges.create(this.snapshot.fileChanges), { ...objectOptions, bytes: String }) : null,
              file_changes_artifact: this.snapshot.fileChanges?.downloadable ? FILE_CHANGES_ARTIFACT : null,
            }),
          ),
        );
      this.reconcileResult();
    }
    this.recorded = this.snapshot.phase;
  }
  /** Save the terminal marker and History together; failure throws and remains pending for retry. */
  private reconcileResult() {
    const finishedAt = this.pendingResultAt;
    if (finishedAt === undefined) return;
    this.store.transaction(() => {
      this.store.write(this.marker!);
      this.runIndex.record(this.enriched(), this.startedAt, finishedAt, this.snapshot.outcome?.error ?? '');
    });
    this.pendingResultAt = undefined;
  }
  private enriched(): Snapshot {
    const status = wire.Status.fromObject(wire.Status.toObject(this.snapshot, objectOptions));
    const terminal = [Phase.FINISHED, Phase.BLOCKED].includes(status.phase);
    status.projectDirectory = this.config.projectRoot;
    status.cpuAvailable =
      terminal &&
      status.outcome?.recordingFinalized === true &&
      status.recordings.find((output) => output.cpu)?.path != null &&
      this.store.available(status.runId, status.recordings.find((output) => output.cpu)!.path!);
    status.reportAvailable = terminal && this.store.available(status.runId, 'console-report.json');
    if (terminal) fileChangeAvailability(join(this.config.dataRoot, 'runs', status.runId), status.fileChanges);
    return status;
  }
  async status() {
    await this.tick();
    return Buffer.from(wire.Status.encode(this.enriched()).finish());
  }
  async update(cursor: Buffer) {
    const status = wire.Status.decode(await this.status()),
      position = wire.StreamCursor.decode(cursor);
    const last = status.metrics.at(-1)?.elapsedNs ?? 0;
    const next = wire.StreamCursor.create({
      runId: status.runId,
      sequence: status.sequence,
      metricElapsedNs: last,
    });
    if (
      position.runId === status.runId &&
      integer(position.sequence) <= integer(status.sequence) &&
      integer(position.metricElapsedNs) <= integer(last)
    ) {
      status.steps = status.steps.filter((event) => integer(event.sequence) > integer(position.sequence));
      status.metrics = status.metrics.filter(
        (metric) => integer(metric.elapsedNs) > integer(position.metricElapsedNs),
      );
    }
    return {
      data: Buffer.from(wire.Status.encode(status).finish()),
      cursor: Buffer.from(wire.StreamCursor.encode(next).finish()),
    };
  }
  /** Replay the latest frame per stream, then forward bridge frames until unsubscribed. */
  frames(consume: (frame: Buffer) => void): () => void {
    return this.relay?.subscribe(consume) ?? (() => {});
  }
  async *fileChanges(id: string, signal: AbortSignal) {
    const run = this.artifactRun(id);
    const reference = run.fileChanges?.report;
    if (!reference || !run.fileChanges?.downloadable) throw new Error('Missing');
    yield* exportFileChanges(join(this.config.dataRoot, 'runs', id), reference, signal);
  }
  private artifactRun(id: string) {
    if (!/^[a-f0-9]{32}$/.test(id) || this.marker?.runId === id && active(this.snapshot.phase)) throw new Error('Missing');
    const run = this.marker?.runId === id && [Phase.FINISHED, Phase.BLOCKED].includes(this.snapshot.phase)
      ? this.snapshot : this.store.activity(id);
    if (!run || run.runId !== id) throw new Error('Missing');
    return run;
  }
  async openArtifact(id: string, kind: string) {
    if (!/^[a-f0-9]{32}$/.test(id)) throw new Error('Missing');
    const current = this.marker?.runId === id;
    if (current && active(this.snapshot.phase)) throw new Error('Missing');
    // Past runs resolve through their saved status.
    const run = current && [Phase.FINISHED, Phase.BLOCKED].includes(this.snapshot.phase) ? this.marker! : this.store.activity(id);
    if (!run || run.runId !== id) throw new Error('Missing');
    const outputs = run.recordings;
    const name =
      kind === 'report'
        ? 'console-report.json'
        : run.outcome?.recordingFinalized
          ? outputs.find((output) => output.id === kind || (kind === 'cpu' && output.cpu))?.path
          : undefined;
    if (!name) throw new Error('Missing');
    try {
      return capabilities.openFile(join(this.config.dataRoot, 'runs'), `${id}/${name}`);
    } catch {
      throw new Error('Missing');
    }
  }
  async history(input: Buffer) {
    return Buffer.from(wire.HistoryPage.encode(this.runIndex.list(wire.HistoryQuery.decode(input))).finish());
  }
  async historyRun(id: string) {
    const detail = this.runIndex.detail(id);
    if (!detail) throw new Error('Missing');
    return Buffer.from(wire.HistoryDetail.encode(detail).finish());
  }
  /** One stored run's comparison inputs, or null when it is unknown. */
  private side(id: string): Side | null {
    const detail = this.runIndex.detail(id);
    return detail && { id, detail, capture: this.runIndex.capture(id), manifest: readManifest(join(this.config.dataRoot, 'runs'), id) };
  }
  async historyCompare(input: Buffer) {
    const { sides } = wire.HistoryComparisonRequest.decode(input);
    const [left, right] = sides.map(({ runIds }) => {
      const side = this.side(runIds![0]);
      if (!side) throw new Error('Missing');
      return side;
    });
    return Buffer.from(wire.HistoryComparison.encode(compareRuns(left, right, sides.map(side => side.label!))).finish());
  }
  /** Compare this review with its baseline. A failure leaves the preview empty, and the review continues. */
  private async reviewComparison(baselineId: string, id: string, project: Project, apps: SelectedApp[], inspected: Inspection) {
    try {
      const chosen = baselineId || this.runIndex.latestCaptured();
      const setup = execution.ExperimentSetup.create({
        ...(await sourceState(project.directory!)),
        images: Object.entries(inspected.plan?.services ?? {}).map(([name, service]) => ({ service: name, imageId: service.imageId })),
      });
      return compareReview(chosen ? this.side(chosen) : null, {
        capture: execution.RunCapture.create({ setup, before: inspected.machine, beforeError: browserError(inspected.machineError) }),
        manifest: readManifest(join(this.config.dataRoot, 'reviews'), id, `${id}/resolved/experiment.input.json`),
        profiles: project.profiles ?? [],
        apps: wire.SystemAppSelection.create({ ids: apps.map((app) => app.id!), profiles: appProfiles(apps) }),
      });
    } catch (error) {
      console.error('Could not compare this review', error);
      return null;
    }
  }
  close() {
    if (!this.closing) this.closing = this.shutdown();
    return this.closing;
  }
  private async shutdown() {
    try {
      this.preparation.abort();
      await this.preparing?.catch(() => {});
      this.stopped = true;
      await this.engine.close();
      await this.startPromise?.catch(() => {});
      await this.tick();
    } finally {
      clearInterval(this.timer);
      this.relay?.close();
      this.closed = true;
      this.store.close();
    }
  }
  private async diagnostic(error: unknown) {
    try {
      const directory = join(this.config.dataRoot, 'diagnostics');
      await mkdir(directory, { recursive: true, mode: 0o700 });
      await writeOnce(
        join(directory, randomUUID() + '.json'),
        Buffer.from(JSON.stringify({ error: String(error) })),
      );
    } catch {}
  }
}
