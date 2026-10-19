import assert from 'node:assert/strict';
import { runQA, reservePort } from './qa-final-common.mjs';
import { chmodSync } from 'node:fs';
chmodSync(new URL('./qa29-bin/omo', import.meta.url), 0o700);
const port = await reservePort();
assert.notEqual(port, 9119);
const r = await runQA(29, process.argv[2], { name: 'task29-direct-live', model: true, env: {
  DASHBOARD_ENABLED: 'false',
  OMON_SESSION_HOSTS: 'local',
  PATH: new URL('./qa29-bin', import.meta.url).pathname + ':' + process.env.PATH,
}}, async c => {
  let dashboardLogs = '';
  const ready = c.observe(() => dashboardLogs.includes('omon dashboard listening'), 'dashboard ready', 45000);
  ready.catch(() => {});
  const dashboard = Bun.spawn([c.binary, 'dashboard', '--host', '127.0.0.1', '--port', String(port)], { cwd: c.root, env: c.env, stdout: 'pipe', stderr: 'pipe' });
  const pumps = Promise.all([dashboard.stdout, dashboard.stderr].map(async stream => {
    for await (const chunk of stream) { dashboardLogs += new TextDecoder().decode(chunk); c.bus.emit('change'); }
  }));
  try {
  await ready;
  const base = `http://127.0.0.1:${port}`;
  const request = async (id, message) => {
    const startedAt = new Date().toISOString(), start = performance.now();
    const response = await fetch(`${base}/api/sessions/${encodeURIComponent(id)}/chat`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ message }), signal: AbortSignal.timeout(30000),
    });
    return { startedAt, receivedAt: new Date().toISOString(), elapsedMs: performance.now() - start, status: response.status, body: await response.json() };
  };
  const prompt = '지금 돌아가는 세션들 상태 알려줘';
  if (!process.env.QA_SHARED_ONLY) {
  const ownerId = 'qa29-owner-' + crypto.randomUUID();
  const before = c.logs();
  const owner = await request(ownerId, prompt);
  const key = owner.body.session_id;
  const messages = c.snapshot('SELECT role,content,created_at FROM messages WHERE session_key=? ORDER BY sequence', key);
  const session = c.snapshot('SELECT state_json FROM sessions WHERE session_key=?', key);
  const obligations = c.snapshot('SELECT * FROM delivery_obligations');
  const metadata = c.snapshot('SELECT metadata_json FROM messages WHERE session_key=? AND role=?', key, 'assistant');
  c.receipt.owner = { ...owner, messages, session, obligations, logs: c.logs().slice(before.length) };
  assert.equal(owner.status, 200); assert.equal(owner.body.direct_reply, true); assert(owner.elapsedMs < 30000);
  assert.equal(messages.length, 2); assert.equal(messages[1].role, 'assistant');
  assert(messages[1].content.startsWith('local\n')); assert(!messages[1].content.includes('오류'));
  assert(!session[0].state_json.includes('omo_thread_id'));
  assert(obligations.length === 1 && obligations[0].state === 'delivered');
  assert.equal(JSON.parse(metadata[0].metadata_json).direct_reply,true);
  const list = JSON.parse(await c.cmd([c.binary,'sessions','list','--json']));
  const counts = {};
  for (const s of list.sessions) counts[s.status ?? 'unknown'] = (counts[s.status ?? 'unknown'] ?? 0) + 1;
  c.receipt.owner.list = list;
  c.receipt.owner.counts = counts;
  c.receipt.owner.metadata = metadata;
  assert.equal(messages[1].content, 'local\n- ' + Object.entries(counts).sort().map(([status,n])=>`${status} ${n}`).join(', '));
  assert(!c.receipt.owner.logs.includes('turn/start'));
  c.receipt.owner.verdict = 'PASS';
  console.log('OWNER_PASS ' + JSON.stringify(c.receipt.owner));
  }
  const parts = ['discord','700000000000000001','700000000000000002',null,'414011632306618368','1465631383862120451'];
  const shared = parts.map(x => x === null ? '-' : `${x.length}:${x}`).join('|');
  c.db.query('INSERT INTO sessions(session_key,platform,guild_id,channel_id,thread_id,user_id,state_json) VALUES(?,?,?,?,?,?,?)').run(shared,parts[0],parts[1],parts[2],null,parts[4],'{}');
  const routed = c.observe(() => {
    const rows = c.snapshot('SELECT session_key,role,content,created_at FROM messages');
    return rows.some(x => x.role === 'user' && x.content === prompt) && rows;
  }, 'shared normal route persistence', 30000);
  routed.catch(() => {});
  const offset = c.logs().length;
  const failure = await request(shared,prompt);
  c.receipt.shared = { ...failure, requestedKey: shared };
  const rows = await routed;
  c.receipt.shared = { ...failure, key: failure.body.session_id, messages: rows, obligations: c.snapshot('SELECT * FROM delivery_obligations'), logs: c.logs().slice(offset) };
  assert.equal(failure.status,202); assert.equal(failure.body.queued,true); assert(!failure.body.direct_reply);
  assert(!rows.some(x => x.role === 'assistant' && x.content.startsWith('local\n')));
  assert.equal(c.receipt.shared.obligations.length,0);
  c.receipt.shared.verdict = 'PASS';
  console.log('SHARED_PASS ' + JSON.stringify(c.receipt.shared));
  } finally {
    dashboard.kill('SIGINT');
    const deadline = setTimeout(() => dashboard.kill('SIGKILL'), 30000);
    const code = await dashboard.exited; clearTimeout(deadline); await pumps;
    c.receipt.dashboard = { pid: dashboard.pid, port, code, logs: dashboardLogs };
    assert.equal(code, 0);
  }
});
console.log('FINAL_RECEIPT ' + JSON.stringify(r));
process.exitCode = r.exitCode;
