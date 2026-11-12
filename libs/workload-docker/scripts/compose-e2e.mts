#!/usr/bin/env node
/** Run shared-Dora workflow tests using a prebuilt doramcap/dora:local image. */
import { spawnSync } from 'node:child_process';
import { closeSync, copyFileSync, mkdirSync, mkdtempSync, openSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const root = resolve(import.meta.dirname, '../../..');
function run(argv: string[], capture = false, stdout?: number, stdin?: number): string {
  const result = spawnSync(argv[0], argv.slice(1), {
    cwd: root, encoding: 'utf8', maxBuffer: Infinity, stdio: [stdin ?? 'ignore', stdout ?? (capture ? 'pipe' : 'inherit'), stdout ?? 'inherit'],
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${argv[0]} failed (${result.signal ?? result.status})`);
  return typeof result.stdout === 'string' ? result.stdout : '';
}
function main() {
  const { values } = parseArgs({ options: {
    output: { type: 'string' }, profile: { type: 'string', default: 'release' },
    socket: { type: 'string', default: '/var/run/docker.sock' }, help: { type: 'boolean' },
  } });
  if (values.help) { console.log('Usage: node compose-e2e.mts --output NEW_DIRECTORY [--profile release|ci] [--socket PATH]'); return; }
  if (!values.output) throw new Error('--output is required');
  const output = resolve(values.output), socket = resolve(values.socket!);
  mkdirSync(dirname(output), { recursive: true });
  mkdirSync(output);
  const docker = ['docker', '--host', `unix://${socket}`];
  const mainCheckout = run(['git', 'worktree', 'list', '--porcelain'], true).split('\n')[0].replace(/^worktree /, '');
  const credential = join(mainCheckout, '.sudo-password');
  let provisioned = false;
  try { const metadata = statSync(credential); provisioned = metadata.isFile() && (metadata.mode & 0o777) === 0o600; } catch { /* Local provisioning is required below. */ }
  if (!provisioned) throw new Error('Provision main .sudo-password locally, mode 0600');
  run(['git', '-C', mainCheckout, 'check-ignore', '-q', '.sudo-password']);
  function elevated(argv: string[], log: number) {
    const secret = openSync(credential, 'r');
    try { run(['sudo', '-S', '-p', '', '--', ...argv], false, log, secret); }
    finally { closeSync(secret); }
  }
  const cargo = ['--locked', '--profile', values.profile!];
  run(['cargo', 'build', ...cargo, '-p', 'mcap-recorder', '--bins', '--examples']);
  const built = run(['cargo', 'test', ...cargo, '-p', 'workload-docker', '--test', 'docker', '--no-run', '--message-format=json'], true);
  const records = built.trim().split('\n').map((line: string) => JSON.parse(line) as { executable?: string });
  const executable = records.find(record => record.executable)?.executable;
  if (!executable) throw new Error('Cargo did not return the Docker test executable');
  const stable = join(output, 'native-tests');
  copyFileSync(executable, stable);
  const metadata = JSON.parse(run(['cargo', 'metadata', '--locked', '--no-deps', '--format-version', '1'], true)) as { target_directory: string };
  const target = join(metadata.target_directory, values.profile === 'dev' ? 'debug' : values.profile!);
  const fixture = join(target, 'examples/dora_attachment_fixture'), recorder = join(target, 'mcap-recorder');
  const image = `doramcap-compose-test:shared-${process.pid}`;
  const stage = mkdtempSync(join(output, 'compose-image-'));
  try {
    const filesystem = join(stage, 'rootfs'); mkdirSync(filesystem);
    const programs = [recorder, fixture, '/bin/sh', '/bin/sleep', '/bin/true', '/bin/false', '/bin/rm', '/bin/touch', '/usr/bin/test'];
    function copy(source: string, destination: string) { mkdirSync(dirname(destination), { recursive: true }); copyFileSync(source, destination); }
    for (const source of programs) {
      if (source !== recorder && source !== fixture) copy(source, join(filesystem, 'bin', basename(source)));
      for (const line of run(['ldd', source], true).split('\n')) {
        const library = line.match(/(?:=>\s+)?(\/\S+)/)?.[1];
        if (library) copy(library, join(filesystem, relative('/', library)));
      }
    }
    mkdirSync(join(filesystem, 'tmp'));
    writeFileSync(join(stage, 'Dockerfile'), 'FROM scratch\nCOPY rootfs /\nENV HOME=/tmp\nENTRYPOINT ["/bin/sh"]\n');
    run([...docker, 'build', '--network=none', '-t', image, stage]);
  } finally { rmSync(stage, { recursive: true }); }
  const environment = ['env', `TMPDIR=${output}`, `DORAMCAP_TEST_IMAGE=${image}`, `DORAMCAP_TEST_SOCKET=${socket}`,
    `DORAMCAP_TEST_DORA_NODE=${fixture}`, `DORAMCAP_TEST_MCAP_RECORDER=${recorder}`];
  const cases: [string, number][] = [
    ['public_dora_', 8], ['direct_session_lifecycle_and_prerun_reset', 1],
    ['stopping_a_run_respects_each_service_grace_period', 1], ['parallel_projects_preserve_readiness_and_shutdown_barriers', 1],
    ['active_experiments_are_exclusive_per_repository_and_independent_between_repositories', 1],
    ['reset_failure_blocks_preparation_and_retries_after_partial_removal', 1],
    ['stable_project_keeps_retained_volumes_isolated_between_experiments', 1], ['selected_profile_starts_its_service', 1],
    ['reset_discards_fixed_workspace_without_launch_history', 1], ['shared_experiment_preserves_native_inputs_and_rejects_conflicts', 1],
    ['mixed_volume_policies_and_evidence_preservation', 1], ['untracked_named_volume_keeps_declared_cleanup_policy', 1],
    ['whole_run_mount_excludes_private_records_supplied_by_each_owner', 1],
  ];
  for (const [name, expected] of cases) {
    const path = join(output, `${name}.log`), log = openSync(path, 'wx');
    try { elevated([...environment, stable, name, '--ignored', '--nocapture', '--test-threads=1'], log); }
    catch { throw new Error(`FAIL: ${name}; evidence and owned resources retained; see ${path}`); }
    finally { closeSync(log); }
    const results = [...readFileSync(path, 'utf8').matchAll(/^test result: ok\. (\d+) passed;/gm)];
    if (Number(results.at(-1)?.[1]) !== expected) throw new Error(`FAIL: ${name} did not run ${expected} tests; see ${path}`);
    console.log(`PASS: ${name}`);
  }
  run([...docker, 'image', 'rm', image]);
  console.log(`PASS: shared-Dora workflow; evidence ${output}`);
}
try { main(); } catch (error) { console.error((error as Error).message); process.exitCode = 1; }
