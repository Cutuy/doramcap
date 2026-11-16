#!/usr/bin/env node
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { closeSync, cpSync, existsSync, mkdirSync, openSync, readFileSync, writeFileSync, appendFileSync } from 'node:fs';
import { request } from 'node:http';
import { EventEmitter, once } from 'node:events';
import { isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { runCommand, WorkerInterrupted } from '../worker-process.mts';

export const UPSTREAM = {
  repository: 'https://github.com/nvidia-holoscan/holoscan-sdk',
  commit: '358c772f2d2e88d117651a961d6f6bf9b650cf8d',
  path: 'examples/first_pipeline/python/first_pipeline.py',
  sha256: 'c7f06121a93ee754b1843144f2bed7ae1fe810fd88c28abe298f381577645fcd',
};
const BASE_IMAGE = 'nvcr.io/nvidia/cuda@sha256:cf5ab24c3f5040a0ea5931658874c2158e606bf740f006809aa4bb9d3334016b';
const SOCKET = '/var/run/docker.sock';
const OPERATION_MS = 30_000, BUILD_MS = 15 * 60_000, STOP_SECONDS = 5;
const SUMMARY = 'first_pipeline: received 10 doubled ticks (last=20) termination=stopped';
const LABEL = 'doramcap.holoscan.probe';
const ARTIFACT = 'build/first_pipeline/applications/first_pipeline/first_pipeline.py';
type Options = { phase: 'preview' | 'real'; cli: string; output: string; mode: 'finite' };
type Event = { Action: string; Actor: { ID: string; Attributes: Record<string, string> } };

/** Require an explicit phase and mode; test output must be a fresh absolute path without mount delimiters. */
export function probeOptions(argv: string[]): Options {
  const { values } = parseArgs({ args: argv, options: {
    phase: { type: 'string' }, cli: { type: 'string' }, output: { type: 'string' }, mode: { type: 'string' },
  } });
  assert(values.phase === 'preview' || values.phase === 'real', '--phase must be preview or real');
  assert.equal(values.mode, 'finite', '--mode finite is required; this fixture has no other mode');
  assert(values.cli && isAbsolute(values.cli), '--cli must name an absolute CLI executable');
  assert(values.output && isAbsolute(values.output) && !/[\s,:]/.test(values.output), '--output must be absolute without whitespace, commas or colons');
  return { phase: values.phase, cli: values.cli, output: resolve(values.output), mode: values.mode };
}

/** Make the native CLI request; no builds occur in run requests and previews always carry --dryrun. */
export function cliArguments(root: string, operation: 'build' | 'run', mode: string, image: string, preview: boolean, dockerOptions?: string, volume?: string): string[] {
  return ['--project-root', root, operation, 'first_pipeline', mode, '--language', 'python', '--img', image,
    ...(operation === 'build' ? ['--base-img', BASE_IMAGE] : ['--no-docker-build', '--no-local-build', '--persistent']),
    ...(dockerOptions ? [`--docker-opts=${dockerOptions}`] : []), ...(volume ? ['--add-volume', volume] : []),
    ...(preview ? ['--dryrun'] : [])];
}

async function engine(path: string, method = 'GET'): Promise<Buffer> {
  return await new Promise((resolveRequest, reject) => {
    const req = request({ socketPath: SOCKET, path, method, signal: AbortSignal.timeout(OPERATION_MS) }, response => {
      const chunks: Buffer[] = [];
      response.on('data', chunk => chunks.push(chunk));
      response.once('error', reject);
      response.once('end', () => {
        const body = Buffer.concat(chunks);
        if (response.statusCode && response.statusCode >= 200 && response.statusCode < 300) resolveRequest(body);
        else reject(new Error(`Engine ${method} ${path}: HTTP ${response.statusCode}: ${body}`));
      });
    });
    req.once('error', reject); req.end();
  });
}

async function watch(label: string, output: string) {
  const events: Event[] = [], bus = new EventEmitter();
  let pending = '', failure: Error | undefined;
  bus.on('error', () => {});
  const req = request({ socketPath: SOCKET, path: '/events?filters=' + encodeURIComponent(JSON.stringify({ label: [`${LABEL}=${label}`] })) });
  const ready = new Promise<void>((resolveReady, reject) => {
    req.once('error', reject);
    req.once('response', response => {
      if (response.statusCode !== 200) { reject(new Error(`Engine events HTTP ${response.statusCode}`)); response.resume(); return; }
      response.setEncoding('utf8');
      response.on('data', (chunk: string) => {
        pending += chunk;
        let newline: number;
        while ((newline = pending.indexOf('\n')) >= 0) {
          const line = pending.slice(0, newline); pending = pending.slice(newline + 1);
          if (!line) continue;
          try { const event: Event = JSON.parse(line); events.push(event); appendFileSync(output, line + '\n'); bus.emit(event.Action, event); }
          catch (error) { failure = error as Error; bus.emit('error', failure); }
        }
      });
      response.once('error', error => { failure = error; bus.emit('error', error); });
      resolveReady();
    });
  });
  req.end();
  await Promise.race([ready, new Promise<never>((_, reject) => {
    const timer = setTimeout(() => { req.destroy(); reject(new Error('Engine event subscription deadline')); }, OPERATION_MS);
    timer.unref(); ready.finally(() => clearTimeout(timer)).catch(() => {});
  })]);
  return {
    events,
    async wait(action: string): Promise<Event> {
      if (failure) throw failure;
      const previous = events.find(event => event.Action === action);
      return previous ?? (await once(bus, action, { signal: AbortSignal.timeout(OPERATION_MS) }))[0] as Event;
    },
    close() { req.destroy(); },
  };
}

/** Exercise only owned temporary resources; write raw commands/results before reporting any failed assumption. */
export async function probe(options: Options): Promise<void> {
  assert(existsSync(options.cli), 'CLI executable is missing');
  assert(!existsSync(options.output), 'Output already exists; choose a new evidence directory');
  if (options.phase === 'real') assert.equal(process.getuid?.(), 0, 'Real probes require sudo for host OverlayFS and Docker access');
  const fixture = join(import.meta.dirname, 'first_pipeline');
  const sample = readFileSync(join(fixture, 'applications/first_pipeline/first_pipeline.py'));
  assert.equal(createHash('sha256').update(sample).digest('hex'), UPSTREAM.sha256, 'Official example changed');
  mkdirSync(options.output, { recursive: true });
  const source = join(options.output, 'source'), data = join(options.output, 'original-data/payload');
  cpSync(fixture, source, { recursive: true }); mkdirSync(data, { recursive: true });
  writeFileSync(join(data, 'input.txt'), 'holoscan-extra-volume\n');
  const owner = randomUUID(), image = `doramcap-holoscan-probe:${owner}`, mounts: string[] = [];
  const report: Record<string, unknown> = { upstream: UPSTREAM, options, image, owner, qualification: 'not_run', commands: [], scenarios: [], cleanup: [] };
  const save = () => writeFileSync(join(options.output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  const interruption = new AbortController();
  let cleaningUp = false;
  const handlers = (['SIGINT', 'SIGTERM'] as const).map(signal => {
    const handler = () => interruption.abort(new WorkerInterrupted(signal)); process.once(signal, handler); return [signal, handler] as const;
  });
  async function command(name: string, program: string, args: string[], timeout = OPERATION_MS, required = true) {
    console.log(`${name}: ${program} ${args.join(' ')}`);
    const log = join(options.output, `${name}.log`), output = openSync(log, 'wx', 0o644);
    const record: Record<string, unknown> = { name, program, args, cwd: source, log, started: new Date().toISOString() };
    (report.commands as unknown[]).push(record); save();
    try {
      const result = await runCommand(program, args, { cwd: source, env: process.env, output,
        signal: cleaningUp ? AbortSignal.timeout(timeout) : AbortSignal.any([interruption.signal, AbortSignal.timeout(timeout)]) });
      record.code = result.code;
      if (required) assert.equal(result.code, 0, `${name} failed; see ${log}`);
      return result.code;
    } catch (error) { record.error = (error as Error).message; throw error; }
    finally { closeSync(output); record.finished = new Date().toISOString(); save(); }
  }
  async function overlay(lower: string, name: string, basename: string) {
    const base = join(options.output, 'overlays', name), merged = join(base, basename);
    for (const directory of ['upper', 'work', basename]) mkdirSync(join(base, directory), { recursive: true });
    await command(`mount-${name}`, 'mount', ['-t', 'overlay', 'overlay', '-o', `lowerdir=${lower},upperdir=${base}/upper,workdir=${base}/work`, merged]);
    mounts.push(merged); return merged;
  }
  async function scenario(name: string, root: string, volume: string, controls: boolean, stop: boolean, readVolume = false) {
    const label = `${owner}-${name}`, cidfile = join(options.output, `${name}.cid`);
    const dockerOptions = `--label ${LABEL}=${label} --cidfile ${cidfile}` + (controls ? ' --cpus 1 --log-driver journald' : '');
    const events = await watch(label, join(options.output, `${name}-events.jsonl`));
    try {
      const stopped = stop ? events.wait('start').then(async event => {
        await engine(`/containers/${event.Actor.ID}/stop?t=${STOP_SECONDS}`, 'POST'); return event.Actor.ID;
      }) : undefined;
      stopped?.catch(() => {});
      const args = readVolume
        ? ['--project-root', root, 'run-container', 'first_pipeline', options.mode, '--language', 'python', '--img', image,
          '--no-docker-build', '--persistent', `--docker-opts=${dockerOptions}`, '--add-volume', volume, '--', 'cat', '/workspace/volumes/payload/input.txt']
        : cliArguments(root, 'run', options.mode, image, false, dockerOptions, volume);
      const cliCode = await command(`${name}-cli`, options.cli, args, OPERATION_MS, !stop);
      if (stopped) { await stopped; await events.wait('stop'); }
      const died = await events.wait('die'), id = readFileSync(cidfile, 'utf8').trim();
      assert.equal(id, died.Actor.ID, 'CID file and Engine events differ');
      const details = JSON.parse((await engine(`/containers/${id}/json`)).toString());
      writeFileSync(join(options.output, `${name}-inspect.json`), JSON.stringify(details, null, 2) + '\n');
      assert.equal(details.Config.Labels[LABEL], label); assert.equal(details.State.Running, false);
      assert.equal(cliCode, details.State.ExitCode, 'CLI exit and application/container exit differ');
      assert.equal(details.Image, report.imageId, 'Created image differs from built image');
      for (const [host, destination] of [[root, '/workspace/holohub'], [volume, '/workspace/volumes/payload']]) {
        assert(details.Mounts.some((mount: { Source: string; Destination: string }) => mount.Source === host && mount.Destination === destination), `Missing native mount ${host} -> ${destination}`);
      }
      assert(events.events.some(event => event.Action === 'create')); assert(events.events.some(event => event.Action === 'start'));
      if (controls) {
        assert.equal(details.HostConfig.NanoCpus, 1_000_000_000); assert.equal(details.HostConfig.LogConfig.Type, 'journald');
      }
      const logs = await engine(`/containers/${id}/logs?stdout=1&stderr=1&tail=50`);
      writeFileSync(join(options.output, `${name}-engine-logs.bin`), logs);
      if (!stop) assert(logs.toString().includes(readVolume ? 'holoscan-extra-volume' : SUMMARY), 'Engine tail is missing expected application output');
      if (controls) {
        await command(`${name}-journal`, 'journalctl', ['--no-pager', '-o', 'json', `CONTAINER_ID_FULL=${id}`]);
        if (!stop) assert(readFileSync(join(options.output, `${name}-journal.log`), 'utf8').includes(readVolume ? 'holoscan-extra-volume' : SUMMARY), 'Host journal is missing application output');
      }
      (report.scenarios as unknown[]).push({ name, container: id, cliCode, exitCode: details.State.ExitCode, stopRequested: stop, outcome: 'passed', actions: events.events.map(event => event.Action) }); save();
    } finally { events.close(); }
  }
  try {
    await command('disk-before', 'df', ['-h', options.output]);
    await command('gpu', 'nvidia-smi', ['--query-gpu=name,driver_version,memory.total,memory.free', '--format=csv,noheader'], OPERATION_MS, false);
    await command('docker', 'docker', ['info', '--format', '{{json .}}'], OPERATION_MS, options.phase === 'real');
    await command('cli-version', options.cli, ['version', '--json']);
    assert.equal(JSON.parse(readFileSync(join(options.output, 'cli-version.log'), 'utf8')).version, '5.0.0');
    await command('catalog', options.cli, ['--project-root', source, 'list', '--json']);
    for (const operation of ['build', 'run'] as const) await command(`${operation}-dryrun`, options.cli, cliArguments(source, operation, options.mode, image, true));
    report.preview = 'passed'; save();
    if (options.phase === 'preview') return;
    await command('build', options.cli, cliArguments(source, 'build', options.mode, image, false), BUILD_MS);
    await command('disk-after-build', 'df', ['-h', options.output]);
    const builtImage = JSON.parse((await engine(`/images/${encodeURIComponent(image)}/json`)).toString());
    report.imageId = builtImage.Id; report.imageBytes = builtImage.Size; save();
    assert.equal(createHash('sha256').update(readFileSync(join(source, ARTIFACT))).digest('hex'), UPSTREAM.sha256, 'Build output differs from upstream source');
    await scenario('plain', source, data, false, false);
    const merged = await overlay(source, 'project', 'project'), extra = await overlay(data, 'data', 'payload');
    await scenario('overlay', merged, extra, true, false);
    await scenario('extra-volume', merged, extra, true, false, true);
    await scenario('engine-stop', merged, extra, true, true);
    report.qualification = 'lightweight_lifecycle_passed';
  } catch (error) { report.failure = (error as Error).stack; throw error; }
  finally {
    cleaningUp = true;
    if (options.phase === 'real') {
      const cleanup = report.cleanup as unknown[];
      try {
        const containers = JSON.parse((await engine('/containers/json?all=1&filters=' + encodeURIComponent(JSON.stringify({ label: [LABEL] })))).toString());
        for (const container of containers) {
          if (!String(container.Labels[LABEL]).startsWith(owner + '-')) continue;
          await engine(`/containers/${container.Id}?force=1`, 'DELETE'); cleanup.push({ container: container.Id, removed: true });
        }
        const remaining = JSON.parse((await engine('/containers/json?all=1&filters=' + encodeURIComponent(JSON.stringify({ label: [LABEL] })))).toString());
        assert(!remaining.some((container: { Labels: Record<string, string> }) => container.Labels[LABEL]?.startsWith(owner + '-')), 'Owned containers remain');
      } catch (error) { cleanup.push({ containersError: (error as Error).message }); process.exitCode = 1; }
      for (const mount of mounts.reverse()) {
        try { await command(`unmount-${mount.endsWith('payload') ? 'data' : 'project'}`, 'umount', [mount]); cleanup.push({ mount, unmounted: true }); }
        catch (error) { cleanup.push({ mount, error: (error as Error).message }); process.exitCode = 1; }
      }
      await command('disk-after', 'df', ['-h', options.output], OPERATION_MS, false);
    }
    for (const [signal, handler] of handlers) process.removeListener(signal, handler);
    save(); console.log(`Evidence: ${join(options.output, 'report.json')}`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { await probe(probeOptions(process.argv.slice(2))); }
  catch (error) { console.error((error as Error).stack); process.exitCode = 1; }
}
