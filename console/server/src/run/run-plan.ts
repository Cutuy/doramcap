import { join } from 'node:path';
import { artifactFilename } from '../generated/transport.js';
import { launch, type Config, type Project } from '../native.js';
import { outputs, type SelectedApp } from './system-apps.js';

/** Resolved Console inputs shared by both native request builders. */
export interface RunInputs {
  id: string;
  project: Project;
  artifacts: string;
  apps: SelectedApp[];
  workspace: string;
  stage: string;
  graph: string;
  relaySettings?: string;
}

/** Shared run facts; Rust enforces Dora, system apps and recording readiness, then users. */
export function runPlan(config: Config, run: Pick<RunInputs, 'id' | 'project' | 'artifacts' | 'apps' | 'graph'>) {
  return launch.RunPlan.create({
    id: run.id,
    repositoryRoot: run.project.directory,
    artifacts: run.artifacts,
    recordings: outputs(run.apps).map((output) => ({
      recording: join(run.artifacts, output.path!),
      stream: output.stream ? join(run.artifacts, output.stream) : '',
    })),
    readinessSeconds: config.runtime!.readinessSeconds!,
    stopSeconds: config.runtime!.stopSeconds!,
    systemApps: run.apps.map((app) => app.id!),
    mcapApps: run.apps.filter((app) => app.recordings?.length).map((app) => app.id!),
    doraApps: config.doraApps,
    dataflowFile: run.graph,
    hostShmAccess: config.hostShmAccess,
    privateRecords: [artifactFilename('report')],
    writePolicy: config.writePolicy,
    passthroughFolders: config.passthroughFolders,
  });
}
