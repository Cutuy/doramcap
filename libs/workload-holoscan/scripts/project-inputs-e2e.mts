import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const exec = promisify(execFile);
const [cli, applications] = process.argv.slice(2);
assert.ok(cli && applications && isAbsolute(cli) && isAbsolute(applications), 'usage: node project-inputs-e2e.mts /absolute/holoscan /absolute/holoscan-applications');
const checkout = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const python = join(dirname(cli), 'python');
const fixture = await mkdtemp(join(tmpdir(), 'holoscan-module-inputs-'));
const module = join(fixture, 'module'), application = join(fixture, 'application'), source = join(fixture, 'source');
const metadata = { $schema:'urn:holoscan:module:v2', module: { name:'holoscan-sample', version:'1.0.0', support_level:'community', description:'fixture', authors:[{name:'Fixture',affiliation:'Fixture'}], license:'Apache-2.0', language:['Python'], namespace:{python:'holoscan_sample'}, holoscan_sdk:{minimum_required_version:'5.0.0',tested_versions:['5.0.0']}, platforms:['x86_64'], tags:['fixture'], operator_names:['Sample'], dependencies:[], dockerfile:'Dockerfile' } };
const configuration = `[project]\nname="unrelated"\n[tool.other]\nname="ignored"\n[tool.holoscan]\ndocker-build-args=["--build-arg","GREETING=hello world"]\ndocker-run-args=["  --label  ","GREETING=hello world","--log-driver","journald"]\n`;
const observe = async (root: string) => {
  const { stdout } = await exec('cargo', ['run','--release','--locked','-q','-p','workload-holoscan','--example','project_inputs','--',root], {cwd:checkout});
  return JSON.parse(stdout);
};
// The upstream Python code is an independent test oracle; production adds no Python wrapper.
const native = async (root: string) => {
  const { stdout } = await exec(python, ['-c', `import json,shlex,sys\nfrom pathlib import Path\nfrom holoscan_cli.project_context import _selected_context\nc=_selected_context(Path(sys.argv[1]),discovery='explicit',environ={})\nprint(json.dumps({'kind':c.kind,'build':shlex.split(c.docker_build_args or ''),'run':shlex.split(c.docker_run_args or '')}))`, root]);
  return JSON.parse(stdout);
};
const querySucceeds = async (root: string) => {
  try { await exec(cli, ['--project-root',root,'list','--json']); return true; }
  catch { return false; }
};
try {
  // Arrange: valid Module, standalone Application and source-repository roots.
  const appMetadata = await readFile(join(applications, 'applications/bci_visualization/metadata.json'));
  for (const root of [module, application, source]) await mkdir(root);
  await writeFile(join(module,'metadata.json'),JSON.stringify(metadata));
  await mkdir(join(module,'applications/sample'),{recursive:true});
  await writeFile(join(module,'applications/sample/metadata.json'),appMetadata);
  await writeFile(join(application,'metadata.json'),appMetadata);
  await writeFile(join(module,'pyproject.toml'),configuration);
  for (const root of [application,source]) await writeFile(join(root,'pyproject.toml'),'broken=[ SECRET_SOURCE_CONTENT');
  // Act / Assert: only the Module reads native token arrays; whitespace remains raw.
  const observed = await observe(module), effective = await native(module);
  assert.equal(observed.declared_kind,3);
  assert.deepEqual(observed.docker_run_args.values,['  --label  ','GREETING=hello world','--log-driver','journald']);
  assert.deepEqual(effective.run,['--label','GREETING=hello world','--log-driver','journald']);
  assert.deepEqual(effective.build,observed.docker_build_args.values);
  assert.equal(observed.configuration.sha256,createHash('sha256').update(configuration).digest('hex'));
  assert.equal(await querySucceeds(module),true);
  for (const [root,kind] of [[application,'application'],[source,'source']] as const) {
    const actual = await native(root), raw = await observe(root);
    assert.equal(actual.kind,kind);
    assert.equal(raw.configuration,null);
    assert.equal(await querySucceeds(root),true);
  }
  // Act / Assert: absent defaults and a non-file pyproject match native optional-file behavior.
  await rm(join(module,'pyproject.toml'));
  assert.deepEqual((await native(module)).run,[]);
  assert.equal((await observe(module)).configuration,null);
  await mkdir(join(module,'pyproject.toml'));
  assert.deepEqual((await native(module)).run,[]);
  assert.equal((await observe(module)).configuration,null);
  await rm(join(module,'pyproject.toml'),{recursive:true});
  // Act / Assert: native invalid arrays fail, and reader errors reveal no source values.
  for (const text of ['secret="SECRET_SOURCE_CONTENT"\nbroken=[','[tool.holoscan]\ndocker-run-args="SECRET_SOURCE_CONTENT"','[tool.holoscan]\ndocker-run-args=[1]','[tool.holoscan]\ndocker-run-args=["  "]']) {
    await writeFile(join(module,'pyproject.toml'),text);
    assert.equal(await querySucceeds(module),false);
    await assert.rejects(observe(module), (error: any) => !String(error.stderr ?? error).includes('SECRET_SOURCE_CONTENT'));
  }
  // Act / Assert: observing raw inputs does not replace other native profile validation.
  await writeFile(join(module,'pyproject.toml'),'[tool.holoscan]\ncuda=[]\ndocker-run-args=["--log-driver=journald"]\n');
  assert.deepEqual((await observe(module)).docker_run_args.values,['--log-driver=journald']);
  assert.equal(await querySucceeds(module),false);
  console.log('CLI5 Module/Application/source inputs, raw tokens, optional defaults, invalid arrays and native validation ownership: passed');
} finally { await rm(fixture,{recursive:true,force:true}); }
