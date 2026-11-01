import Long from 'long';
import { mkdir, rm } from 'node:fs/promises';
import { closeSync, fstatSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { randomBytes, randomUUID } from 'node:crypto';
import {
  capabilities,
  wire,
  execution,
  compose,
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
import { discover, selectApps, appOptions, outputs, bindOutputs } from './system-apps.js';
import { Graphs } from './graphs.js';
import { Store, writeOnce } from './store.js';
import { Progress, MAX_PROGRESS_ROWS } from './progress.js';
import { CpuFeeds, MAX_SNAPSHOTS, MAX_GROUP_NAMES } from './cpu-feed.js';

type Inspection = doramcap.console.execution.v1.Inspection;
type ExecutionResult = doramcap.console.execution.v1.IExecutionResult;
type Prepared = {
  review: typeof wire.Review.prototype;
  project: Project;
  apps: App[];
  workspace: string;
  snapshot: doramcap.compose.v1.ConfigurationSnapshot;
};
const { Phase } = wire;
const active = (phase: number) => [Phase.STARTING, Phase.RUNNING, Phase.STOPPING].includes(phase);
const MAX_METRICS = 1200;
const integer = (value: unknown) => BigInt(String(value ?? 0));
const errorCode = (error: unknown) => (error as Error)?.message ?? 'Internal';

/** The TypeScript application owns request coordination, browser state and persistence. */
export class ConsoleService implements ConsoleBackend {
  private snapshot: Snapshot = wire.Status.create({ phase: Phase.IDLE });
  private marker: typeof wire.Marker.prototype | null = null;
  private prepared?: Prepared;
  private progress = new Progress();
  private graphs: Graphs;
  private feeds: CpuFeeds;
  private timer: ReturnType<typeof setInterval>;
  private ticking?: Promise<void>;
  private preparing?: Promise<Buffer>;
  private startKey = '';
  private startId = '';
  private startPromise?: Promise<Buffer>;
  private admissionPending = false;
  private accepted = false;
  private stopped = false;
  private completed = true;
  private metadataFailed = false;
  private executionStarted = false;
  private begun = 0n;
  private lastSample = 0;
  private previous?: { bytes: bigint; time: number };
  private closing?: Promise<void>;
  private closed = false;
  constructor(
    readonly config: Config,
    private engine: NativeExecution,
    private store: Store,
  ) {
    this.graphs = new Graphs(capabilities, config.systemApps, config.dora?.graph ?? '');
    this.feeds = new CpuFeeds(
      { status: async () => Buffer.from(wire.Status.encode(this.snapshot).finish()) },
      config.systemApps
        .filter((app) => app.cpuFeedUrl)
        .map((app) => ({ appId: app.id, url: app.cpuFeedUrl })),
    );
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
      const engine = await capabilities.open(config.dockerSocket, 1);
      const backend = new ConsoleService(config, engine, store);
      await backend.restore();
      return { backend, config, secret };
    } catch (error) {
      store.close();
      throw error;
    }
  }
  private async restore() {
    const marker = await this.store.marker();
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
      projectDirectory: marker.projectDirectory,
      profiles: marker.profiles,
      systemApps: marker.systemApps,
      recordings: marker.recordings,
    });
    const saved = await this.store.activity(marker.runId);
    if (
      saved &&
      saved.runId === marker.runId &&
      saved.phase === marker.phase &&
      saved.steps.length <= MAX_PROGRESS_ROWS &&
      saved.metrics.length <= MAX_METRICS &&
      saved.cpuSnapshots.length <= MAX_SNAPSHOTS &&
      saved.cpuGroupNames.length <= MAX_GROUP_NAMES
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
      const project = await discover(this.config.projectRoot, request.directory, request.profiles),
        apps = selectApps(this.config.systemApps, request.systemApps?.ids ?? undefined);
      const profiles = (
        await this.engine.profiles(
          execution.ProfilesRequest.create({
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
      const run = await this.runRequest(id, project, apps, workspace, join(workspace, 'resolved'), 'review');
      let inspected: Inspection;
      try {
        inspected = await this.engine.inspect(run);
      } catch (error) {
        if (errorCode(error) === 'EmptySelection' && profiles.length && !request.profiles.length) {
          await rm(workspace, { recursive: true, force: true });
          this.snapshot = wire.Status.create({ phase: Phase.IDLE });
          return Buffer.from(
            wire.Review.encode({
              availableProfiles: profiles,
              systemApps: appOptions(this.config.systemApps, apps),
              warnings: ['Select a profile to enable at least one service.'],
            }).finish(),
          );
        }
        throw error;
      }
      const review = wire.Review.create({
        preparedId: id,
        runId: id,
        availableProfiles: profiles,
        selectedProfiles: request.profiles,
        services: Object.entries(inspected.plan!.services ?? {}).map(([name, service]) => ({
          name,
          imageId: service.imageId,
        })),
        systemApps: appOptions(this.config.systemApps, apps),
        recordings: outputs(apps),
        warnings: [],
      });
      this.prepared = {
        review,
        project,
        apps,
        workspace,
        snapshot: compose.ConfigurationSnapshot.fromObject(inspected.snapshot!),
      };
      this.snapshot = wire.Status.create({
        phase: Phase.REVIEWED,
        runId: id,
        projectDirectory: project.directory,
        profiles: request.profiles,
        systemApps: { ids: apps.map((app) => app.id!) },
        recordings: outputs(apps),
      });
      return Buffer.from(wire.Review.encode(review).finish());
    } catch (error) {
      this.snapshot = wire.Status.create({ phase: Phase.IDLE });
      if (workspace) await rm(workspace, { recursive: true, force: true });
      await this.diagnostic(error);
      const code = errorCode(error);
      throw new Error(['Busy', 'Closing'].includes(code) ? code : 'InvalidInput');
    }
  }
  private async runRequest(
    id: string,
    project: Project,
    apps: App[],
    workspace: string,
    artifacts: string,
    stage: string,
  ): Promise<Run> {
    const graph = await this.graphs.build(apps.map((app) => app.id!));
    const dora = { ...this.config.dora };
    if (graph.length) {
      const path = join(workspace, `${stage}.graph.json`);
      await writeOnce(path, graph);
      dora.graph = path;
    }
    const run = execution.RunRequest.create({
      id,
      repositoryRoot: project.directory,
      artifacts,
      projects: [project],
      dora,
      recordings: outputs(apps).map((output) => ({
        recording: join(artifacts, output.path!),
        stream: output.stream ? join(artifacts, output.stream) : '',
      })),
      readinessSeconds: this.config.runtime!.readinessSeconds!,
      stopSeconds: this.config.runtime!.stopSeconds!,
    });
    await bindOutputs(apps, run, workspace, stage);
    return run;
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
      projectDirectory: prepared.project.directory,
      profiles: prepared.project.profiles,
      systemApps: { ids: prepared.apps.map((app) => app.id!) },
      recordings: outputs(prepared.apps),
      startAccepted: false,
    });
    this.snapshot = wire.Status.create({ ...this.marker, phase: Phase.STARTING, metrics: [], steps: [] });
    this.startPromise = this.startRequest(prepared);
    return this.startPromise;
  }
  private async startRequest(prepared: Prepared): Promise<Buffer> {
    try {
      // Persist intent before execution; interrupted requests are never replayed on restart.
      await this.store.write(this.marker!);
      const fresh = await discover(
        this.config.projectRoot,
        prepared.project.directory!,
        prepared.project.profiles ?? [],
      ).catch(() => {
        throw new Error('StaleReview');
      });
      if (JSON.stringify(fresh) !== JSON.stringify(prepared.project)) throw new Error('StaleReview');
      const runs = join(this.config.dataRoot, 'runs');
      await mkdir(runs, { recursive: true, mode: 0o700 });
      let run: Run;
      try {
        run = await this.runRequest(
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
      await this.engine.start(run, prepared.snapshot);
      this.accepted = true;
      this.marker!.startAccepted = true;
      try {
        await this.store.write(this.marker!);
      } catch (error) {
        this.metadataFailed = true;
        await this.engine.stop(this.startId);
        throw error;
      }
      return Buffer.from(wire.StartResponse.encode({ runId: this.startId }).finish());
    } catch (error) {
      await this.diagnostic(error);
      let preflightFailed = false;
      if (this.executionStarted && !this.accepted) {
        const update = this.engine.poll();
        preflightFailed = update.runId === this.startId && update.result?.error != null;
        await this.consume(update);
        if (update.runId !== this.startId) this.executionStarted = false;
      }
      if (!this.executionStarted) await this.finish({ blocked: false, error: errorCode(error) });
      const code = errorCode(error) === 'ConfigurationChanged' ? 'StaleReview' : errorCode(error);
      throw new Error(
        ['StaleReview', 'Stopped', 'Conflict', 'Busy', 'Closing'].includes(code)
          ? code
          : preflightFailed
            ? 'PreflightRejected'
            : 'Internal',
      );
    } finally {
      this.admissionPending = false;
      await this.ticking;
      await this.tick();
    }
  }
  async stop(input: Buffer) {
    const request = wire.StopRequest.decode(input);
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
    await this.consume(this.engine.poll());
  }
  private async consume(update: typeof execution.ExecutionUpdate.prototype) {
    if (!this.completed && update.runId === this.snapshot.runId) {
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
      await this.sample();
      if (update.result && !this.admissionPending) await this.finish(update.result);
    }
  }
  private async sample() {
    const now = performance.now();
    if (now - this.lastSample < this.config.runtime!.sampleIntervalMs!) return;
    this.lastSample = now;
    const recordings = this.marker?.recordings ?? [];
    const cpu = this.feeds.sample(this.snapshot.runId);
    if (!recordings.length && !cpu) return;
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
    if (!recordings.length) bytes = undefined;
    const rate =
      bytes != null && this.previous && bytes >= this.previous.bytes
        ? Number(bytes - this.previous.bytes) / Math.max((now - this.previous.time) / 1000, 0.001)
        : null;
    if (bytes != null) this.previous = { bytes, time: now };
    this.snapshot.metrics.push(
      wire.RunMetric.fromObject({
        ...cpu,
        elapsedNs: String(process.hrtime.bigint() - this.begun),
        recordingBytes: bytes?.toString() ?? null,
        writeBytesPerSecond: rate,
      }),
    );
    if (this.snapshot.metrics.length > MAX_METRICS) this.snapshot.metrics.shift();
  }
  private async finish(result: ExecutionResult) {
    this.completed = true;
    const clean = result.boundaryClean ?? !result.blocked;
    this.snapshot.phase =
      this.metadataFailed || result.blocked || !clean
        ? Phase.BLOCKED
        : !this.accepted && !this.stopped
          ? Phase.IDLE
          : Phase.FINISHED;
    this.snapshot.outcome = wire.Outcome.create({
      execution: !this.accepted && this.stopped ? 'Cancelled' : result.error ? 'Failed' : 'Completed',
      boundaryClean: !!clean,
      writersStopped: this.accepted ? result.writersStopped : null,
      recordingFinalized: this.marker!.recordings.length ? result.recordingFinalized : null,
      eventStreamHealthy: this.accepted ? result.eventStreamHealthy : null,
    });
    this.marker!.phase = this.snapshot.phase;
    this.marker!.outcome = this.snapshot.outcome;
    if (result.error) await this.diagnostic(new Error(result.error));
    try {
      await this.store.write(this.marker!);
      const report = join(this.config.dataRoot, 'runs', this.snapshot.runId, 'console-report.json');
      if (this.executionStarted) await mkdir(dirname(report), { recursive: true, mode: 0o700 });
      if (this.executionStarted)
        await writeOnce(
          report,
          Buffer.from(
            JSON.stringify({
              execution: this.snapshot.outcome.execution,
              writers_stopped: this.snapshot.outcome.writersStopped,
              recording_finalized: this.snapshot.outcome.recordingFinalized,
              boundary_clean: this.snapshot.outcome.boundaryClean,
              event_stream_healthy: this.snapshot.outcome.eventStreamHealthy,
              system_apps: this.snapshot.systemApps?.ids ?? [],
            }),
          ),
        );
      if (this.executionStarted) await this.store.checkpoint(this.enriched());
    } catch {
      this.snapshot.phase = Phase.BLOCKED;
      this.marker!.phase = Phase.BLOCKED;
      await this.store.write(this.marker!).catch(() => {});
    }
  }
  private enriched(): Snapshot {
    const status = wire.Status.fromObject(
      this.feeds.enrich(wire.Status.toObject(this.snapshot, objectOptions)),
    );
    const terminal = [Phase.FINISHED, Phase.BLOCKED].includes(status.phase);
    status.cpuAvailable =
      terminal &&
      status.outcome?.recordingFinalized === true &&
      status.recordings.find((output) => output.cpu)?.path != null &&
      this.store.available(status.runId, status.recordings.find((output) => output.cpu)!.path!);
    status.reportAvailable = terminal && this.store.available(status.runId, 'console-report.json');
    return status;
  }
  async status() {
    await this.tick();
    return Buffer.from(wire.Status.encode(this.enriched()).finish());
  }
  async update(cursor: Buffer) {
    const status = wire.Status.decode(await this.status()),
      position = wire.StreamCursor.decode(cursor);
    const last = status.metrics.at(-1)?.elapsedNs ?? 0,
      cpu = status.cpuSnapshots.at(-1)?.sequence ?? 0;
    const next = wire.StreamCursor.create({
      runId: status.runId,
      sequence: status.sequence,
      metricElapsedNs: last,
      cpuSequence: cpu,
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
      if (integer(cpu) === integer(position.cpuSequence)) {
        status.cpuSnapshots = [];
        status.cpuGroupNames = [];
      }
    }
    return {
      data: Buffer.from(wire.Status.encode(status).finish()),
      cursor: Buffer.from(wire.StreamCursor.encode(next).finish()),
    };
  }
  async openArtifact(id: string, kind: string) {
    if (
      !/^[a-f0-9]{32}$/.test(id) ||
      this.marker?.runId !== id ||
      ![Phase.FINISHED, Phase.BLOCKED].includes(this.snapshot.phase)
    )
      throw new Error('Missing');
    const outputs = this.marker.recordings;
    const name =
      kind === 'report'
        ? 'console-report.json'
        : this.marker.outcome?.recordingFinalized
          ? outputs.find((output) => output.id === kind || (kind === 'cpu' && output.cpu))?.path
          : undefined;
    if (!name) throw new Error('Missing');
    try {
      return capabilities.openFile(join(this.config.dataRoot, 'runs'), `${id}/${name}`);
    } catch {
      throw new Error('Missing');
    }
  }
  close() {
    if (!this.closing) this.closing = this.shutdown();
    return this.closing;
  }
  private async shutdown() {
    try {
      await this.preparing?.catch(() => {});
      this.stopped = true;
      await this.engine.close();
      await this.startPromise?.catch(() => {});
      await this.tick();
    } finally {
      clearInterval(this.timer);
      this.feeds.close();
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
