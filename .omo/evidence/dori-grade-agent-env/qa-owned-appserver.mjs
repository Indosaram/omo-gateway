#!/usr/bin/env bun
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { EventEmitter } from 'node:events';
import { mkdtempSync, chmodSync, writeFileSync, rmSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir, homedir } from 'node:os';
import { join } from 'node:path';

// Keep this handle in memory; never print gatewayEnv or the bearer token.
// await withOwnedAppServer(async ({ gatewayEnv }) => { ...spawn owned QA gateway... });
// The caller must stop its gateway and close its sockets before returning.
function bounded(promise, label, ms) {
  let timer;
  return Promise.race([promise, new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${label} deadline`)), ms);
  })]).finally(() => clearTimeout(timer));
}

async function reservePort(port = 0) {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen({ host: '127.0.0.1', port, exclusive: true }, resolve);
  });
  const selected = server.address().port;
  await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  return selected;
}

async function groupMembers(pgid) {
  const child = Bun.spawn(['/bin/ps', '-axo', 'pid=,pgid='], { stdout: 'pipe', stderr: 'pipe' });
  const [out, err, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
  assert.equal(code, 0, 'Process group inspection failed');
  assert.equal(err, '');
  return out.trim().split('\n').map(line => line.trim().split(/\s+/).map(Number))
    .filter(([, group]) => group === pgid).map(([pid]) => pid);
}

export async function withOwnedAppServer(caller, toolEnv = {}) {
  assert.equal(process.env.HOME, homedir(), 'Owner normal HOME is required');
  const started = performance.now();
  const port = await reservePort();
  assert.notEqual(port, 19742);
  const url = `ws://127.0.0.1:${port}`;
  const scratch = mkdtempSync(join(tmpdir(), 'omon-qa-owned-appserver-'));
  chmodSync(scratch, 0o700);
  const tokenFile = join(scratch, 'ws-token');
  const token = randomBytes(32).toString('hex');
  writeFileSync(tokenFile, token + '\n', { mode: 0o600, flag: 'wx' });
  const argv = ['omo', 'app-server', '--listen', url, '--ws-auth', tokenFile];
  const receipt = { startedAt: new Date().toISOString(), argv, cwd: process.cwd(), home: toolEnv.HOME ?? process.env.HOME,
    auth: 'private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access',
    url, port, timingsMs: {} };
  const bus = new EventEmitter();
  // Subscribe before spawning; pipes retain output until handlers are attached.
  const ready = new Promise((resolve, reject) => {
    bus.once('ready', resolve);
    bus.once('failed', reject);
  });
  ready.catch(() => {});
  let child, exited, cleanupPromise;
  const killGroup = signal => {
    if (!child) return;
    try { process.kill(-child.pid, signal); }
    catch (error) { if (error.code !== 'ESRCH') throw error; }
  };
  const cleanup = () => cleanupPromise ??= (async () => {
    const stopping = performance.now();
    if (child) {
      receipt.membersBeforeShutdown = await groupMembers(child.pid);
      if (child.exitCode === null && child.signalCode === null) {
        killGroup('SIGTERM');
        try { await bounded(exited, 'Owned group shutdown', 15000); }
        catch { killGroup('SIGKILL'); await bounded(exited, 'Owned group forced shutdown', 5000); }
      }
      const remaining = await groupMembers(child.pid);
      if (remaining.length) killGroup('SIGKILL');
      receipt.remainingGroupPids = await groupMembers(child.pid);
      assert.deepEqual(receipt.remainingGroupPids, [], 'Owned process group still exists');
      try { process.kill(child.pid, 0); receipt.pidAbsent = false; }
      catch (error) { if (error.code !== 'ESRCH') throw error; receipt.pidAbsent = true; }
      assert(receipt.pidAbsent);
      receipt.childExit = { code: child.exitCode, signal: child.signalCode };
    }
    // Exclusive rebind proves that the listener is gone, without contacting another server.
    assert.equal(await reservePort(port), port);
    receipt.portRebindSucceeded = true;
    rmSync(scratch, { recursive: true });
    receipt.privateScratchRemoved = true;
    receipt.timingsMs.cleanup = Math.round(performance.now() - stopping);
    receipt.timingsMs.total = Math.round(performance.now() - started);
    receipt.finishedAt = new Date().toISOString();
  })();
  const onExit = () => killGroup('SIGKILL');
  const onSignal = signal => {
    void cleanup().then(() => process.exit(signal === 'SIGINT' ? 130 : 143), () => {
      killGroup('SIGKILL'); process.exit(1);
    });
  };
  process.on('exit', onExit);
  process.on('SIGINT', onSignal);
  process.on('SIGTERM', onSignal);
  try {
    // detached creates a new process group, not a daemon: pipes stay attached,
    // the child is never unref'd, and this owner waits for its exit.
    child = spawn(argv[0], argv.slice(1), { cwd: process.cwd(), env: { ...process.env, ...toolEnv },
      detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
    receipt.pid = child.pid;
    exited = new Promise((resolve, reject) => {
      child.once('error', error => { bus.emit('failed', new Error('Owned app-server spawn failed')); reject(error); });
      child.once('exit', (code, signal) => {
        bus.emit('failed', new Error('Owned app-server exited before readiness'));
        resolve({ code, signal });
      });
    });
    exited.catch(() => {});
    for (const stream of [child.stdout, child.stderr]) {
      let pending = '';
      stream.setEncoding('utf8');
      stream.on('data', text => {
        pending += text;
        let end;
        while ((end = pending.indexOf('\n')) !== -1) {
          const line = pending.slice(0, end); pending = pending.slice(end + 1);
          if (line === `senpi app-server listening on ${url}`) bus.emit('ready');
        }
        // Never forward child output: extensions could print credentials.
      });
    }
    await bounded(ready, 'Pre-subscribed listener readiness', 30000);
    receipt.timingsMs.ready = Math.round(performance.now() - started);
    const handle = { url, receipt };
    Object.defineProperty(handle, 'gatewayEnv', { value: {
      OMON_OMO_APPSERVER_URL: url, OMON_OMO_APPSERVER_AUTH_TOKEN: token,
    } });
    await caller(handle);
  } finally {
    try { await cleanup(); }
    finally {
      process.off('exit', onExit); process.off('SIGINT', onSignal); process.off('SIGTERM', onSignal);
      bus.removeAllListeners();
    }
  }
  return receipt;
}

if (import.meta.main) {
  assert.deepEqual(process.argv.slice(2), ['--verify'], 'Usage: bun qa-owned-appserver.mjs --verify');
  const receipt = await withOwnedAppServer(async ({ url, gatewayEnv, receipt }) => {
    const started = performance.now();
    const request = { jsonrpc: '2.0', id: 'qa-owned-initialize-1', method: 'initialize',
      params: { clientInfo: { name: 'qa-owned-appserver', version: '1' } } };
    const ws = new WebSocket(url, { headers: { Authorization: `Bearer ${gatewayEnv.OMON_OMO_APPSERVER_AUTH_TOKEN}` } });
    try {
      const response = await bounded(new Promise((resolve, reject) => {
        ws.addEventListener('message', event => {
          try { const value = JSON.parse(String(event.data)); if (value.id === request.id) resolve(value); }
          catch { reject(new Error('Initialize response was not JSON')); }
        });
        ws.addEventListener('error', () => reject(new Error('Authenticated WebSocket failed')));
        ws.addEventListener('close', () => reject(new Error('WebSocket closed before initialize response')));
        ws.addEventListener('open', () => ws.send(JSON.stringify(request)), { once: true });
      }), 'Authenticated initialize', 15000);
      assert.equal(response.id, request.id);
      assert.equal(response.error, undefined, 'Initialize RPC rejected');
      assert.equal(typeof response.result?.userAgent, 'string');
      receipt.initialize = { request, response, authenticated: true, requestCount: 1, threadsCreated: 0, modelTurns: 0 };
      receipt.timingsMs.initialize = Math.round(performance.now() - started);
    } finally {
      if (ws.readyState !== WebSocket.CLOSED) {
        const closed = new Promise(resolve => ws.addEventListener('close', resolve, { once: true }));
        ws.close(); await bounded(closed, 'Verification socket close', 5000);
      }
    }
  });
  assert.deepEqual(receipt.childExit, { code: 0, signal: null });
  console.log('QA_OWNED_APPSERVER_VERIFIED ' + JSON.stringify(receipt));
}
