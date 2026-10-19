import { test } from 'bun:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { readFileSync } from 'node:fs';
import { validateRelease, observe, sha, loadJudgeEnv, judgeCommand, judgeKeys, clean, mailWrapper, armDeadline, scenarioDeadlines, recorder, bot } from './qa2-common.mjs';
import { actualUsage, sentPosts, rejectedDiff, closedProof, personaFixture, tokenVerdict } from './qa2-scenarios.mjs';

test('release authority refuses holds before any disk binary read', () => {
  const held = JSON.parse(readFileSync(new URL('./wave4b-strict-gate-release.json', import.meta.url), 'utf8'));
  const forbidden = () => { throw new Error('binary read must not happen'); };
  assert.throws(() => validateRelease(held, forbidden), /Held build refused/);
  assert.throws(() => validateRelease({ status: 'released', release_allowed: false }, forbidden), /Held build refused/);
  assert.throws(() => validateRelease({ status: 'pending' }, forbidden), /Unreleased build refused/);
});
test('judge loader exports only three NUL-delimited values and redacts all', async () => {
  let calls = 0;
  const env = await loadJudgeEnv((argv, config) => {
    calls++; assert.deepEqual(argv, judgeCommand); assert.equal(config.stderr, 'ignore');
    return { stdout: new Response('key-fixture\0https://judge.invalid\0judge-model\0').body, exited: Promise.resolve(0) };
  });
  assert.equal(calls, 1); assert.deepEqual(Object.keys(env), judgeKeys);
  assert.equal(clean(Object.values(env).join(' '), env), '[redacted] [redacted] [redacted]');
  await assert.rejects(loadJudgeEnv(() => ({ stdout: new Response('missing\0\0\0').body, exited: Promise.resolve(0) })), /Three nonempty/);
});
test('mail wrapper limits owner HOME to readonly mail with write denial', () => {
  const wrapper = mailWrapper('/fixture/owner');
  assert(wrapper.includes('mail:list|mail:watch'));
  assert(wrapper.includes('*) exit 64'));
  assert(wrapper.includes('(deny file-write* (require-all (subpath "/fixture/owner") (require-not (subpath "/fixture/owner/.zele"))))'));
  assert(wrapper.includes("HOME='/fixture/owner'"));
  assert(wrapper.includes('/usr/bin/sandbox-exec'));
});
test('scenario deadline writes receipt before cancellation and cancels observers', async () => {
  assert(scenarioDeadlines[16] > 600000);
  for (const n of [15,24,26,35]) assert(scenarioDeadlines[n] > 660000);
  const receipt = {}, controller = new AbortController(), bus = new EventEmitter();
  let fire, saved;
  armDeadline(receipt, 900000, r => { assert(!controller.signal.aborted); saved = structuredClone(r); }, controller, callback => { fire = callback; return 1; });
  const observing = observe(bus, () => null, 'pending', 1000, controller.signal);
  fire(); await assert.rejects(observing, /Scenario deadline/);
  assert.equal(saved.status, 'deadline'); assert.equal(saved.exitCode, 1);
  assert.equal(saved.deadlineMs, 900000); assert.equal(bus.listenerCount('change'), 0);
});
test('recorder answers current bot identity without forwarding', async () => {
  const sink = recorder(new EventEmitter());
  try {
    const response = await fetch(sink.origin + '/api/v10/users/@me');
    assert.equal(response.status, 200);
    const user = await response.json(); assert.equal(user.id, bot); assert.equal(user.bot, true);
    assert.equal(sink.requests.length, 1); assert.equal(sink.requests[0].status, 200);
  } finally { await sink.stop(); }
});
test('persona fixture has three failures fixed by implementation only', async () => {
  const load = async code => import('data:text/javascript;base64,' + Buffer.from(code).toString('base64'));
  for (const [code, passing] of [[personaFixture['range.mjs'], false], [personaFixture['range.mjs'].replace('last - first', 'last - first + 1'), true]]) {
    const { inclusiveRange } = await load(code);
    const { scheduledDays } = await load(personaFixture['schedule.mjs'].replace("import { inclusiveRange } from './range.mjs';", 'const inclusiveRange = ' + inclusiveRange.toString() + ';'));
    const checks = [() => assert.deepEqual(inclusiveRange(2,4), [2,3,4]), () => assert.deepEqual(inclusiveRange(5,5), [5]), () => assert.deepEqual(scheduledDays(1,2), [{day:1},{day:2}])];
    for (const check of checks) if (passing) check(); else assert.throws(check);
  }
  assert.equal(Object.keys(personaFixture).length, 4);
});
test('absent upstream tokens block only token assertion, invalid counters fail', () => {
  assert.equal(tokenVerdict([]), 'BLOCKED-UPSTREAM');
  assert.equal(tokenVerdict([{input_tokens:2,output_tokens:3}]), 'PASS');
  assert.equal(tokenVerdict([{input_tokens:0,output_tokens:0}]), 'FAIL');
});
test('legacy and structured release identities require actual byte hashes', () => {
  const bytes = Buffer.from('unit fixture, not an executable'), hash = sha(bytes);
  assert.equal(validateRelease({ status: 'released', binary: '/fixture', binarySha256: hash }, () => bytes), hash);
  assert.equal(validateRelease({ release_allowed: true, binary: { path: '/fixture', sha256: hash } }, () => bytes), hash);
  assert.throws(() => validateRelease({ status: 'released', binary: '/fixture', binarySha256: '0'.repeat(64) }, () => bytes), /mismatch/);
  assert.throws(() => validateRelease({ status: 'released', binary: 'relative', binarySha256: hash }, () => bytes), /Absolute/);
  assert.throws(() => validateRelease({ status: 'released', binary: '/fixture' }, () => bytes), /SHA-256 required/);
});
test('observer reads initial authority and subscribes before triggers', async () => {
  const bus = new EventEmitter();
  assert.equal(await observe(bus, () => 7, 'already persisted', 1000), 7);
  assert.equal(bus.listenerCount('change'), 0);
  let state = null;
  const result = observe(bus, () => state, 'event', 1000);
  assert.equal(bus.listenerCount('change'), 1);
  state = { persisted: true }; bus.emit('change');
  assert.deepEqual(await result, state); assert.equal(bus.listenerCount('change'), 0);
  await assert.rejects(observe(bus, () => { throw new Error('authority failed'); }, 'error', 1000), /authority failed/);
  assert.equal(bus.listenerCount('change'), 0);
});
test('machine verdicts reject incomplete and synthetic success', () => {
  assert(actualUsage([{ input_tokens: 2, output_tokens: 3 }]));
  for (const rows of [[], [{input_tokens: 0,output_tokens: 0}], [{input_tokens: -1,output_tokens: 2}], [{input_tokens: 1.5,output_tokens: 2}]]) assert(!actualUsage(rows));
  assert.deepEqual(sentPosts([{text:'{"state":"pending"}'},{text:'{"state":"sent","id":"1"}'}]), [{state:'sent',id:'1'}]);
  const history = [{from_status:'done_claimed',to_status:'not_done',reason:'done_diff fulfills=0.01 clean=0.02 risk=1.57'}];
  assert(rejectedDiff(history)); assert(!rejectedDiff([]));
  const verified = [{to_status:'verified_done'}];
  assert(closedProof({status:'closed'}, verified, true, true));
  assert(!closedProof({status:'closed'}, [], true, true));
  assert(!closedProof({status:'closed'}, verified, false, true));
  assert(!closedProof({status:'closed'}, verified, true, false));
});
