#!/usr/bin/env bun
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync, copyFileSync, readdirSync, statSync, constants } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join } from 'node:path';
import { Database } from 'bun:sqlite';
import { EventEmitter } from 'node:events';
import { observeState, settledState } from './lane-question-observer.mjs';

const [mode, preparedPath, releasePath, ...extra] = process.argv.slice(2);
assert(['validate', 'execute'].includes(mode) && preparedPath?.startsWith('/') && releasePath?.startsWith('/') && !extra.length,
  'Usage: lane-question-qa.mjs validate|execute <fresh prepared.json> <release.json>');
const helper = join(dirname(import.meta.path), 'lane-scratch-qa.mjs');
const validation = Bun.spawn([process.execPath, '--no-env-file', helper, 'validate', preparedPath, releasePath], { stdout: 'pipe', stderr: 'pipe' });
const validationExit = validation.exited;
const [validationOut, validationErr, validationCode] = await Promise.all([new Response(validation.stdout).text(), new Response(validation.stderr).text(), validationExit]);
assert.equal(validationCode, 0, `Preparation/release validation failed: ${validationErr}`);
const p = JSON.parse(readFileSync(preparedPath, 'utf8'));
assert.equal(JSON.parse(readFileSync(releasePath,'utf8')).backendScope?.kind, 'local-authenticated-preflight-v1', 'Task12 requires local authenticated preflight, not a model scope turn');
if (mode === 'validate') {
  console.log(JSON.stringify({ result: 'QUESTION_QA_VALIDATED_RUNTIME_UNRUN', validationCode, validationOut }));
  process.exit(0);
}
const token = process.env.OMON_OMO_APPSERVER_AUTH_TOKEN?.trim();
assert(token, 'Private OMON_OMO_APPSERVER_AUTH_TOKEN required');
for (const key of ['TYPESAFE_API_KEY', 'TYPESAFE_URL', 'TYPESAFE_MODEL']) assert(process.env[key], `Missing private ${key}`);
const env = { ...p.env, OMON_OMO_APPSERVER_AUTH_TOKEN: token };
for (const key of ['TYPESAFE_API_KEY', 'TYPESAFE_URL', 'TYPESAFE_MODEL']) env[key] = process.env[key];
assert.equal(env.OMON_DISCORD_ENABLED, 'false');
const sha = value => createHash('sha256').update(value).digest('hex');
writeFileSync(join(p.root, 'execution-started.json'), JSON.stringify({ scenario: '12-question', preparedSha256: sha(readFileSync(preparedPath)), releaseSha256: sha(readFileSync(releasePath)), at: new Date().toISOString() }), { flag: 'wx', mode: 0o600 });
const archive = join(dirname(import.meta.path), `task-12-question-${p.root.split('-').at(-1)}-${Date.now()}`);
mkdirSync(archive, { mode: 0o700 });
const records = [], children = new Map(), cancelObservers = new Set(), bus = new EventEmitter();
let gateway, log = '', db, verified = false, failure, gatewayDrain, observedProof;
const privateValues = [token, env.TYPESAFE_API_KEY].filter(Boolean);
const redact = text => privateValues.reduce((value, secret) => value.split(secret).join('[redacted]'), text);
const artifact = (name, value) => writeFileSync(join(p.root, name), typeof value === 'string' ? value : JSON.stringify(value, null, 2), { mode: 0o600 });
const record = value => { records.push({ ...value, at: new Date().toISOString() }); artifact('question-receipts.json', records); };
function spawn(argv) {
  const child = Bun.spawn(argv, { cwd: p.root, env, stdout: 'pipe', stderr: 'pipe' });
  const exited = child.exited.then(code => { record({ kind: 'process_exit', pid: child.pid, argv, code }); return code; });
  children.set(child, exited);
  return child;
}
async function bounded(promise, ms, label) {
  let timer;
  try { return await Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`${label}: deadline`)), ms); })]); }
  finally { clearTimeout(timer); }
}
async function command(argv) {
  const child = spawn(argv);
  const result = Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), children.get(child)]);
  let out, err, code;
  try { [out, err, code] = await bounded(result, 30000, 'CLI'); }
  catch (error) { child.kill('SIGTERM'); throw error; }
  record({ kind: 'cli', argv, code, stdout: redact(out), stderr: redact(err), databaseUrl: env.DATABASE_URL });
  assert.equal(code, 0);
  return out;
}
function observe(label, predicate, ms) {
  return observeState(p, bus, cancelObservers, label, predicate, ms);
}
async function drain(stream, name) {
  const decoder = new TextDecoder();
  for await (const bytes of stream) { log += `${name}: ${decoder.decode(bytes, { stream: true })}`; artifact('question-gateway.private.log', redact(log)); bus.emit('line'); }
}
const query = (sql, ...args) => { assert(verified && db); return db.query(sql).all(...args); };
const row = () => query("SELECT * FROM lanes WHERE key='qa-1'")[0];
const events = () => query("SELECT * FROM lane_events WHERE lane_key='qa-1' AND kind='question' ORDER BY id");
const transcript = () => query("SELECT * FROM lane_transcript WHERE lane_key='qa-1' ORDER BY id");
function settled(count) {
  return settledState(count, row(), transcript(), events());
}
const lane = args => { assert(verified); return command([p.binary, 'lane', ...args]); };
try {
  record({ kind: 'identity', preparedSha256: sha(readFileSync(preparedPath)), binarySha256: p.binarySha256, manifestSha256: p.manifestSha256, validationCode, authPresent: true, scratch: p.root });
  await command(['/usr/bin/git', '-C', p.root, 'init', 'qa-repo']);
  const ready = await command(['/usr/bin/curl', '--fail', '--max-time', '10', 'http://127.0.0.1:19742/readyz']);
  record({ kind: 'shared_ready', output: ready });
  const startup = observe('gateway readiness', () => log.includes('gateway database opened') && log.includes(p.dbPath) && log.includes('gateway lane service ready') ? { ready: true } : undefined, 30000);
  gateway = spawn([p.binary, 'run']);
  gatewayDrain = Promise.all([drain(gateway.stdout, 'stdout'), drain(gateway.stderr, 'stderr')]);
  let readyObserved = false;
  await Promise.race([startup.then(v => { readyObserved = true; return v; }), children.get(gateway).then(async code => { if (!readyObserved) { await gatewayDrain; throw new Error(`gateway startup exit ${code}`); } })]);
  const descriptors = await command(['/usr/sbin/lsof', '-a', '-p', String(gateway.pid), '-Fn']);
  const paths = descriptors.split('\n').filter(v => v.startsWith('n')).map(v => v.slice(1));
  assert(paths.includes(p.dbPath));
  assert(!paths.some(v => v.endsWith('.db') && v !== p.dbPath));
  db = new Database(p.dbPath, { readonly: true });
  const pragma = db.query('PRAGMA database_list').all();
  assert.equal(pragma.find(v => v.name === 'main').file, p.dbPath);
  verified = true;
  record({ kind: 'database_verified', pragma, paths, gatewayPid: gateway.pid });
  const brief = join(p.root, 'question-brief.md');
  artifact('question-brief.md', 'Isolated scratch QA. Do not use tools, delegation, goals, credentials, Discord, network writes or claim commands. Emit exactly [REPORT] qa-1 | question | pick A or B and end the turn. Do not continue until the lead sends an answer. On that answer emit exactly [REPORT] qa-1 | question | A received; waiting for next instruction and end the turn again. No file or claim requirement.\n');
  const started = Date.now();
  const paused = observe('question within two minutes and settled backend', () => {
    const state = settled(1);
    return typeof state === 'object' && state !== null && state.row?.omo_thread_id ? state : undefined;
  }, 120000);
  await lane(['start', 'qa-1', '--title', 'Question QA', '--brief', brief, '--done', 'file unused-question-signal', '--cwd', p.repo, '--lead', '4:lane|-|4:lead|-|9:lane:lead']);
  const first = await paused;
  const elapsedToPauseMs = Date.now() - started;
  const eventElapsedMs = Date.parse(first.events[0].created_at) - started;
  assert(Number.isFinite(eventElapsedMs) && eventElapsedMs >= 0 && eventElapsedMs <= 120000, 'question event outside trigger deadline');
  assert(first.transcript.some(v => v.role === 'agent' && v.text.includes('[REPORT] qa-1 | question | pick A or B')));
  assert(first.transcript.some(v => v.role === 'outcome' && v.text === 'completed'));
  artifact('question-paused.json', first);
  await lane(['log', 'qa-1']);
  assert.equal(row().turns_run, 1); assert.equal(row().status, 'blocked');
  assert.equal(transcript().filter(v => v.role === 'user').length, 1);
  const ownerText = 'A. Emit exactly [REPORT] qa-1 | question | A received; waiting for next instruction and end the turn. Do not use tools or claim.';
  artifact('question-owner-answer.txt', ownerText);
  const resumed = observe('same-thread answer prompt and resumed terminal turn', () => {
    const state = settled(2);
    return typeof state === 'object' && state !== null && state.row
      && state.transcript.some(v => v.role === 'user' && v.text === ownerText)
      && state.transcript.some(v => v.role === 'agent' && v.text.includes('[REPORT] qa-1 | question | A received; waiting for next instruction')) ? state : undefined;
  }, 120000);
  await lane(['send', 'qa-1', '--text', ownerText]);
  const second = await resumed;
  assert.equal(second.row.omo_thread_id, first.row.omo_thread_id);
  assert.equal(second.transcript.filter(v => v.role === 'outcome' && v.text === 'completed').length, 2);
  assert.equal(second.transcript.filter(v => v.role === 'user').length, 2);
  observedProof = { scope: 'actual dedicated backend question lane; no Discord', first, ownerText, second, elapsedToPauseMs, eventElapsedMs };
  await lane(['log', 'qa-1']);
  assert(settled(2), 'Resumed backend state changed before cleanup');
} catch (error) { failure = error; record({ kind: 'failure', message: redact(error.message) }); }
finally {
  for (const cancel of [...cancelObservers]) cancel();
  if (verified && gateway?.exitCode === null && row() && row().status !== 'closed') {
    try { await lane(['close', 'qa-1', '--force', '--note', 'forced: dedicated question QA cleanup']); }
    catch (error) { failure ??= error; }
  }
  for (const [child, exited] of children) {
    if (child.exitCode === null) child.kill('SIGTERM');
    const timer = setTimeout(() => child.kill('SIGKILL'), 10000);
    const code = await exited; clearTimeout(timer);
    let absent = false;
    try { process.kill(child.pid, 0); } catch (error) { if (error.code === 'ESRCH') absent = true; else throw error; }
    record({ kind: 'owned_process_cleanup', pid: child.pid, code, absent });
    if (!absent) failure ??= new Error('Owned PID survives cleanup');
  }
  await gatewayDrain;
  if (verified) {
    try {
      const finalState = { row: row(), events: events(), transcript: transcript(), pragma: db.query('PRAGMA database_list').all() };
      artifact('question-postcleanup-db.json', finalState);
      assert.equal(finalState.pragma.find(v => v.name === 'main').file, p.dbPath);
      if (observedProof) {
        assert.equal(finalState.row.status, 'closed');
        assert.equal(finalState.row.turns_run, 2);
        assert.equal(finalState.row.last_turn_outcome, 'completed');
        assert.equal(finalState.row.omo_thread_id, observedProof.first.row.omo_thread_id);
        assert.deepEqual(finalState.events.map(v => [v.id, v.kind, v.body, v.created_at]), observedProof.second.events.map(v => [v.id, v.kind, v.body, v.created_at]));
        const users = finalState.transcript.filter(v => v.role === 'user');
        assert.equal(users.length, 2); assert.equal(users[1].text, observedProof.ownerText);
        assert.equal(finalState.transcript.filter(v => v.role === 'outcome' && v.text === 'completed').length, 2);
        assert.deepEqual(finalState.transcript.filter(v => ['agent', 'user', 'outcome'].includes(v.role)), observedProof.second.transcript.filter(v => ['agent', 'user', 'outcome'].includes(v.role)));
        assert.equal(await children.get(gateway), 0, 'Owned gateway cleanup exit not zero');
        if (!failure) artifact('task-12-question.json', { ...observedProof, postCleanup: finalState, gatewayExit: await children.get(gateway) });
      }
    } catch (error) { failure ??= error; record({ kind: 'postcleanup_assertion_failure', message: redact(error.message) }); }
  }
  db?.close();
  record({ kind: 'cleanup', sharedDaemonUntouched: true, scratchRetained: p.root, status: failure ? 'failure' : 'observed' });
  for (const name of readdirSync(p.root)) if (statSync(join(p.root, name)).isFile() && !name.startsWith('qa.db')) copyFileSync(join(p.root, name), join(archive, name), constants.COPYFILE_EXCL);
  console.log(JSON.stringify({ result: failure ? 'FAILURE' : 'QUESTION_SEND_RESUME_OBSERVED', archive, root: p.root }));
  process.exitCode = failure ? 1 : 0;
}
