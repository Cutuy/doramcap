/** Confirm individual screening candidates using frozen JSON evidence and native CLI output schemas. */
import { spawn, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile, readdir, mkdtemp, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '../..');
const model = 'gpt-6.1-sol', effort = 'high';
const hash = (s: string) => createHash('sha256').update(s).digest('hex');
const readJson = async (p: string) => JSON.parse(await readFile(p, 'utf8'));
const json = async (p: string, v: unknown, exclusive = false) => writeFile(p, `${JSON.stringify(v, null, 2)}\n`, { flag: exclusive ? 'wx' : 'w' });
const sourceAt = (revision: string, path: string) => execFileSync('git', ['show', `${revision}:${path}`], { cwd: repo, encoding: 'utf8' });
function artifact(id: string, content: string, origin: string) {
  return { artifact_id: id, sha256: hash(content), origin, content };
}
async function prepare(run: string) {
  const spec = await readJson(join(run, 'context.json'));
  const screen = join(here, 'runs', spec.screen_run);
  const manifest = await readJson(join(screen, 'manifest.json'));
  const revision = manifest.base_commit;
  const guide = sourceAt(revision, 'docs/impl-guide.md');
  const rule = guide.match(/## I-05[^]*?(?=\n## |$)/)?.[0];
  if (!rule || hash(rule) !== manifest.rule_hash) throw new Error('Rule revision mismatch');
  const taskEntries: any[] = [];
  await mkdir(join(run, 'inputs'), { recursive: true });
  await mkdir(join(run, 'raw'), { recursive: true });
  for (const [taskId, context] of Object.entries(spec.tasks) as [string, any][]) {
    const candidate = await readJson(join(screen, `${taskId}.json`));
    if (candidate.status !== 'candidate') throw new Error(`${taskId} is not a candidate`);
    const artifacts = context.sources.map((path: string) => artifact(path, sourceAt(revision, path), `git:${revision}:${path}`));
    for (const path of context.history) artifacts.push(artifact(path, await readFile(join(run, path), 'utf8'), path));
    const primary = artifacts.find((a: any) => a.artifact_id === candidate.path);
    if (!primary || primary.sha256 !== candidate.source_hash) throw new Error('Candidate source mismatch');
    const input = { task_id: taskId, round: 1, source_commit: revision,
      rule: { id: manifest.rule_id, text: rule, focus: 'Repeated sleep/timed-wait/park/yield used to poll state outside tests; sampling, pacing, simulated work and real deadlines are permitted timer uses.' },
      candidate: candidate.result, evidence: artifacts, prior_decisions: [], scope: 'Confirm this candidate only. No fixes or unrelated findings.' };
    await json(join(run, 'inputs', `${taskId}.r1.json`), input, true);
    taskEntries.push({ task_id: taskId, round: 1, path: candidate.path, source_hash: candidate.source_hash,
      artifacts: artifacts.map(({ artifact_id, sha256, origin }: any) => ({ artifact_id, sha256, origin })) });
  }
  await json(join(run, 'manifest.json'), { source_commit: revision, rule_id: manifest.rule_id, rule_hash: manifest.rule_hash,
    requested_model: model, reasoning_effort: effort, max_concurrency: 2, tasks: taskEntries }, true);
  console.log(JSON.stringify({ prepared_tasks: taskEntries.length, source_commit: revision }));
}
async function invoke(run: string, file: string, cwd: string) {
  const input = await readJson(join(run, 'inputs', file));
  const key = file.slice(0, -5);
  const disabled = ['shell_tool', 'unified_exec', 'apps', 'plugins', 'multi_agent', 'memories', 'skill_search', 'browser_use', 'computer_use', 'image_generation', 'view_image', 'goals', 'sleep_tool'];
  const answerPath = join(run, 'raw', `${key}.answer.json`);
  const args = ['exec', '--ignore-user-config', '--ephemeral', '--skip-git-repo-check', '--cd', cwd,
    '--model', model, '--sandbox', 'read-only', '-c', 'approval_policy="never"', '-c', `model_reasoning_effort="${effort}"`,
    '-c', 'project_doc_max_bytes=0', ...disabled.flatMap(name => ['--disable', name]), '--enable', 'skip_host_skill_discovery',
    '--output-schema', join(here, 'confirm.schema.json'), '--output-last-message', answerPath, '--json', '--color', 'never', '-'];
  const displayInput = { ...input, evidence: input.evidence.map((a: any) => ({ ...a,
    content: a.content.split('\n').map((line: string, i: number) => `${i + 1}: ${line}`).join('\n') })) };
  const prompt = `${await readFile(join(here, 'confirm.prompt.txt'), 'utf8')}\n${JSON.stringify(displayInput)}`;
  const before = performance.now(), started = new Date().toISOString();
  const child = spawn('codex', args, { cwd, stdio: ['pipe', 'pipe', 'pipe'] });
  let stdout = '', stderr = '';
  child.stdout.on('data', d => { stdout += d; }); child.stderr.on('data', d => { stderr += d; });
  child.stdin.on('error', () => {}); child.stdin.end(prompt);
  const code = await new Promise<number | null>((ok, fail) => { child.once('error', fail); child.once('close', ok); });
  await writeFile(join(run, 'raw', `${key}.events.jsonl`), stdout);
  await writeFile(join(run, 'raw', `${key}.stderr.txt`), stderr);
  const events = stdout.split('\n').filter(Boolean).flatMap(s => { try { return [JSON.parse(s)]; } catch { return []; } });
  const tools = events.filter(e => e.item && /command_execution|tool_call|web_search|file_change|mcp/.test(e.item.type));
  const record: any = { task_id: input.task_id, round: input.round, source_commit: input.source_commit,
    requested_model: model, requested_reasoning_effort: effort, provider_reported_model: null,
    cli_version: execFileSync('codex', ['--version'], { encoding: 'utf8' }).trim(),
    command: ['codex', ...args], input_hash: hash(JSON.stringify(input)),
    started_at: started, completed_at: new Date().toISOString(), duration_seconds: (performance.now() - before) / 1000,
    exit_code: code, usage: events.find(e => e.type === 'turn.completed')?.usage ?? null,
    thread_id: events.find(e => e.type === 'thread.started')?.thread_id ?? null, tool_events: tools,
    status: 'unresolved', result: null, rejected_response: null, error: null };
  let answer: any = null;
  try {
    if (code !== 0 || tools.length) throw new Error(`CLI failed or used tools: ${stderr.slice(-2000)}`);
    answer = await readJson(answerPath);
    if (answer.task_id !== input.task_id) throw new Error('Task identity mismatch');
    for (const e of answer.evidence) {
      const a = input.evidence.find((a: any) => a.artifact_id === e.artifact_id);
      if (!a || hash(a.content) !== a.sha256 || !Number.isInteger(e.line) || !e.quote || e.line < 1 || !a.content.split('\n').slice(e.line - 1, e.line - 1 + e.quote.split('\n').length).join('\n').includes(e.quote)) {
        throw new Error(`Invalid evidence: ${e.artifact_id}:${e.line}`);
      }
    }
    if (['confirmed_violation', 'refuted'].includes(answer.status) && !answer.evidence.length) throw new Error('Terminal verdict lacks evidence');
    if (answer.status === 'confirmed_violation' && (!answer.finding || answer.context_requests.length)) throw new Error('Violation is incomplete');
    if (answer.status === 'needs_context' && !answer.context_requests.length) throw new Error('Context request is empty');
    record.status = answer.status; record.result = answer;
  } catch (e) { record.error = String(e); record.rejected_response = answer; }
  await json(join(run, `${key}.json`), record, true);
  console.log(JSON.stringify({ task_id: input.task_id, round: input.round, status: record.status, seconds: record.duration_seconds, error: record.error }));
}
async function execute(run: string, only?: string) {
  const completed = new Set(await readdir(run));
  const files = (await readdir(join(run, 'inputs'))).filter(f => /^i05-\d+\.r\d+\.json$/.test(f) && !completed.has(f) && (!only || f.startsWith(`${only}.`))).sort();
  const cwd = await mkdtemp(join(tmpdir(), 'doramcap-code-confirm-'));
  let cursor = 0, active = 0, peak = 0;
  const started = new Date().toISOString();
  try {
    await Promise.all(Array.from({ length: Math.min(2, files.length) }, async () => {
      while (cursor < files.length) {
        const file = files[cursor++]; active++; peak = Math.max(peak, active);
        try { await invoke(run, file, cwd); } finally { active--; }
      }
    }));
  } finally {
    await json(join(run, `batch-${Date.now()}.json`), { started_at: started, completed_at: new Date().toISOString(), observed_peak_concurrency: peak, attempted_calls: files.length });
    await rm(cwd, { recursive: true, force: true });
  }
}
const [command, runName, only] = process.argv.slice(2);
if (!runName || !/^[a-zA-Z0-9_-]+$/.test(runName)) throw new Error('Usage: node confirm.ts prepare|run RUN_NAME [TASK_ID]');
const run = join(here, 'confirmations', runName);
if (command === 'prepare') await prepare(run);
else if (command === 'run') await execute(run, only);
else throw new Error('Unknown command');
