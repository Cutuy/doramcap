import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { launch, live, wire } from '../src/native.js';
import { selectApps } from '../src/system-apps.js';
import { prepareRelay } from '../src/live.js';
import { buildRunRequest } from '../src/run-request.js';

for (const form of [launch.Form.COMPOSE, launch.Form.SYSTEMD]) {
  test(`${launch.Form[form]} preserves selected bindings and owns its native inputs`, async () => {
    // Arrange
    const workspace = await mkdtemp(join(tmpdir(), 'console-run-request-'));
    const artifacts = join(workspace, 'artifacts');
    const selectedConfig = join(workspace, 'capture.pbtxt');
    const config = wire.ConsoleConfig.create({
      form, binaries: '/opt/bin',
      hostShmAccess: launch.HostShmAccess.HOST_SHM_ACCESS_PASSTHROUGH,
      runtime: { readinessSeconds: 60, stopSeconds: 30 },
      apps: [
        {
          id: 'capture', process: { argv: ['capture'] },
          input: { directory: workspace, files: ['capture.yaml'], envFiles: ['app.env'], profiles: ['record'] },
          configMount: { service: 'writer', target: '/config/capture.pbtxt' },
          outputMounts: [{ service: 'writer', target: '/results' }, { service: 'helper', target: '/scratch' }],
          launchProfiles: [{ id: 'record', config: selectedConfig, recordings: [{ id: 'capture', path: 'capture.mcap' }] }],
          liveOutputs: [{ source: 'capture/messages', kind: live.Kind.CPU_ACCOUNTING }],
          files: [{ source: join(workspace, 'shared.pbtxt'), target: '/config/shared.pbtxt' }],
          systemd: { files: [{ source: join(workspace, 'native.pbtxt'), target: '/config/native.pbtxt' }] },
        },
        {
          id: 'live-bridge', process: { argv: ['live-bridge'] },
          input: { directory: workspace, files: ['bridge.yaml'] },
          configMount: { service: 'live-bridge', target: '/config/bridge.pbtxt' },
          liveRelay: { port: 43190 },
        },
        { id: 'doramcap-shm', input: { directory: workspace, files: ['shm.yaml'] } },
      ],
    });
    const apps = selectApps(config.apps, ['capture', 'live-bridge'], { capture: 'record' });
    const project = { key: 'workload', directory: workspace, files: ['compose.yaml'], profiles: ['extra'] };
    try {
      for (const stage of ['review', 'execution']) {
        // Act
        const relay = await prepareRelay(apps[1], apps, workspace, stage);
        const request = await buildRunRequest(config, {
          id: 'run', project, apps, artifacts, workspace, stage,
          graph: relay.graph, relaySettings: relay.settings,
        });

        // Assert
        assert.equal(request.run!.repositoryRoot, workspace);
        assert.equal(request.run!.dataflowFile, relay.graph);
        assert.deepEqual(request.run!.recordings!.map(output => output.recording), [join(artifacts, 'capture.mcap')]);
        assert.deepEqual(config.apps[0].input!.files, ['capture.yaml']);
        const graph = JSON.parse(await readFile(relay.graph, 'utf8'));
        assert.equal(graph.nodes[0].inputs.capture_0.source, 'capture/messages');
        assert.match(await readFile(relay.settings, 'utf8'), /^port: 43190\n/);
        if (form === launch.Form.COMPOSE) {
          assert.equal(request.systemd, null);
          assert.deepEqual(request.run!.systemApps, ['capture', 'live-bridge', 'doramcap-shm']);
          assert.deepEqual(request.compose!.projects!.map(input => input.key), ['workload', 'capture', 'live-bridge', 'doramcap-shm']);
          assert.deepEqual(request.compose!.projects![0], project);
          const capture = request.compose!.projects![1];
          assert.deepEqual(capture.envFiles, ['app.env']);
          assert.deepEqual(capture.profiles, ['record']);
          const overlay = JSON.parse(await readFile(capture.files!.at(-1)!, 'utf8'));
          assert.deepEqual(overlay.services, {
            writer: { volumes: [
              { type: 'bind', source: artifacts, target: '/results', bind: { create_host_path: false } },
              { type: 'bind', source: selectedConfig, target: '/config/capture.pbtxt', read_only: true, bind: { create_host_path: false } },
            ] },
            helper: { volumes: [{ type: 'bind', source: artifacts, target: '/scratch', bind: { create_host_path: false } }] },
          });
          const bridge = request.compose!.projects![2];
          const bridgeOverlay = JSON.parse(await readFile(bridge.files!.at(-1)!, 'utf8'));
          assert.deepEqual(bridgeOverlay.services['live-bridge'].volumes, [
            { type: 'bind', source: relay.settings, target: '/config/bridge.pbtxt', read_only: true, bind: { create_host_path: false } },
          ]);
        } else {
          assert.equal(request.compose, null);
          assert.deepEqual(request.run!.systemApps, ['capture', 'live-bridge']);
          assert.deepEqual(request.systemd!.programs!.map(program => program.id), ['capture', 'live-bridge']);
          assert.deepEqual(request.systemd!.programs![0].binds, [
            { source: selectedConfig, target: '/config/capture.pbtxt', readOnly: true },
            { source: artifacts, target: '/results', readOnly: false },
            { source: artifacts, target: '/scratch', readOnly: false },
            { source: join(workspace, 'shared.pbtxt'), target: '/config/shared.pbtxt', readOnly: true },
            { source: join(workspace, 'native.pbtxt'), target: '/config/native.pbtxt', readOnly: true },
          ]);
          assert.deepEqual(request.systemd!.programs![1].binds, [
            { source: relay.settings, target: '/config/bridge.pbtxt', readOnly: true },
          ]);
          assert.deepEqual((await readdir(workspace)).filter(file => file.endsWith('.json') && !file.endsWith('.graph.json')), []);
        }
      }
    } finally {
      await rm(workspace, { recursive: true, force: true });
    }
  });
}

test('systemd retains its missing-config rejection while Compose preserves native config', async () => {
  // Arrange
  const workspace = await mkdtemp(join(tmpdir(), 'console-run-config-'));
  const config = wire.ConsoleConfig.create({
    form: launch.Form.SYSTEMD, binaries: '/opt/bin', runtime: { readinessSeconds: 60, stopSeconds: 30 },
    apps: [{
      id: 'capture', process: { argv: ['capture'] },
      input: { directory: workspace, files: ['capture.yaml'] },
      configMount: { service: 'writer', target: '/config/capture.pbtxt' },
    }],
  });
  const run = { id: 'run', project: { directory: workspace }, apps: config.apps, artifacts: join(workspace, 'artifacts'), workspace, stage: 'review', graph: '' };
  try {
    // Act / Assert
    await assert.rejects(buildRunRequest(config, run), /capture: nothing to show at \/config\/capture.pbtxt/);
    config.form = launch.Form.COMPOSE;
    const request = await buildRunRequest(config, run);
    assert.deepEqual(request.compose!.projects![1].files, ['capture.yaml']);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});
