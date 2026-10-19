#!/usr/bin/env bun
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, realpathSync, readFileSync, writeFileSync, watch, existsSync, symlinkSync } from 'node:fs';
import { EventEmitter } from 'node:events';
import { createServer } from 'node:net';
import { Database } from 'bun:sqlite';
import { homedir } from 'node:os';
import { withOwnedAppServer } from './qa-owned-appserver.mjs';
export const judgeKeys = ['TYPESAFE_API_KEY', 'TYPESAFE_URL', 'TYPESAFE_MODEL'];
let judgeEnvOnce;
export const judgeCommand = ['zsh', '-c', 'source ~/.zshenv; printf "%s\\0" "$TYPESAFE_API_KEY" "$TYPESAFE_URL" "$TYPESAFE_MODEL"'];
export async function loadJudgeEnv(spawn = Bun.spawn) {
  const p = spawn(judgeCommand, { env: { ...process.env, HOME: homedir() }, stdout: 'pipe', stderr: 'ignore' });
  const timer = setTimeout(() => p.kill('SIGKILL'), 15000);
  try {
    const [out, code] = await Promise.all([new Response(p.stdout).text(), p.exited]);
    assert.equal(code, 0, 'Judge configuration load failed');
    const values = out.split('\0');
    assert(values.length === 4 && values.pop() === '' && values.every(Boolean), 'Three nonempty judge values required');
    return Object.fromEntries(judgeKeys.map((key, i) => [key, values[i]]));
  } finally { clearTimeout(timer); }
}
export function clean(text, env) {
  return [...judgeKeys.map(key => env[key]), env.OMON_OMO_APPSERVER_AUTH_TOKEN, env.OMON_OMO_CRON_APPSERVER_AUTH_TOKEN].filter(Boolean).sort((a,b) => b.length - a.length).reduce((s, secret) => s.split(secret).join('[redacted]'), text);
}
export const scenarioDeadlines = { 15: 1200000, 16: 780000, 18: 900000, 20: 900000, 24: 1200000, 25: 600000, 28: 600000, 29: 600000, 31: 1500000, 32: 600000, 26: 900000, 35: 1200000 };
export function mailWrapper(home) {
  const quote = value => "'" + value.replaceAll("'", "'\\''") + "'";
  const policy = '(version 1)(allow default)(deny file-write* (require-all (subpath ' + JSON.stringify(home) + ') (require-not (subpath ' + JSON.stringify(home + '/.zele') + '))))';
  return '#!/bin/sh\ncase "$1:$2" in mail:list|mail:watch) ;; *) exit 64 ;; esac\nexec /usr/bin/sandbox-exec -p ' + quote(policy) + ' /usr/bin/env HOME=' + quote(home) + ' /Users/indo/.bun/install/global/node_modules/.bin/zele "$@"\n';
}
export function armDeadline(receipt, ms, save, controller, schedule = setTimeout) {
  assert(Number.isInteger(ms) && ms > 0);
  receipt.deadlineMs = ms;
  receipt.deadlineAt = new Date(Date.now() + ms).toISOString();
  return schedule(() => {
    receipt.status = 'deadline'; receipt.exitCode = 1;
    receipt.error = 'Scenario deadline exceeded';
    save(receipt); // Persist before cancellation/cleanup; never disappear without a receipt.
    controller.abort(new Error(receipt.error));
  }, ms);
}
export const sha = data => createHash('sha256').update(data).digest('hex');
export const owner = '414011632306618368';
export const bot = '1465631383862120451';
export const lead = '7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451';
export function validateRelease(r, read = readFileSync) {
  assert(r.release_allowed !== false && r.binary?.released !== false, 'Held build refused');
  assert(r.status === 'released' || r.release_allowed === true, 'Unreleased build refused');
  const binary = typeof r.binary === 'string' ? r.binary : r.binary?.path ?? r.binary_path;
  assert(binary?.startsWith('/'), 'Absolute binary path required');
  const expected = r.binarySha256 ?? r.binary_sha256 ?? r.binary?.sha256;
  assert(/^[a-f0-9]{64}$/.test(expected), 'Release binary SHA-256 required');
  assert.equal(sha(read(binary)), expected, 'Release binary SHA-256 mismatch');
  if (r.manifest || r.manifest_path) {
    const manifest = typeof r.manifest === 'string' ? r.manifest : r.manifest?.path ?? r.manifest_path;
    const manifestSha = r.manifestSha256 ?? r.manifest?.sha256 ?? r.manifest_sha256;
    assert(manifest?.startsWith('/') && /^[a-f0-9]{64}$/.test(manifestSha));
    assert.equal(sha(read(manifest)), manifestSha, 'Manifest SHA-256 mismatch');
  }
  return expected;
}
export function observe(bus, check, label, ms = 180000, signal) {
  return new Promise((resolve, reject) => {
    let timer;
    const finish = () => { clearTimeout(timer); bus.off('change', inspect); signal?.removeEventListener('abort', abort); };
    const abort = () => { finish(); reject(signal.reason); };
    const inspect = () => {
      try { const value = check(); if (value) { finish(); resolve(value); return true; } }
      catch (error) { finish(); reject(error); return true; }
      return false;
    };
    bus.on('change', inspect);
    signal?.addEventListener('abort', abort, { once: true });
    if (signal?.aborted) { abort(); return; }
    timer = setTimeout(() => { if (!inspect()) { finish(); reject(new Error(label + ' deadline; final authoritative query failed')); } }, ms);
    inspect();
  });
}
export async function reservePort() {
  const s = createServer();
  await new Promise((resolve, reject) => { s.once('error', reject); s.listen(0, '127.0.0.1', resolve); });
  const p = s.address().port;
  await new Promise((resolve, reject) => s.close(e => e ? reject(e) : resolve()));
  return p;
}
export function recorder(bus) {
  let id = 800000000000000000n;
  const requests = [], messages = new Map(), channels = new Map(), deleted = new Set();
  const channel = id => channels.get(id) ?? { id, type: 11, guild_id: '700000000000000001', parent_id: '700000000000000002', name: 'QA lane', position: 0, permission_overwrites: [], thread_metadata: { archived: false, auto_archive_duration: 60, archive_timestamp: new Date().toISOString(), locked: false } };
  const user = { id: bot, username: 'local-qa', discriminator: '0000', avatar: null, bot: true };
  const server = Bun.serve({ hostname: '127.0.0.1', port: 0, async fetch(req) {
    const u = new URL(req.url); let body = null;
    if (req.method !== 'GET' && req.method !== 'DELETE') body = await req.json();
    const record = { method: req.method, path: u.pathname, body, at: new Date().toISOString() }; requests.push(record);
    const parts = u.pathname.startsWith('/api/v10/') ? u.pathname.slice(9).split('/') : [];
    let value, status = 200;
    if (req.method === 'GET' && parts.length === 2 && parts[0] === 'users' && parts[1] === '@me') value = user;
    else if (parts[0] !== 'channels') { value = { message: 'Unexpected fixture route', code: 0 }; status = 404; }
    else if (parts[2] === 'messages' && parts[4] === 'threads' && req.method === 'POST') {
      const key = String(++id); value = { ...channel(key), name: body.name }; channels.set(key, value);
    } else if (parts[2] === 'messages') {
      const mid = parts[3];
      if (req.method === 'POST') {
        assert.deepEqual(body.allowed_mentions?.parse ?? [], [], 'Mentions must be disabled');
        assert(!body.allowed_mentions?.users?.length && !body.allowed_mentions?.roles?.length);
        const key = String(++id); value = { id: key, channel_id: parts[1], author: user, content: body.content, timestamp: new Date().toISOString(), edited_timestamp: null, tts: false, mention_everyone: false, mentions: [], mention_roles: [], attachments: [], embeds: [], pinned: false, type: 0, nonce: body.nonce }; messages.set(key, value);
      } else if (deleted.has(mid) || !messages.has(mid)) { status = 404; value = { message: 'Unknown Message', code: 10008 }; }
      else if (req.method === 'DELETE') { deleted.add(mid); status = 204; }
      else { value = { ...messages.get(mid), ...(body ?? {}) }; messages.set(mid, value); }
    } else {
      value = channel(parts[1]);
      if (req.method === 'PATCH') value = { ...value, ...body, thread_metadata: { ...value.thread_metadata, ...(body.archived === undefined ? {} : { archived: body.archived }) } };
      channels.set(parts[1], value);
    }
    record.status = status; record.responseId = value?.id; bus.emit('change');
    return status === 204 ? new Response(null, { status }) : Response.json(value, { status });
  } });
  return { origin: server.url.origin, requests, messages, channels, deleted, stop: () => server.stop(true) };
}
export async function runQA(todo, releasePath, options, scenario) {
  assert(releasePath?.startsWith('/'), 'Usage: execute /absolute/wave4b-strict-gate-release.json');
  const release = JSON.parse(readFileSync(releasePath, 'utf8'));
  const expected = validateRelease(release); // Before scratch, listener, app-server, or child creation.
  const binary = typeof release.binary === 'string' ? release.binary : release.binary?.path ?? release.binary_path;
  const root = realpathSync(mkdtempSync('/private/tmp/omon-task' + todo + '-live-final-'));
  for (const d of ['home', 'workspace', 'bin']) mkdirSync(root + '/' + d, { mode: 0o700 });
  symlinkSync(binary, root + '/bin/omo-gateway');
  const judge = process.env.QA2_JUDGE ?? '/Users/indo/.cargo/bin/typesafe-judge';
  assert(existsSync(judge), 'Real typesafe-judge executable required');
  symlinkSync(judge, root + '/bin/typesafe-judge');
  const dbPath = root + '/qa.db', bus = new EventEmitter();
  const receipt = { todo, scenario: options.name, root, dbPath, releasePath, binarySha256: expected, commands: [], cleanup: [], status: 'running' };
  const save = (name, data) => writeFileSync(root + '/' + name, clean(JSON.stringify(data, null, 2), env), { mode: 0o600 });
  let db, gateway, pumps, logs = '', wal, sink, appReceipt;
  const folder = watch(root, () => bus.emit('change'));
  const env = { HOME: root + '/home', USER: process.env.USER, PATH: root + '/bin:/opt/homebrew/bin:/Users/indo/.bun/bin:/Users/indo/.bun/install/global/node_modules/.bin:/usr/bin:/bin:/usr/sbin:/sbin', TMPDIR: root, DATABASE_URL: 'sqlite://' + dbPath, OMON_RUNTIME_LOCK_DIR: root, OMON_WORKSPACE_ROOT: root + '/workspace', OMON_TOOL_ROOTS: root, OMON_DISCORD_ENABLED: 'false', OMON_LANES_ENABLED: 'true', OMON_INBOX_ENABLED: 'false', OMON_KIBITZER_ENABLED: 'false', OMON_OMO_AUTOSPAWN: 'off', DASHBOARD_ENABLED: 'false', OMON_LANE_KEYWORDS: '', OMON_LANE_DEFAULT_LEAD: lead, NO_COLOR: '1', RUST_LOG: 'info', ...options.env };
  for (const [k, v] of Object.entries(process.env)) if (/^(DEFAULT_MODEL|OMON_DEFAULT_MODEL|OMON_MODEL_.*|OMON_OMO_FALLBACK_MODEL|FALLBACK_MODEL|OMON_PIRATETALK_URL|OMON_INBOX_KAKAO_ROOMS)$/.test(k)) env[k] = v;
  const controller = new AbortController();
  let deadline;
  const socket = '/tmp/omon-lanes-' + sha(Buffer.from(dbPath)).slice(0, 24) + '.sock';
  async function cmd(argv, expectedCode = 0, config = {}) {
    validateRelease(release);
    controller.signal.throwIfAborted();
    const p = Bun.spawn(argv, { cwd: root, env, stdout: 'pipe', stderr: 'pipe' });
    const abort = () => p.kill('SIGKILL');
    controller.signal.addEventListener('abort', abort, { once: true });
    const t = setTimeout(() => p.kill('SIGKILL'), 60000);
    const [out, err, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited]); clearTimeout(t);
    controller.signal.removeEventListener('abort', abort);
    receipt.commands.push({ argv, pid: p.pid, code, ...(config.source ? { outputBytes: out.length, errorBytes: err.length } : { out: clean(out, env), err: clean(err, env) }) });
    controller.signal.throwIfAborted();
    assert.equal(code, expectedCode, 'Unexpected command exit: ' + argv.join(' '));
    return out;
  }
  async function pump(s) {
    const decoder = new TextDecoder(); let pending = '';
    for await (const b of s) {
      pending += decoder.decode(b, { stream: true });
      const end = pending.lastIndexOf('\n');
      if (end < 0) continue;
      logs += clean(pending.slice(0, end + 1), env); pending = pending.slice(end + 1);
      writeFileSync(root + '/task-' + todo + '-live-final.log', logs, { mode: 0o600 }); bus.emit('change');
    }
    logs += clean(pending + decoder.decode(), env);
    writeFileSync(root + '/task-' + todo + '-live-final.log', logs, { mode: 0o600 }); bus.emit('change');
  }
  async function stop() {
    if (!gateway) return;
    if (gateway.exitCode === null) gateway.kill('SIGTERM');
    const t = setTimeout(() => gateway.kill('SIGKILL'), 45000);
    const code = await gateway.exited; clearTimeout(t); await pumps;
    let absent = false; try { process.kill(gateway.pid, 0); } catch (e) { if (e.code !== 'ESRCH') throw e; absent = true; }
    const entry = { pid: gateway.pid, code, pidAbsent: absent, socketRemoved: !existsSync(socket) }; receipt.cleanup.push(entry); gateway = null;
    wal?.close(); wal = null;
    assert.equal(code, 0); assert(absent); assert(entry.socketRemoved);
  }
  async function start() {
    assert(!gateway);
    validateRelease(release);
    const offset = logs.length;
    controller.signal.throwIfAborted();
    const ready = observe(bus, () => logs.slice(offset).includes('headless gateway lead runtime ready'), 'gateway startup', 45000, controller.signal); ready.catch(() => {});
    gateway = Bun.spawn([binary, 'run'], { cwd: root, env, stdout: 'pipe', stderr: 'pipe' });
    pumps = Promise.all([pump(gateway.stdout), pump(gateway.stderr)]);
    gateway.exited.then(() => bus.emit('change'));
    await ready;
    db ??= new Database(dbPath); db.exec('PRAGMA busy_timeout=5000');
    wal?.close(); wal = watch(dbPath + '-wal', () => bus.emit('change'));
    const pragma = db.query('PRAGMA database_list').all(); assert.equal(pragma[0].file, dbPath);
    assert.equal(db.query('PRAGMA journal_mode').get().journal_mode, 'wal');
    const descriptors = await cmd(['/usr/sbin/lsof', '-a', '-p', String(gateway.pid), '-Fn']);
    assert(descriptors.split('\n').includes('n' + dbPath)); assert(existsSync(socket));
    receipt.initial = { pragma, lanes: db.query('SELECT count(*) n FROM lanes').get(), messages: db.query('SELECT count(*) n FROM messages').get() };
    save('descriptor.json', { root, dbPath, socket, pid: gateway.pid, pragma, descriptors, envKeys: Object.keys(env) });
    bus.emit('change');
  }
  const c = { root, dbPath, env, binary, bus, receipt, cmd, save, start, stop, signal: controller.signal, logs: () => logs, observe: (check, label, ms) => observe(bus, check, label, ms, controller.signal), snapshot: (sql, ...args) => db.query(sql).all(...args), get db() { return db; }, get sink() { return sink; } };
  try {
    deadline = armDeadline(receipt, scenarioDeadlines[todo], r => save('task' + todo + '-live-final.json', r), controller);
    Object.assign(env, await (judgeEnvOnce ??= loadJudgeEnv()));
    receipt.judgeEnv = { keys: judgeKeys, loadedFrom: 'owner ~/.zshenv', valuesRecorded: false };
    if (todo === 24) {
      writeFileSync(root + '/bin/zele', mailWrapper(homedir()), { mode: 0o700 });
      receipt.mailSource = { home: homedir(), commands: ['mail list', 'mail watch'], ownerHomeWrites: 'denied except ~/.zele sandbox allowance', otherHome: env.HOME, contentLogged: false };
    }
    if (options.sink) { sink = recorder(bus); env.OMON_DISCORD_API_BASE = sink.origin; }
    const execute = async handle => {
      if (handle) { Object.assign(env, handle.gatewayEnv); env.OMON_OMO_CRON_APPSERVER_URL = handle.url; env.OMON_OMO_CRON_APPSERVER_AUTH_TOKEN = handle.gatewayEnv.OMON_OMO_APPSERVER_AUTH_TOKEN; }
      try { await start(); await scenario(c); }
      finally { await stop(); }
    };
    if (options.model) appReceipt = await withOwnedAppServer(async handle => { appReceipt = handle.receipt; await execute(handle); }, env); else {
      // Backend construction requires an endpoint, but no scenario may turn without an owned app-server.
      env.OMON_OMO_APPSERVER_URL = 'ws://127.0.0.1:' + await reservePort(); await execute();
    }
    receipt.status = receipt.blocked?.length ? 'blocked' : 'passed';
  } catch (e) { receipt.status = controller.signal.aborted ? 'deadline' : 'failed'; receipt.error = clean(String(e.stack ?? e), env); }
  finally {
    clearTimeout(deadline);
    try { await stop(); } catch (e) { receipt.status = 'failed'; receipt.cleanupError = clean(String(e), env); }
    receipt.appserver = appReceipt; if (sink) { save('http-recorder.json', sink.requests); await sink.stop(); }
    if (db) { const tables = ['lanes','lane_history','lane_transcript','lane_events','kibitzer_state','judge_calls','cron_jobs','cron_runs','cron_problems','cron_remediation_outcomes','inbox_decisions','inbox_proposals','delivery_obligations']; const state = {}; for (const table of tables) { try { state[table] = db.query('SELECT * FROM '+table).all(); } catch {} } save('final-machine-state.json', state); }
    wal?.close(); folder.close(); db?.close();
    try { receipt.finalBinarySha256 = validateRelease(release); } catch (e) { receipt.status = 'failed'; receipt.identityError = clean(String(e), env); }
    receipt.exitCode = receipt.status === 'passed' ? 0 : receipt.status === 'blocked' ? 3 : 1;
    receipt.scratchRetained = existsSync(root); save('task' + todo + '-live-final.json', receipt);
    console.log(JSON.stringify({ todo, scenario: options.name, root, status: receipt.status, exitCode: receipt.exitCode }));
  }
  return receipt;
}
export function block(c, scenario, reason) { (c.receipt.blocked ??= []).push({ scenario, reason }); }
export function seedItem(c, id, cls = 'bot', snippet = 'Local QA repo task awaiting owner approval') {
  const now = new Date().toISOString();
  c.db.query('INSERT INTO inbox_items(id,source,source_ref,title,snippet,received_at,raw_hash,collected_at,link) VALUES(?,?,?,?,?,?,?,?,?)').run(id, 'discord', id, 'Owned QA', snippet, now, sha(snippet), now, 'https://example.invalid/' + id);
  if (cls) c.db.query('INSERT INTO inbox_decisions(item_id,class,reason,proposed_action,status,decided_at,model) VALUES(?,?,?,?,?,?,?)').run(id, cls, 'Owned QA fixture', 'Perform isolated coding task', 'open', now, 'fixture');
}
