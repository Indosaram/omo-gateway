import { test, mock } from 'bun:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync } from 'node:fs';

test('task29 records started-session evidence before deadline cleanup without extending 30 seconds', async () => {
  const root = mkdtempSync('/private/tmp/omon-task29-offline-');
  mkdirSync(root+'/bin');
  mkdirSync(root+'/home');
  const calls = [], saved = new Map(), deadline = new Error('reply deadline');
  const sessions = [{session_key:'lead',state_json:'{"metadata":{"omo_thread_id":"owned-thread"}}'}];
  const c = {
    root, env:{HOME:root+'/home'}, receipt:{},
    db:{query:()=>({run:()=>{}})},
    save:(name,value)=>{ calls.push(name); saved.set(name,value); },
    logs:()=> 'dashboard listening',
    snapshot:sql=>sql.includes('FROM sessions') ? sessions : sql.includes('FROM messages') ? [{role:'user'}] : [],
    observe:(_check,_label,ms)=>{ assert.equal(ms,30000); calls.push('armed'); return Promise.reject(deadline); },
  };
  mock.module('./qa-final-common.mjs', () => ({
    lead:'lead',bot:'bot',owner:'owner',block:()=>{},
    runQA:async (_n,_release,_options,scenario)=>{
      try { await scenario(c); } finally { calls.push('stop'); }
    },
  }));
  mock.module('./qa-final-scenarios.mjs', () => ({
    dashboard:async ()=> 'http://owned.invalid',
    chat:async ()=>{ assert(calls.includes('armed')); calls.push('posted'); },
    reply:()=>null,
  }));
  try {
    const { extra } = await import('./qa-last-readonly-extra.mjs');
    await assert.rejects(extra(29,'unused'), error=>error===deadline);
    const evidence = saved.get('owner-session-status-deadline.json');
    assert.equal(evidence.replyDeadlineMissed,true);
    assert.deepEqual(evidence.sessions,sessions);
    assert.equal(evidence.messages[0].role,'user');
    assert.deepEqual(evidence.deliveries,[]);
    assert(calls.indexOf('owner-session-status-deadline.json') < calls.indexOf('stop'));
    assert(!saved.has('owner-session-status.json'));
  } finally { mock.restore(); rmSync(root,{recursive:true}); }
});
