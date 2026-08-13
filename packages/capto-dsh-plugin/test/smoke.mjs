// Offline smoke tests for capto-dsh-plugin. Run: node test/smoke.mjs
//
// Does NOT require the Capto desktop or its control plane: success paths run
// against test/fixtures/fake-capto.mjs (a fake `capto` CLI speaking the JSON
// envelope contract). When target/debug/capto.exe exists, one real-CLI error
// path check runs too.
import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const pkgDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const importPkg = (rel) => import(pathToFileURL(path.join(pkgDir, rel)).href);

const plugin = await importPkg('src/index.js');
const { runCapto, CaptoError } = await importPkg('src/capto.js');

const fakeCli = path.join(pkgDir, 'test', 'fixtures', 'fake-capto.mjs');
const fake = (extra = {}) => ({
  command: [process.execPath, fakeCli],
  timeoutMs: 2000,
  autoOpen: false,
  ...extra,
});

let passed = 0;
const ok = (name) => {
  passed += 1;
  console.log(`  ok - ${name}`);
};

// --- plugin contract -------------------------------------------------------
console.log('plugin contract');
assert.equal(plugin.name, 'capto');
assert.deepEqual(plugin.inject, ['tools', 'systemPrompt']);
assert.equal(typeof plugin.Config, 'function');
assert.equal(typeof plugin.apply, 'function');
ok('exports { name, inject, Config, apply }');

// --- Config validation (schemastery) --------------------------------------
console.log('config validation');
assert.doesNotThrow(() => plugin.Config({}));
const coerced = plugin.Config({ command: ['D:\\x\\capto.exe'], timeoutMs: 5000 });
assert.equal(coerced.command[0], 'D:\\x\\capto.exe');
assert.equal(coerced.timeoutMs, 5000);
assert.throws(() => plugin.Config({ timeoutMs: -1 }));
assert.throws(() => plugin.Config({ timeoutMs: 0 }));
assert.throws(() => plugin.Config({ command: [] }));
assert.throws(() => plugin.Config({ noLaunch: 'yes' }));
ok('Config coerces defaults and rejects bad values');

// --- apply registers every tool + prompt section --------------------------
console.log('apply');
const names = [
  'capto_status',
  'capto_doctor',
  'capto_open',
  'capto_list',
  'capto_shot',
  'capto_record_start',
  'capto_record_stop',
  'capto_record_pause',
  'capto_record_resume',
  'capto_config_get',
  'capto_config_set',
  'capto_config_path',
  'capto_outputs_recent',
  'capto_outputs_open',
];
const registered = [];
const sections = [];
plugin.apply(
  {
    systemPrompt: { section: (s) => sections.push(s) },
    tools: { register: (def) => registered.push(def) },
  },
  { command: [process.execPath, fakeCli] },
);
assert.deepEqual(
  registered.map((d) => d.name),
  names,
);
assert.ok(sections.some((s) => s.name === 'tool:capto' && s.order === 110));
ok('registers 14 capto_* tools + tool:capto prompt section');

const byName = Object.fromEntries(registered.map((d) => [d.name, d]));
const signal = new AbortController().signal;

// --- render ----------------------------------------------------------------
console.log('render');
const rendered = byName.capto_status.output.render({}, { state: 'idle' });
assert.equal(rendered[0].type, 'text');
assert.ok(rendered[0].text.includes('"state": "idle"'));
ok('output.render emits pretty JSON text');

// --- success paths through the tools (fake CLI) ---------------------------
console.log('success paths');
let r = await byName.capto_status.execute({}, { signal });
assert.equal(r.state, 'idle');
r = await byName.capto_doctor.execute({}, { signal });
assert.equal(r.ffmpegOk, true);
r = await byName.capto_open.execute({}, { signal });
assert.equal(r.path, 'C:\\fake\\capto-app.exe');
r = await byName.capto_list.execute({ what: 'displays' }, { signal });
assert.deepEqual(r.args, ['list', 'displays']);
r = await byName.capto_record_start.execute({ source: 'window', window: 42, fps: 30 }, { signal });
assert.equal(r.state, 'recording');
r = await byName.capto_record_stop.execute({}, { signal });
assert.equal(r.state, 'idle');
ok('status/doctor/open/list/record run against fake CLI');

// --- arg mapping (fake echoes argv under FAKE_CAPTO_ECHO) ------------------
console.log('arg mapping');
process.env.FAKE_CAPTO_ECHO = '1';
try {
  r = await byName.capto_shot.execute(
    { source: 'region', x: 0, y: 0, width: 1280, height: 720 },
    { signal },
  );
  assert.deepEqual(r.args, [
    'shot',
    '--source',
    'region',
    '--x',
    '0',
    '--y',
    '0',
    '--width',
    '1280',
    '--height',
    '720',
  ]);
  r = await byName.capto_record_start.execute(
    { source: 'display', format: 'gif', fps: 15, noCursor: true },
    { signal },
  );
  assert.deepEqual(r.args, ['record', 'start', '--source', 'display', '--format', 'gif', '--fps', '15', '--no-cursor']);
  r = await byName.capto_record_start.execute({}, { signal });
  assert.deepEqual(r.args, ['record', 'start', '--source', 'display']); // defaults
  r = await byName.capto_config_get.execute({ key: 'fps' }, { signal });
  assert.deepEqual(r.args, ['config', 'get', 'fps']);
  r = await byName.capto_config_set.execute({ json: '{"fps":60}', pairs: ['includeCursor=true'] }, { signal });
  assert.deepEqual(r.args, ['config', 'set', '--json', '{"fps":60}', 'includeCursor=true']);
  r = await byName.capto_config_path.execute({}, { signal });
  assert.deepEqual(r.args, ['config', 'path']);
  r = await byName.capto_outputs_recent.execute({ limit: 5 }, { signal });
  assert.deepEqual(r.args, ['outputs', 'recent', '--limit', '5']);
  r = await byName.capto_outputs_open.execute({ last: true, folder: true }, { signal });
  assert.deepEqual(r.args, ['outputs', 'open', '--last', '--folder']);
} finally {
  delete process.env.FAKE_CAPTO_ECHO;
}
ok('CLI arg mapping matches the docs/CLI.md contract');

// --- config_set validation -------------------------------------------------
await assert.rejects(
  byName.capto_config_set.execute({}, { signal }),
  /capto_config_set: provide `json` or at least one `pairs` entry/,
);
ok('capto_config_set requires json or pairs');

// --- failure paths ---------------------------------------------------------
console.log('failure paths');
process.env.FAKE_CAPTO_BOOM = '1';
try {
  await assert.rejects(
    runCapto(fake(), ['status']),
    (e) =>
      e instanceof CaptoError &&
      e.exitCode === 2 &&
      e.code === 'desktopUnavailable' &&
      /capto_open/.test(e.message),
  );
  await assert.rejects(byName.capto_status.execute({}, { signal }), /desktopUnavailable/);
} finally {
  delete process.env.FAKE_CAPTO_BOOM;
}
ok('exit 2 → CaptoError(desktopUnavailable) with capto_open guidance');

// --- timeout ---------------------------------------------------------------
console.log('timeout');
process.env.FAKE_CAPTO_SLEEP_MS = '5000';
try {
  await assert.rejects(
    runCapto(fake({ timeoutMs: 300 }), ['status']),
    (e) => e instanceof CaptoError && /timed out after 300ms/.test(e.message),
  );
} finally {
  delete process.env.FAKE_CAPTO_SLEEP_MS;
}
ok('timeout kills the CLI child and reports it');

// --- autoOpen recovery (marker file) --------------------------------------
console.log('autoOpen recovery');
const dir = mkdtempSync(path.join(os.tmpdir(), 'capto-dsh-'));
const marker = path.join(dir, 'down.marker');
writeFileSync(marker, 'x');
process.env.FAKE_CAPTO_MARKER = marker;
try {
  await assert.rejects(runCapto(fake({ autoOpen: false }), ['status']), /capto_open/);
  const recovered = await runCapto(fake({ autoOpen: true }), ['status']);
  assert.equal(recovered.data.state, 'idle');
  assert.ok(!existsSync(marker), 'open removed the marker');
} finally {
  delete process.env.FAKE_CAPTO_MARKER;
  rmSync(dir, { recursive: true, force: true });
}
ok('autoOpen runs capto open once and retries');

// --- real CLI (only when built; desktop state decides the branch) ----------
console.log('real CLI');
const realCli = path.resolve(pkgDir, '..', '..', 'target', 'debug', 'capto.exe');
if (existsSync(realCli)) {
  try {
    // control plane up: the envelope must parse into a status snapshot
    const res = await runCapto({ command: [realCli, '--no-launch'], timeoutMs: 10000 }, ['status']);
    assert.equal(typeof res.data, 'object');
    assert.ok('state' in res.data);
    ok('real capto.exe: control plane up, envelope parsed');
  } catch (e) {
    // control plane down: normalized desktopUnavailable with exit code 2
    assert.ok(
      e instanceof CaptoError && e.exitCode === 2 && e.code === 'desktopUnavailable',
      `unexpected real-CLI error: ${e.message}`,
    );
    ok('real capto.exe reports desktopUnavailable (exit 2)');
  }
} else {
  console.log('  skip - capto.exe not built');
}

console.log(`\n${passed} checks passed`);
