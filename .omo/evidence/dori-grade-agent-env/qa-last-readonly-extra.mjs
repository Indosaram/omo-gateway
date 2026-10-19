import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { runQA, lead, bot, owner, block } from './qa-final-common.mjs';
import { dashboard, chat, reply } from './qa-final-scenarios.mjs';

function session(c) {
  c.db.query('INSERT INTO sessions(session_key,platform,channel_id,user_id,state_json) VALUES(?,?,?,?,?)').run(lead, 'discord', '1474231559895646240', owner, JSON.stringify({ system_prompt: 'Owned isolated QA. Work only in '+c.root+'. Never send, interrupt, adopt or release external sessions. Never contact Discord or change models. Use omo-gateway on PATH for local sessions list/read and cron health --json. No external writes.', enabled_toolsets: ['terminal','file'] }));
}
function job(c, id, payload) {
  c.db.query('INSERT INTO cron_jobs(id,session_key,expression,payload_json,enabled,next_run_at) VALUES(?,?,?,?,1,?)').run(id, lead, 'interval:24h', JSON.stringify(payload), '2099-01-01T00:00:00Z');
}
export async function extra(n, release) {
  return runQA(n, release, { name: 'task'+n+'-final', model: true }, async c => {
    session(c);
    if (n === 29) {
      const profilePath = '/Users/indo/code/project/omon-gateway-wt/dori-work/bot_profiles/omomeow.json';
      const profile = JSON.parse(readFileSync(profilePath, 'utf8'));
      const state = JSON.parse(c.db.query('SELECT state_json FROM sessions WHERE session_key=?').get(lead).state_json);
      state.system_prompt = profile.system_prompt + '\n' + state.system_prompt;
      c.db.query('UPDATE sessions SET state_json=? WHERE session_key=?').run(JSON.stringify(state), lead);
      c.save('measurement-profile.json', {path:profilePath,system_prompt:profile.system_prompt,modelApplied:false});
    }
    if (n === 28 || n === 29) {
      const wrapper = '#!/bin/sh\ncase "$1:$2" in thread:list|thread:read) exec /usr/bin/env HOME=/Users/indo /Users/indo/.bun/bin/omo "$@" ;; *) exit 64 ;; esac\n';
      writeFileSync(c.root+'/bin/omo', wrapper, {mode:0o700});
      c.receipt.readonlyOmoWrapper = {path:c.root+'/bin/omo', content:wrapper, allowed:['thread list','thread read'], rejectedExit:64, gatewayHome:c.env.HOME, readonlyThreadHome:'/Users/indo'};
      c.save('readonly-omo-wrapper.json', c.receipt.readonlyOmoWrapper);
    }
    mkdirSync(c.env.HOME+'/.omon', {recursive:true});
    writeFileSync(c.env.HOME+'/.omon/session-hosts.json', '[]');
    if (n === 28) {
      const listed = JSON.parse(await c.cmd([c.binary,'sessions','list','--json']));
      c.save('local-session-list.json',listed);
      const local = listed.sessions.find(x=>x.machine === 'local');
      if (local) c.save('local-session-read.json', JSON.parse(await c.cmd([c.binary,'sessions','read','local:'+local.session_id,'--limit','2'])));
      else block(c,'local session read','No local session exists in the private HOME; list exercised, read not fabricated.');
      block(c,'remote mutation excluded','Owner explicitly skipped real-session remote send/interrupt/adopt.');
      return;
    }
    if (n === 29) {
      const origin = await dashboard(c);
      const observationMs = process.env.QA29_OBSERVATION_MS === '180000' ? 180000 : 30000;
      const response = c.observe(()=>reply(c,lead),'session status delivery observation',observationMs); response.catch(()=>{});
      const started = performance.now(), postAt = new Date().toISOString();
      await chat(c,origin,lead,'지금 실행 중인 세션 상태 알려줘. 로컬 세션만 조회하고 목록을 보여줘. send/interrupt/adopt는 하지 마.');
      let messages;
      try { messages=await response; }
      catch (error) {
        // A final-reply deadline is not proof that no lead turn ran. Capture
        // the binding and transcript before runQA's finally stops the gateway.
        c.save('owner-session-status-deadline.json', {
          elapsedMs:Math.round(performance.now()-started),
          postAt, observationMs, replyDeadlineMs:30000,
          replyDeadlineMissed:true,
          sessions:c.snapshot('SELECT session_key,state_json,updated_at FROM sessions WHERE session_key=?',lead),
          messages:c.snapshot('SELECT role,content,sequence,created_at FROM messages WHERE session_key=? ORDER BY sequence',lead),
          deliveries:c.snapshot('SELECT * FROM delivery_obligations WHERE session_key=?',lead),
          gatewayLog:c.logs(),
          error:String(error),
        });
        throw error;
      }
      const elapsedMs = Math.round(performance.now()-started);
      c.save('owner-session-status.json',{elapsedMs,postAt,observationMs,replyDeadlineMs:30000,verdict:elapsedMs<=30000?'PASS':'FAIL',messages,
        sessions:c.snapshot('SELECT session_key,state_json,updated_at FROM sessions WHERE session_key=?',lead),
        rows:c.snapshot('SELECT role,content,sequence,created_at FROM messages WHERE session_key=? ORDER BY sequence',lead),
        deliveries:c.snapshot('SELECT * FROM delivery_obligations WHERE session_key=?',lead)});
      block(c,'session listing meaning','NEEDS-REVIEW: raw delivered response and tool evidence require human judgement of session listing.');
      return;
    }
    if (n === 31) {
      const repo=c.root+'/workspace/cron-fixture'; mkdirSync(repo);
      await c.cmd(['/usr/bin/git','init',repo]);
      writeFileSync(repo+'/job.py','#!/usr/bin/env python3\nprint("count=" + str(total))\n');
      await c.cmd(['/usr/bin/git','-C',repo,'add','job.py']);
      await c.cmd(['/usr/bin/git','-C',repo,'-c','user.name=Owned QA','-c','user.email=qa@example.invalid','commit','-m','Failing scratch cron fixture']);
      job(c,'qa31-scratch',{name:'Owned QA broken counter',script:'/usr/bin/python3 '+repo+'/job.py',cwd:repo,expect:{output_regex:'count=7'},deliver:'local'});
      const opened=c.observe(()=>{const p=c.db.query("SELECT * FROM cron_problems WHERE job_id='qa31-scratch' AND lane_key IS NOT NULL").get();return p&&c.db.query('SELECT * FROM lanes WHERE key=?').get(p.lane_key)?p:null;},'scratch cron remediation lane',360000);opened.catch(()=>{});
      const first=JSON.parse(await c.cmd([c.binary,'cron','run-now','qa31-scratch','--wait','--json'],1)); c.save('initial-cron-failure.json',first);
      const problem=await opened; c.save('remediation-lane.json',problem);
      const fixed=await c.observe(()=>{const p=c.db.query('SELECT * FROM cron_problems WHERE id=?').get(problem.id);return p.status==='fixed'?p:null;},'scratch cron verified single rerun after settlement',900000);
      const runs=c.snapshot("SELECT * FROM cron_runs WHERE job_id='qa31-scratch' ORDER BY started_at");
      assert.equal(runs.length,2); assert.equal(runs[0].status,'failed'); assert.equal(runs[1].status,'succeeded');
      assert.equal(JSON.parse(fixed.detail).reruns,1);
      const outputs=c.snapshot("SELECT * FROM cron_outputs WHERE job_id='qa31-scratch'"); assert(outputs.some(x=>x.run_id===runs[1].run_id&&/count=7/.test(x.output)));
      c.save('single-verified-rerun.json',{fixed,runs,outputs,outcomes:c.snapshot('SELECT * FROM cron_remediation_outcomes')});
      return;
    }
    if (n === 32) {
      job(c,'qa32-ok',{script:'printf "count=7\\n"',deliver:'local'});
      await c.cmd([c.binary,'cron','run-now','qa32-ok','--wait','--json']);
      job(c,'qa32-never',{script:'printf "not run\\n"',deliver:'local'});
      const report=JSON.parse(await c.cmd([c.binary,'cron','health','--json']));
      assert.equal(report.ok_jobs,1); assert(report.jobs.find(x=>x.job_id==='qa32-ok').last_success_kst.endsWith('KST')); assert.equal(report.jobs.find(x=>x.job_id==='qa32-never').last_status,null);
      c.save('cron-health-report.json',report);
      const origin=await dashboard(c), boundary=c.db.query('SELECT COALESCE(MAX(sequence),0) n FROM messages WHERE session_key=?').get(lead).n;
      const response=c.observe(()=>reply(c,lead,boundary),'owner cron health delivered reply',240000);response.catch(()=>{});
      await chat(c,origin,lead,'크론 상태 알려줘. cron health --json으로 조회해서 한 메시지로 답해줘.');
      c.save('cron-health-owner-reply.json',await response);
      block(c,'cron health prose meaning','NEEDS-REVIEW: machine JSON is asserted; human must inspect actual one-message owner summary.');
    }
  });
}

