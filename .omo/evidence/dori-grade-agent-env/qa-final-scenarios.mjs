#!/usr/bin/env bun
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { runQA, block, seedItem, lead, bot, owner, reservePort, sha } from './qa-final-common.mjs';
const sourceRoot = '/Users/indo/code/project/omon-gateway-wt/dori-work';
export function sentPosts(rows) { return rows.map(x => JSON.parse(x.text)).filter(x => x.state === 'sent'); }
export function actualUsage(rows) { return rows.length > 0 && rows.every(x => Number.isInteger(x.input_tokens) && x.input_tokens >= 0 && Number.isInteger(x.output_tokens) && x.output_tokens >= 0 && x.input_tokens + x.output_tokens > 0); }
export function tokenVerdict(rows) { return actualUsage(rows) ? 'PASS' : rows.length ? 'FAIL' : 'BLOCKED-UPSTREAM'; }
export const personaFixture = {
  'range.mjs': 'export function inclusiveRange(first, last) { return Array.from({ length: last - first }, (_, i) => first + i); }\n',
  'schedule.mjs': "import { inclusiveRange } from './range.mjs';\nexport function scheduledDays(first, last) { return inclusiveRange(first, last).map(day => ({ day })); }\n",
  'range.test.mjs': "import { test } from 'node:test';\nimport assert from 'node:assert/strict';\nimport { inclusiveRange } from './range.mjs';\ntest('inclusive endpoints', () => assert.deepEqual(inclusiveRange(2, 4), [2, 3, 4]));\ntest('single endpoint', () => assert.deepEqual(inclusiveRange(5, 5), [5]));\n",
  'schedule.test.mjs': "import { test } from 'node:test';\nimport assert from 'node:assert/strict';\nimport { scheduledDays } from './schedule.mjs';\ntest('schedule includes last day', () => assert.deepEqual(scheduledDays(1, 2), [{ day: 1 }, { day: 2 }]));\n",
};
export function rejectedDiff(history) { return history.some(x => x.from_status === 'done_claimed' && x.to_status === 'not_done' && /done_diff fulfills=[0-9.]+ clean=[0-9.]+ risk=[0-9.]+/.test(x.reason ?? '')); }
export function closedProof(lane, history, archived, removed) { return lane?.status === 'closed' && history.some(x => x.to_status === 'verified_done') && archived && removed; }
function lane(c, key) { return c.db.query('SELECT * FROM lanes WHERE key=?').get(key); }
function history(c, key) { return c.snapshot('SELECT * FROM lane_history WHERE lane_key=? ORDER BY id', key); }
function completed(c, key, token) { const rows = c.snapshot('SELECT * FROM lane_transcript WHERE lane_key=? ORDER BY id', key); const agent = rows.find(x => x.role === 'agent' && x.text.includes(token)); return agent && rows.some(x => x.id > agent.id && x.role === 'outcome' && x.text === 'completed') ? rows : null; }
async function newRepo(c, name, committed = false) {
  const path = c.root + '/workspace/' + name; mkdirSync(path, { mode: 0o700 });
  await c.cmd(['/usr/bin/git', 'init', path]);
  if (committed) {
    writeFileSync(path + '/README.md', 'QA base\n', { mode: 0o600 });
    await c.cmd(['/usr/bin/git', '-C', path, 'add', 'README.md']);
    await c.cmd(['/usr/bin/git', '-C', path, '-c', 'user.name=Owned QA', '-c', 'user.email=qa@example.invalid', 'commit', '-m', 'QA fixture baseline']);
  }
  return path;
}
async function startLane(c, key, cwd, instructions, done = 'file done.txt', thread = false) {
  const brief = c.root + '/' + key + '-brief.md';
  writeFileSync(brief, 'Owned local QA. Work only inside ' + cwd + '. No network, delegation, source messaging, services, or production files. Use supplied DATABASE_URL and the omo-gateway on PATH. Do not change models. ' + instructions, { mode: 0o600 });
  const args = [c.binary, 'lane', 'start', key, '--title', 'QA2 ' + key, '--brief', brief, '--done', done, '--cwd', cwd, '--lead', lead];
  if (thread) args.push('--thread', '700000000000000002');
  await c.cmd(args);
  return brief;
}
async function settle(c, key, target) {
  return c.observe(() => {
    const h = history(c, key), claim = h.filter(x => x.to_status === 'done_claimed' && x.from_status !== 'done_claimed').at(-1);
    if (!claim) return null;
    const result = h.find(x => x.id > claim.id && x.from_status === 'done_claimed' && x.to_status === target);
    if (result) { assert(Date.parse(result.at) >= Date.parse(claim.at) + 300000, 'No early settlement'); return { lane: lane(c, key), history: h }; }
    return null;
  }, key + ' real five-minute settlement', 660000);
}
async function closeScenario(release, dirty) {
  return runQA(15, release, { name: dirty ? 'dirty-worktree-refused' : 'close-archive-reply-unarchive', model: true, sink: true }, async c => {
    const repo = await newRepo(c, 'base', true), worktree = c.root + '/workspace/lane-worktree';
    await c.cmd(['/usr/bin/git', '-C', repo, 'worktree', 'add', '-b', 'qa-owned', worktree]);
    const key = dirty ? 'qa15-dirty' : 'qa15-clean';
    const finished = c.observe(() => completed(c, key, 'QA15_READY'), 'initial lane turn'); finished.catch(() => {});
    await startLane(c, key, worktree, 'Create done.txt containing QA15_READY. Emit QA15_READY and a blocker report, then end. Do not claim, commit, remove worktrees, or delete branches.', 'file done.txt', true);
    await finished;
    await c.cmd([c.binary, 'lane', 'register-worktree', key, '--repo', repo, '--path', worktree]);
    // The clean brief asks the lane itself to commit done.txt; only commit here
    // when the lane left it uncommitted (the dirty brief forbids committing).
    if ((await c.cmd(['/usr/bin/git', '-C', worktree, 'status', '--porcelain', '--', 'done.txt'])).trim()) {
      await c.cmd(['/usr/bin/git', '-C', worktree, 'add', 'done.txt']);
      await c.cmd(['/usr/bin/git', '-C', worktree, '-c', 'user.name=Owned QA', '-c', 'user.email=qa@example.invalid', 'commit', '-m', 'QA owned completion signal']);
    }
    if (dirty) writeFileSync(worktree + '/README.md', 'Intentional uncommitted QA change\n');
    const settlement = settle(c, key, dirty ? 'not_done' : 'verified_done'); settlement.catch(() => {});
    // claim from blocked is legal; live command, no synthetic status/deadline update.
    // The clean lane claims done itself; only the dirty lane (told not to claim) needs the harness claim.
    if (lane(c, key).status !== 'done_claimed') await c.cmd([c.binary, 'lane', 'claim-done', key, '--evidence', 'QA15 actual file signal']);
    const proof = await settlement; c.save('settlement.json', proof);
    if (dirty) {
      assert(proof.lane.objection.includes('uncommitted'));
      assert(!history(c, key).some(x => x.to_status === 'closed'));
      await c.cmd([c.binary, 'lane', 'close', key], 2);
      assert(existsSync(worktree + '/README.md'));
      assert((await c.cmd(['/usr/bin/git', '-C', repo, 'worktree', 'list', '--porcelain'])).includes(worktree));
      c.save('dirty-retained.json', { status: lane(c, key).status, reason: proof.lane.objection, worktreeRetained: true }); return;
    }
    const close = c.observe(() => lane(c, key)?.status === 'closed' && Array.from(c.sink.channels.values()).some(x => x.id === lane(c, key).thread_channel_id && x.thread_metadata.archived) ? lane(c, key) : null, 'closed and archived'); close.catch(() => {});
    await c.cmd([c.binary, 'lane', 'close', key]); await close;
    assert(!(await c.cmd(['/usr/bin/git', '-C', repo, 'worktree', 'list', '--porcelain'])).includes(worktree));
    assert((await c.cmd(['/usr/bin/git', '-C', repo, 'branch', '--list', 'qa-owned'])).includes('qa-owned'));
    const savedThread = lane(c, key).omo_thread_id; assert(savedThread);
    assert(closedProof(lane(c, key), history(c, key), c.sink.channels.get(lane(c, key).thread_channel_id).thread_metadata.archived, !existsSync(worktree)));
    const reopened = c.observe(() => completed(c, key, 'QA15_REOPEN_REPLY') && c.snapshot("SELECT * FROM lane_owner_replies WHERE lane_key=? AND state='applied'", key).length === 1 && c.sink.channels.get(lane(c, key).thread_channel_id)?.thread_metadata.archived === false ? lane(c, key) : null, 'durable reply reopened real turn'); reopened.catch(() => {});
    // This is the durable adapter receipt boundary, NOT an authenticated Discord inbound message.
    const generation = c.db.query("SELECT COALESCE(MAX(id),0) n FROM lane_history WHERE lane_key=? AND reason='reopened'").get(key).n;
    c.db.query('INSERT INTO lane_owner_replies(message_id,lane_key,bot_id,author_id,text,generation) VALUES(?,?,?,?,?,?)').run('qa15-owned-reply', key, bot, owner, 'Do not do any tools. Emit QA15_REOPEN_REPLY plus a blocker report, then end.', generation);
    await reopened; assert.equal(lane(c, key).omo_thread_id, savedThread);
    assert.equal(history(c, key).filter(x => x.reason === 'reopened').length, 1);
    c.save('reopen-receipt.json', { lane: lane(c, key), history: history(c, key), replies: c.snapshot('SELECT * FROM lane_owner_replies'), chatMessageCount: c.db.query('SELECT count(*) n FROM messages').get().n });
    block(c, 'authenticated adapter reopen-on-owner-reply', 'Loopback REST sink cannot deliver Discord gateway inbound events. Durable lane_owner_replies receipt, session reuse, single reopen, archive/unarchive and real turn are exercised; adapter authentication/routing is not. No live Discord API permitted.');
  });
}
async function kibitzerScenario(release, idle) {
  return runQA(16, release, { name: idle ? 'idle-ten-minutes' : 'inbox-insert-real-kibitzer', model: true, env: { OMON_KIBITZER_ENABLED: 'true' } }, async c => {
    assert(c.db.query("SELECT name FROM sqlite_master WHERE name='kibitzer_inbox_insert'").get(), 'Enabled kibitzer must be wired');
    const state = () => ({ rows: c.snapshot('SELECT * FROM kibitzer_state'), usage: c.snapshot("SELECT * FROM model_usage WHERE purpose='kibitzer'"), judges: c.snapshot("SELECT * FROM judge_calls WHERE purpose='kibitzer_wake'"), events: c.snapshot("SELECT * FROM lane_events WHERE kind='kibitzer'") });
    const before = state(); assert.equal(before.usage.length, 0); c.save('kibitzer-initial.json', before);
    if (idle) {
      // Duration itself is the behavior under test. One timer; WAL events immediately reject activity.
      await new Promise((resolve, reject) => {
        const finish = () => { clearTimeout(timer); c.bus.off('change', inspect); c.signal.removeEventListener('abort', abort); };
        const abort = () => { finish(); reject(c.signal.reason); };
        const inspect = () => { try { assert.deepEqual(state(), before); } catch (e) { finish(); reject(e); } };
        c.bus.on('change', inspect);
        const timer = setTimeout(() => { finish(); try { assert.deepEqual(state(), before); resolve(); } catch(e) { reject(e); } }, 600000);
        c.signal.addEventListener('abort', abort, { once: true });
        if (c.signal.aborted) { abort(); return; }
        inspect();
      }); c.save('idle-ten-minutes.json', { elapsedTargetMs: 600000, state: state() }); return;
    }
    const hour = new Date().getUTCHours(); if ((hour + 9) % 24 < 8) { block(c, 'kibitzer happy', 'Outside 08:00-24:00 KST. No clock manipulation or idle model polling.'); return; }
    const decision = c.observe(() => { const s = state(); if (s.events.length) { assert.equal(s.events.length, 1); const d = JSON.parse(s.events[0].body); assert.equal(typeof d.wake, 'boolean'); assert.equal(typeof d.reason, 'string'); assert.equal(typeof d.suggested, 'string'); assert(d.target_lane === null || typeof d.target_lane === 'string'); assert.equal(s.usage.length, 1); assert(actualUsage(s.usage)); return { ...s, parsed: d }; } return null; }, 'single real kibitzer decision', 180000); decision.catch(() => {});
    seedItem(c, 'qa16-new', null, 'Owner must decide an urgent operational blocker now; agent should ask for a recommendation immediately.');
    const result = await decision;
    c.save('kibitzer-result.json', result);
    if (result.parsed.wake) {
      c.save('kibitzer-lead-turn.json', await c.observe(() => reply(c, lead), 'kibitzer real lead wake turn', 240000));
    } else block(c, 'kibitzer lead wake', 'Actual small-model decision was wake=false; no fabricated wake or model polling.');
  });
}
async function milestoneScenario(release, replacement) {
  return runQA(18, release, { name: replacement ? 'deleted-status-replaced' : 'two-milestones', model: true, sink: true }, async c => {
    const cwd = await newRepo(c, 'lane'), key = 'qa18';
    const posts = () => sentPosts(c.snapshot("SELECT * FROM lane_transcript WHERE lane_key=? AND role='thread_post' ORDER BY id", key));
    const first = c.observe(() => posts().filter(x => x.body.includes('QA18_ONE')).length === 1 && lane(c, key)?.status_message_id ? lane(c, key) : null, 'first real milestone and status'); first.catch(() => {});
    await startLane(c, key, cwd, 'Do no tools. Emit exactly a milestone report for QA18_ONE, then a blocker report saying waiting for lead. End.', 'file unused.txt', true);
    await first;
    const old = lane(c, key).status_message_id;
    if (replacement) {
      const response = await fetch(c.sink.origin + '/api/v10/channels/' + lane(c, key).thread_channel_id + '/messages/' + old, { method: 'DELETE' }); assert.equal(response.status, 204);
    }
    const offset = c.sink.requests.length;
    const second = c.observe(() => {
      const p = posts(), l = lane(c, key), requests = c.sink.requests.slice(offset);
      if (p.filter(x => x.body.includes('QA18_TWO')).length !== 1) return null;
      if (replacement && (!l.status_message_id || l.status_message_id === old || !requests.some(x => x.status === 404))) return null;
      if (!replacement && !requests.some(x => x.method === 'PATCH' && x.path.includes('/messages/'))) return null;
      return { lane: l, posts: p, requests };
    }, 'second milestone acknowledged with status update'); second.catch(() => {});
    await c.cmd([c.binary, 'lane', 'send', key, '--text', 'Do no tools. Emit milestone report QA18_TWO then blocker report waiting for lead. End.']);
    const proof = await second;
    assert.equal(proof.posts.filter(x => x.body.includes('QA18_ONE')).length, 1); assert.equal(proof.posts.filter(x => x.body.includes('QA18_TWO')).length, 1);
    const sendCount = text => c.sink.requests.filter(x => x.method === 'POST' && x.body?.content?.includes(text)).length;
    assert.equal(sendCount('QA18_ONE'), 1); assert.equal(sendCount('QA18_TWO'), 1);
    c.save('milestone-result.json', proof);
  });
}
export async function dashboard(c) {
  const port = await reservePort(); c.env.DASHBOARD_ENABLED = 'true'; c.env.DASHBOARD_PORT = String(port); c.env.DASHBOARD_HOST = '127.0.0.1';
  await c.stop(); await c.start();
  const origin = 'http://127.0.0.1:' + port;
  const res = await fetch(origin + '/api/readiness', { signal: AbortSignal.timeout(15000) }); assert.equal(res.status, 200);
  return origin;
}
export async function chat(c, origin, session, message) {
  const res = await fetch(origin + '/api/sessions/' + encodeURIComponent(session) + '/chat', { method: 'POST', headers: { 'Content-Type': 'application/json', Origin: origin }, body: JSON.stringify({ message }), signal: AbortSignal.timeout(15000) });
  assert.equal(res.status, 202); return await res.json();
}
export function reply(c, session, after = 0) {
  const rows = c.snapshot('SELECT role,content,sequence FROM messages WHERE session_key=? AND sequence>? ORDER BY sequence', session, after);
  const delivered = c.snapshot("SELECT * FROM delivery_obligations WHERE session_key=? AND state='delivered'", session);
  return rows.some(x => x.role === 'assistant' && delivered.some(y => y.content === x.content)) ? rows : null;
}
async function profileChat(c) {
  const repo = await newRepo(c, 'qa20');
  for (const [name, text] of Object.entries(personaFixture)) writeFileSync(repo + '/' + name, text, { mode: 0o600 });
    const profile = JSON.parse(readFileSync(sourceRoot + '/bot_profiles/omomeow.json', 'utf8'));
  c.db.query('INSERT INTO bot_profiles(bot_id,name,model,system_prompt,enabled_toolsets) VALUES(?,?,?,?,?)').run(bot, profile.name ?? 'QA', profile.model ?? null, profile.system_prompt, JSON.stringify(profile.enabled_toolsets ?? ['terminal','file']));
  const immutable = () => c.db.query('SELECT name,model,enabled_toolsets FROM bot_profiles WHERE bot_id=?').get(bot);
  const before = immutable();
  c.save('profile-before.json', JSON.parse(await c.cmd([c.binary, 'profile', 'export', bot])));
  await c.cmd([c.binary, 'profile', 'apply-block', '--bot-id', 'unknown-qa-bot', '--block-file', sourceRoot + '/bot_profiles/blocks/lanes.md'], 2);
  await c.cmd([c.binary, 'profile', 'apply-block', '--bot-id', bot, '--block-file', sourceRoot + '/bot_profiles/blocks/lanes.md']);
  const prompt = c.db.query('SELECT system_prompt FROM bot_profiles WHERE bot_id=?').get(bot).system_prompt;
  await c.cmd([c.binary, 'profile', 'apply-block', '--bot-id', bot, '--block-file', sourceRoot + '/bot_profiles/blocks/lanes.md']);
  assert.equal(sha(prompt), sha(c.db.query('SELECT system_prompt FROM bot_profiles WHERE bot_id=?').get(bot).system_prompt)); assert.deepEqual(immutable(), before);
  const session = lead;
  c.db.query('INSERT INTO sessions(session_key,platform,channel_id,user_id,state_json) VALUES(?,?,?,?,?)').run(session, 'discord', '1474231559895646240', owner, JSON.stringify({ system_prompt: prompt + '\nThis is isolated QA; only work in ' + repo + '. No network, services, or outside writes. Never send to Discord.', active_model: before.model, enabled_toolsets: ['terminal','file'] }));
  const origin = await dashboard(c);
  const initialBoundary = c.db.query('SELECT COALESCE(MAX(sequence),0) n FROM messages WHERE session_key=?').get(session).n;
  const started = c.observe(() => reply(c, session, initialBoundary), 'persona actual reply', 240000); started.catch(() => {});
  await chat(c, origin, session, '이 레포 테스트 실패 다 고쳐줘 (repo: ' + repo + ')');
  const actualReply = await started, lanes = c.snapshot('SELECT * FROM lanes');
  c.save('persona-lane-start.json', { rows: actualReply, lanes, verdict: 'NEEDS-REVIEW', reason: 'Read actual tool/reply evidence to judge delegation; no semantic automatic PASS.' });
  if (!lanes.length) block(c, 'persona delegation', 'NEEDS-REVIEW: model replied without starting a lane; retain actual reply, do not alter the user message.');
  const boundary = c.db.query('SELECT COALESCE(MAX(sequence),0) n FROM messages WHERE session_key=?').get(session).n;
  const eventReply = c.observe(() => reply(c, session, boundary), 'real synthetic lane-event reply', 240000); eventReply.catch(() => {});
  await chat(c, origin, session, '[LANE_EVENT] qa-1 question: A or B?');
  const rows = await eventReply;
  c.save('persona-question.json', { rows, lanes: c.snapshot('SELECT * FROM lanes') });
  // Recommendation semantics require QA-by-read as the plan specifies; no prose-pinning test.
  block(c, 'question response semantic review', 'Read persona-question.json: confirm lane send tool call or an owner question with a recommendation, not verbatim forwarding. Stored actual reply is not an automatic semantic PASS.');
}
async function collectScenario(c) {
  // Probe only read operations, retaining counts and numeric exits, never source message text.
  await c.cmd([c.root + '/bin/zele', 'mail', 'list', '--folder', 'inbox', '--account', 'freedomzero91@gmail.com', '--filter', 'is:unread -category:promotions -category:social -category:updates', '--limit', '1'], 0, { source: true });
  await c.cmd(['/opt/homebrew/bin/imsg', 'chats', '--limit', '1', '--json'], 0, { source: true });
  const pirateURL = process.env.OMON_PIRATETALK_URL ?? 'https://indo.taila0cef2.ts.net';
  const pirate = await fetch(pirateURL + '/api/rooms', { signal: AbortSignal.timeout(15000) }); assert.equal(pirate.status, 200); const bytes = (await pirate.arrayBuffer()).byteLength;
  c.save('source-preflight.json', { pirateHTTP: pirate.status, pirateBytes: bytes, contentLogged: false });
  const counts = () => ({ cursors: c.snapshot('SELECT source,cursor,updated_at,error IS NOT NULL AS failed FROM inbox_cursors'), classes: c.snapshot('SELECT class,count(*) count FROM inbox_decisions GROUP BY class'), items: c.db.query('SELECT count(*) n FROM inbox_items').get().n, turns: c.db.query("SELECT count(*) n FROM model_usage WHERE purpose='inbox_write'").get().n });
  const initial = counts(); c.save('collector-initial.json', initial);
  // Enable producer only after descriptor, PRAGMA and subscriptions have been established.
  await c.stop(); c.env.OMON_INBOX_ENABLED = 'true';
  const first = c.observe(() => { const s = counts(); return s.cursors.length >= 3 && s.cursors.every(x => !x.failed) && s.items > 0 && s.classes.reduce((n, x) => n + x.count, 0) === s.items ? s : null; }, 'real headless collection and triage', 300000); first.catch(() => {});
  await c.start(); const one = await first; assert.equal(one.turns, 1, 'First live collection must include exactly one real writing turn'); c.save('collector-first-counts.json', one);
  await c.stop();
  const boundary = counts();
  const second = c.observe(() => { const s = counts(); return s.cursors.length >= 3 && s.cursors.every(x => boundary.cursors.some(b => b.source === x.source && b.updated_at !== x.updated_at)) ? s : null; }, 'second startup source collection', 180000); second.catch(() => {});
  await c.start(); await second; await c.stop();
  const two = counts(); assert.equal(two.items, boundary.items, 'New live arrivals make the unchanged-input scenario inconclusive'); assert.equal(two.turns, boundary.turns); assert.deepEqual(two.classes, boundary.classes);
  c.save('collector-second-counts.json', two);
}
async function digestScenario(c) {
  seedItem(c, 'qa25-owner', 'owner'); seedItem(c, 'qa25-bot', 'bot');
  c.save('list-not-rerun.json', { priorVerdict: 'PASS', rerun: false });
  const jobs = c.snapshot("SELECT * FROM cron_jobs WHERE id='omon:inbox-digest'");
  if (!jobs.length) {
    block(c, 'native digest local headless delivery', 'Current headless main.rs never constructs CronScheduler/DigestExecutor or registers omon:inbox-digest; standalone dashboard uses AgentCronExecutor, not DigestExecutor. Do not substitute a script/model-generated digest for the native executor.');
    c.save('digest-unreachable.json', { jobs, nativeDigestRegistered: false, outputCount: c.db.query('SELECT count(*) n FROM delivery_obligations').get().n }); return;
  }
  // Runs only if release exposes the real native job on an attached loopback scheduler.
  const origin = await dashboard(c), baseline = c.db.query("SELECT count(*) n FROM cron_runs WHERE job_id='omon:inbox-digest'").get().n;
  const delivered = c.observe(() => {
    const runs = c.snapshot("SELECT * FROM cron_runs WHERE job_id='omon:inbox-digest' ORDER BY started_at");
    const outputs = c.snapshot("SELECT * FROM cron_outputs WHERE job_id='omon:inbox-digest'");
    const obligations = c.snapshot("SELECT * FROM delivery_obligations WHERE state='delivered'");
    return runs.length > baseline && runs.at(-1).status === 'succeeded' && outputs.some(x => x.run_id === runs.at(-1).run_id && obligations.some(y => y.content === x.output)) && c.logs().includes('headless local output accepted') ? { runs, outputs, obligations } : null;
  }, 'native digest local output', 180000); delivered.catch(() => {});
  const res = await fetch(origin + '/api/cron/jobs/omon%3Ainbox-digest/trigger', { method: 'POST', headers: { Origin: origin }, signal: AbortSignal.timeout(15000) }); assert.equal(res.status, 200);
  c.save('digest-delivery.json', await delivered);
}
async function approvalScenario(c) {
  const repo = await newRepo(c, 'qa26'); seedItem(c, 'qa26', 'bot');
  const proposal = JSON.parse(await c.cmd([c.binary, 'inbox-propose', 'qa26', '--channel', '1474231559895646240', '--action-json', JSON.stringify({ kind: 'repo', cwd: repo, done: 'file done.txt' })]));
  assert.equal(c.snapshot('SELECT * FROM lanes').length, 0);
  c.save('unapproved-initial.json', { proposalNumber: proposal.number, lanes: 0 });
  // Real turn exercises refusal without presenting/approving; use durable lane-event path.
  const token = 'QA26_NO_APPROVAL';
  const turn = c.observe(() => c.snapshot("SELECT * FROM messages WHERE role='assistant' AND content=?", token).length && c.snapshot("SELECT * FROM delivery_obligations WHERE state='delivered' AND content=?", token).length ? true : null, 'unapproved real lead turn'); turn.catch(() => {});
  c.db.query('INSERT INTO lane_events(lead_session_key,kind,body,created_at) VALUES(?,?,?,?)').run(lead, 'done_claimed', 'Owned QA. No owner approval exists for inbox item qa26. Do not run tools or start work. Reply exactly ' + token, new Date().toISOString()); await turn;
  assert.equal(c.snapshot('SELECT * FROM lanes').length, 0); assert.equal(c.db.query('SELECT status FROM inbox_decisions WHERE item_id=?').get('qa26').status, 'proposed');
  c.save('unapproved-real-turn.json', { lanes: 0, usage: c.snapshot('SELECT * FROM model_usage'), proposal: c.snapshot('SELECT number,state,approval_message_id FROM inbox_proposals') });
  await c.cmd([c.binary, 'inbox', 'set', 'qa26', '--status', 'approved']);
  // QA2 source settlement: local approval now executes the native proposal itself.
  const approvedLane = c.observe(() => {
    const rows = c.snapshot('SELECT * FROM lanes WHERE cwd=?', repo);
    const item = c.db.query('SELECT * FROM inbox_decisions WHERE item_id=?').get('qa26');
    return rows.length === 1 && item.lane_key === rows[0].key && rows[0].omo_thread_id && c.snapshot('SELECT * FROM lane_transcript WHERE lane_key=? AND role=? AND text=?', rows[0].key, 'outcome', 'completed').length ? { lanes: rows, decision: item } : null;
  }, 'approved item real lead and lane turn', 300000); approvedLane.catch(() => {});
  c.save('approved-lane.json', await approvedLane);
  block(c, 'authenticated approval -> lane', 'NEEDS-REVIEW: local inbox set approval exercises native proposal execution, not authenticated Discord numbered-reply ingress. No fabricated CAS approval rows.');
}
async function usageScenario(release, negative) {
  return runQA(35, release, { name: negative ? 'real-unrelated-diff-rejected' : 'real-lane-tokens', model: true }, async c => {
    const repo = await newRepo(c, 'qa35', negative), key = negative ? 'qa35-negative' : 'qa35-positive';
    const turn = c.observe(() => completed(c, key, 'QA35_READY'), 'real completed turn', 240000); turn.catch(() => {});
    const instructions = negative ? 'Negative gate QA: the required goal is a working arithmetic implementation and tests. Intentionally do not implement it. Edit ONLY README.md, commit the unrelated edit locally using QA identity, do NOT push or contact network. Create an untracked done.txt signal. Emit QA35_READY plus blocker report; do not claim. When rejected do not repair or re-claim: emit QA35_REJECTED plus blocker report.' : 'Create untracked done.txt containing QA35_READY; emit QA35_READY plus blocker report, then end. Do not claim. No commits.';
    await startLane(c, key, repo, instructions);
    await turn;
    const rows = c.snapshot("SELECT * FROM model_usage WHERE purpose IN ('lane_small','lane_large')"), verdict = tokenVerdict(rows);
    c.save('real-token-rows.json', { rows, verdict });
    // Final owner scope: verdict depends on done_diff and judge_calls only.
    const usage = JSON.parse(await c.cmd([c.binary, 'usage', '--since', '1h', '--json'])); c.save('usage-totals.json', usage);
    const proof = settle(c, key, negative ? 'not_done' : 'verified_done'); proof.catch(() => {});
    await c.cmd([c.binary, 'lane', 'claim-done', key, '--evidence', 'QA35 actual signal and turn']);
    const result = await proof;
    if (negative) { assert(rejectedDiff(result.history)); assert(c.snapshot("SELECT * FROM judge_calls WHERE purpose='done_diff'").length > 0); }
    else { const closing = c.observe(() => lane(c, key)?.status === 'closed' ? lane(c, key) : null, 'usage lane closed'); closing.catch(() => {}); await c.cmd([c.binary, 'lane', 'close', key]); await closing; }
    c.save('done-gate-result.json', { ...result, judges: c.snapshot('SELECT * FROM judge_calls') });
    assert(c.snapshot("SELECT * FROM judge_calls WHERE purpose='done_diff'").length > 0);
  });
}
async function otherScenario(n, release) {
  assert([20,24,25,26].includes(n));
  return runQA(n, release, { name: 'task' + n + '-scenarios', model: true }, async c => {
    if (n === 20) await profileChat(c);
    if (n === 24) await collectScenario(c);
    if (n === 25) await digestScenario(c);
    if (n === 26) await approvalScenario(c);
  });
}

export async function executeTodo(n, release) {
  const results = [];
  if (n === 15) results.push(...await Promise.all([closeScenario(release, false), closeScenario(release, true)]));
  else if (n === 16) results.push(await kibitzerScenario(release, false));
  else if (n === 18) results.push(...await Promise.all([milestoneScenario(release, false), milestoneScenario(release, true)]));
  else if (n === 35) results.push(await usageScenario(release, true));
  else results.push(await otherScenario(n, release));
  return results;
}

if (import.meta.main) {
  const [mode, todo, release, ...extra] = process.argv.slice(2);
  assert(mode === 'execute' && !extra.length && [15,16,18,20,24,25,26,35].includes(Number(todo)), 'Usage: bun qa2-scenarios.mjs execute <todo> /absolute/release.json');
  const results = await executeTodo(Number(todo), release);
  process.exitCode = results.some(x => x.exitCode === 1) ? 1 : results.some(x => x.exitCode === 3) ? 3 : 0;
}
