/** Run one repair CLI in a lease already held by the controller; record, never accept, its patch. */
import { spawn, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createWriteStream } from 'node:fs';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url)), repo = resolve(here, '../..');
const [runName] = process.argv.slice(2);
if (!runName || !/^[a-zA-Z0-9_-]+$/.test(runName)) throw new Error('Usage: node fix.ts RUN_NAME');
const run = join(here, 'fixes', runName), raw = join(run, 'raw');
const hash = (s: string) => createHash('sha256').update(s).digest('hex');
const git = (...args: string[]) => execFileSync('git', args, { cwd: repo, encoding: 'utf8' }).trim();
const changedPaths = () => [...new Set([
  ...git('diff', '--name-only', 'HEAD').split('\n'),
  ...git('ls-files', '--others', '--exclude-standard').split('\n'),
])].filter(p => p && !p.startsWith('cd/code-scan/')).sort();
const taskText = await readFile(join(run, 'FixTask.json'), 'utf8');
const task = JSON.parse(taskText);
if (task.lease.managed_by !== 'controller' || repo !== task.lease.path || git('branch', '--show-current') !== task.lease.branch || git('rev-parse', 'HEAD') !== task.checkout_head) throw new Error('Checkout identity mismatch');
const pool = JSON.parse(execFileSync(process.execPath, [join(repo, '.agents/skills/worktree-pool/scripts/pool.ts'), 'status'], { cwd: repo, encoding: 'utf8' }));
if (!pool.slots.some((s: any) => s.path === repo && s.lease?.owner === task.lease.owner && s.lease?.branch === task.lease.branch && s.lease?.state === 'active')) throw new Error('Controller lease is not active');
if (changedPaths().length) throw new Error('Source changes exist before the repair');
await mkdir(raw, { recursive: true });
const provenance = await Promise.all([...task.governing_files, ...task.inspect_first, task.finding_path].map(async (path: string) => ({ path, sha256: hash(await readFile(join(repo, path), 'utf8')) })));
const args = ['exec', '--ignore-user-config', '--ephemeral', '--cd', repo,
  '--model', task.execution.model, '--approve-for-me',
  '-c', `model_reasoning_effort="${task.execution.reasoning_effort}"`, '-c', 'project_doc_max_bytes=0',
  ...['apps', 'plugins', 'multi_agent', 'memories', 'skill_search', 'browser_use', 'computer_use', 'image_generation', 'view_image', 'goals', 'sleep_tool'].flatMap(n => ['--disable', n]),
  '--enable', 'skip_host_skill_discovery', '--output-schema', join(here, 'fix.schema.json'),
  '--output-last-message', join(raw, 'answer.json'), '--json', '--color', 'never', '-'];
const request = `${await readFile(join(here, 'fix.prompt.txt'), 'utf8')}\n${taskText}`;
const before = performance.now(), started = new Date().toISOString();
await writeFile(join(run, 'invocation.json'), `${JSON.stringify({ task_id: task.task_id, started_at: started,
  command: ['codex', ...args], task_sha256: hash(taskText), request_sha256: hash(request),
  schema_sha256: hash(await readFile(join(here, 'fix.schema.json'), 'utf8')), provenance,
  lease_managed_by: 'controller', max_concurrency: 1 }, null, 2)}\n`, { flag: 'wx' });
const out = createWriteStream(join(raw, 'events.jsonl')), err = createWriteStream(join(raw, 'stderr.txt'));
const child = spawn('codex', args, { cwd: repo, stdio: ['pipe', 'pipe', 'pipe'] });
child.stdout.pipe(out); child.stderr.pipe(err);
child.stdin.on('error', () => {}); child.stdin.end(request);
console.log(JSON.stringify({ task_id: task.task_id, pid: child.pid, model: task.execution.model, effort: task.execution.reasoning_effort, lease: task.lease.path }));
const code = await new Promise<number | null>((ok, fail) => { child.once('error', fail); child.once('close', ok); });
await Promise.all([out, err].map(stream => stream.writableFinished ? Promise.resolve() : new Promise<void>(ok => stream.once('finish', ok))));
const events = (await readFile(join(raw, 'events.jsonl'), 'utf8')).split('\n').filter(Boolean).flatMap(s => { try { return [JSON.parse(s)]; } catch { return []; } });
const actualPaths = changedPaths();
const record: any = { task_id: task.task_id, requested_model: task.execution.model, requested_reasoning_effort: task.execution.reasoning_effort,
  provider_reported_model: null, cli_version: execFileSync('codex', ['--version'], { encoding: 'utf8' }).trim(),
  started_at: started, completed_at: new Date().toISOString(), duration_seconds: (performance.now() - before) / 1000,
  exit_code: code, usage: events.find(e => e.type === 'turn.completed')?.usage ?? null,
  thread_id: events.find(e => e.type === 'thread.started')?.thread_id ?? null,
  completed_tool_items: events.filter(e => e.type === 'item.completed' && e.item?.type !== 'agent_message').map(e => ({ id: e.item.id, type: e.item.type, exit_code: e.item.exit_code ?? null })),
  actual_changed_paths: actualPaths, patch_sha256: hash(git('diff', 'HEAD', '--', ...task.allowed_edit_prefixes)),
  status: 'unresolved', result: null, error: null, controller_verification: 'pending' };
try {
  if (code !== 0) throw new Error(`CLI exited ${code}; inspect raw logs`);
  execFileSync('python3', ['-c', 'import json,jsonschema,sys; jsonschema.validate(json.load(open(sys.argv[1])),json.load(open(sys.argv[2])))', join(raw, 'answer.json'), join(here, 'fix.schema.json')]);
  const answer = JSON.parse(await readFile(join(raw, 'answer.json'), 'utf8'));
  if (answer.task_id !== task.task_id) throw new Error('Task identity mismatch');
  if (git('rev-parse', 'HEAD') !== task.checkout_head || git('branch', '--show-current') !== task.lease.branch) throw new Error('Checkout identity changed');
  if (actualPaths.some(p => !task.allowed_edit_prefixes.some((prefix: string) => p.startsWith(prefix)))) throw new Error('Out-of-scope source changes; preserve for inspection');
  if (JSON.stringify([...answer.changed_paths].sort()) !== JSON.stringify(actualPaths)) throw new Error('Reported source changes differ from actual changes');
  for (const e of answer.evidence) {
    const path = resolve(repo, e.path);
    if (!path.startsWith(`${repo}/`)) throw new Error('Evidence path outside checkout');
    const content = await readFile(path, 'utf8');
    if (!e.quote || !content.split('\n').slice(e.line - 1, e.line - 1 + e.quote.split('\n').length).join('\n').includes(e.quote)) throw new Error(`Invalid evidence ${e.path}:${e.line}`);
  }
  if (answer.status === 'needs_decision' && (!answer.decisions.length || actualPaths.length)) throw new Error('Decision result must identify a decision and precede source edits');
  if (answer.status === 'patch_ready' && (!actualPaths.length || answer.decisions.length)) throw new Error('Patch result must contain changes with no pending decision');
  for (const d of answer.decisions) if (!d.options.some((o: any) => o.id === d.recommended_option)) throw new Error('Invalid recommended option');
  record.status = answer.status; record.result = answer;
  await writeFile(join(run, 'PatchProposal.json'), `${JSON.stringify(answer, null, 2)}\n`, { flag: 'wx' });
} catch (e) { record.error = String(e); }
await writeFile(join(run, 'result.json'), `${JSON.stringify(record, null, 2)}\n`, { flag: 'wx' });
console.log(JSON.stringify({ status: record.status, seconds: record.duration_seconds, usage: record.usage, changed_paths: actualPaths, error: record.error }));
