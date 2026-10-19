#!/usr/bin/env bun
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, realpathSync, statSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join } from 'node:path';

const [preparedPath, ...extra] = process.argv.slice(2);
assert(preparedPath?.startsWith('/') && !extra.length, 'Usage: lane-local-preflight.mjs <fresh prepared.json>');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const bytes = readFileSync(preparedPath), p = JSON.parse(bytes);
assert.equal(p.schema, 'omon-lane-qa-prepared-v1');
assert(/^\/private\/tmp\/omon-lane-scratch-qa-[^/]+$/.test(p.root));
assert.equal(realpathSync(p.root), p.root);
assert.equal(realpathSync(preparedPath), join(p.root, 'prepared.json'));
assert.equal(statSync(p.root).mode & 0o777, 0o700);
assert.equal(readFileSync(join(p.root, '.env'), 'utf8'), '');
assert.equal(p.repo, join(p.root, 'qa-repo')); assert.equal(realpathSync(p.repo), p.repo);
assert.equal(p.dbPath, join(p.root, 'qa.db')); assert.equal(p.databaseUrl, `sqlite://${p.dbPath}`);
assert.equal(p.env.DATABASE_URL, p.databaseUrl); assert.equal(p.env.OMON_DISCORD_ENABLED, 'false');
assert.equal(p.env.OMON_OMO_APPSERVER_URL, 'ws://127.0.0.1:19742');
assert.equal(sha(readFileSync(p.binary)), p.binarySha256);
assert.equal(sha(readFileSync(p.manifestPath)), p.manifestSha256);
const seen = new Set();
for (const line of readFileSync(p.manifestPath, 'utf8').trim().split('\n')) {
  const match = /^([a-f0-9]{64})  (.+)$/.exec(line); assert(match);
  const relative = match[2]; assert(!relative.startsWith('/') && !relative.split('/').includes('..'));
  assert(!seen.has(relative)); seen.add(relative);
  assert.equal(sha(readFileSync(join(p.sourceRoot, relative))), match[1]);
}
for (const name of ['Cargo.toml', 'Cargo.lock', 'src/lanes/runner.rs', 'src/agent/omo_config.rs']) assert(seen.has(name));
for (const suffix of ['', '-wal', '-shm', '-journal']) assert(!existsSync(p.dbPath + suffix));
assert(!existsSync(join(p.root, 'execution-started.json')));
const path = join(p.root, 'local-preflight.json'); assert(!existsSync(path));
const token = process.env.OMON_OMO_APPSERVER_AUTH_TOKEN?.trim();
assert(token && !/[\r\n]/.test(token), 'Supported private primary auth required');
let ws, child, timer;
const receipts = [];
try {
  child = Bun.spawn([p.binary, '--help'], { cwd: p.repo, env: p.env, stdout: 'pipe', stderr: 'pipe' });
  const exited = child.exited;
  timer = setTimeout(() => child.kill('SIGKILL'), 10000);
  const [out, err, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), exited]);
  clearTimeout(timer); assert.equal(code, 0);
  receipts.push({ kind: 'local_child', pid: child.pid, exitCode: code, stdoutSha256: sha(out), stderrSha256: sha(err) });
  ws = new WebSocket(p.env.OMON_OMO_APPSERVER_URL, { headers: { Authorization: `Bearer ${token}` } });
  const request = { id: 'task12-local-initialize', method: 'initialize', params: { capabilities: { experimentalApi: true }, clientInfo: { name: 'task12-local-preflight', version: '1' } } };
  const response = await new Promise((resolve, reject) => {
    timer = setTimeout(() => reject(new Error('Authenticated initialize deadline')), 15000);
    ws.addEventListener('message', event => {
      try { const value = JSON.parse(String(event.data)); if (value.id === request.id) { value.error ? reject(new Error('Initialize RPC rejected')) : resolve(value); } }
      catch (error) { reject(error); }
    });
    ws.addEventListener('error', () => reject(new Error('Authenticated WebSocket failed')));
    ws.addEventListener('close', () => reject(new Error('WebSocket closed before initialize')));
    ws.addEventListener('open', () => ws.send(JSON.stringify(request)), { once: true });
  });
  clearTimeout(timer);
  assert.equal(typeof response.result?.userAgent, 'string');
  receipts.push({ kind: 'authenticated_initialize', request, response });
  assert.equal(sha(readFileSync(p.binary)), p.binarySha256);
  const preflight = {
    kind: 'local-authenticated-preflight-v1', status: 'observed', purpose: 'task12-question',
    preparedSha256: sha(bytes), root: p.root, dbPath: p.dbPath, cwd: p.repo, endpoint: p.env.OMON_OMO_APPSERVER_URL,
    binarySha256: p.binarySha256, manifestSha256: p.manifestSha256, observedAt: new Date().toISOString(),
    localChild: { executable: p.binary, argv: [p.binary, '--help'], cwd: p.repo, explicitEnv: p.env,
      databaseUrl: p.databaseUrl, exitCode: code, stdoutSha256: sha(out), stderrSha256: sha(err) },
    protocol: { authenticated: true, requestId: request.id, method: request.method, response: response.result },
    remoteEnvironmentObserved: false, remoteTurnCreated: false,
    databaseIsolation: 'must be proven by gateway descriptor and PRAGMA before lane trigger'
  };
  writeFileSync(path, JSON.stringify({ schema: 'omon-lane-local-preflight-v1', preflight, receipts }, null, 2), { flag: 'wx', mode: 0o600 });
  console.log(JSON.stringify({ result: 'LOCAL_PREFLIGHT_OBSERVED_NO_TURN', path, sha256: sha(readFileSync(path)) }));
} catch (error) {
  writeFileSync(join(p.root, `local-preflight-failure-${Date.now()}.json`), JSON.stringify({ receipts, error: error.message, remoteTurnCreated: false }), { flag: 'wx', mode: 0o600 });
  throw error;
} finally { clearTimeout(timer); ws?.close(); if (child?.exitCode === null) { child.kill('SIGKILL'); await child.exited; } }
