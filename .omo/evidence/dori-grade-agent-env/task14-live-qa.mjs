import { Database } from 'bun:sqlite';
import { mkdtempSync, mkdirSync, watch, appendFileSync, writeFileSync, unlinkSync, realpathSync, readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { EventEmitter } from 'node:events';
import assert from 'node:assert/strict';
const binary='/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway';
const [releasePath,...extra]=process.argv.slice(2);
assert(releasePath?.startsWith('/')&&extra.length===0,'Usage: bun task14-live-qa.mjs /absolute/next67-release.json');
const release=JSON.parse(readFileSync(releasePath,'utf8'));
assert.equal(release.status,'released');
assert.equal(release.binary,binary);
assert(release.manifest?.startsWith('/')&&/^[a-f0-9]{64}$/.test(release.binarySha256)&&/^[a-f0-9]{64}$/.test(release.manifestSha256));
assert.notEqual(release.binarySha256,'884a16827325e3512ef7a4ce7441f4dd0aa1fa2e4b8954cf95feefaf42bc461e','Old binary predates durability fix');
const manifest=release.manifest;
const sha=async p=>createHash('sha256').update(new Uint8Array(await Bun.file(p).arrayBuffer())).digest('hex');
assert.equal(await sha(binary),release.binarySha256);
assert.equal(await sha(manifest),release.manifestSha256);
const credentialPath='/Users/indo/.omo/agent/app-server/ws-token';
const credential=readFileSync(credentialPath,'utf8').trim();
assert(credential&&!/[\r\n]/.test(credential));
await new Promise((resolve,reject)=>{
 const ws=new WebSocket('ws://127.0.0.1:19742',{headers:{Authorization:`Bearer ${credential}`}});
 const timer=setTimeout(()=>{ws.close();reject(new Error('authenticated initialize probe deadline'));},15000);
 ws.onopen=()=>ws.send(JSON.stringify({jsonrpc:'2.0',id:1,method:'initialize',params:{clientInfo:{name:'task14-auth-scope',version:'1'}}}));
 ws.onmessage=event=>{const value=JSON.parse(event.data);if(value.id===1){clearTimeout(timer);ws.close();if(value.error)reject(new Error('authenticated initialize rejected'));else resolve();}};
 ws.onerror=()=>{clearTimeout(timer);ws.close();reject(new Error('authenticated scope transport rejected'));};
});
console.log('TASK14_SCOPE authenticated_initialize_pass no_model_turn');
const root=realpathSync(mkdtempSync('/private/tmp/omon-task14-live-'));
const workspace=root+'/workspace', home=root+'/home', dbPath=root+'/task14.db', logPath=root+'/task14.log';
mkdirSync(workspace,{mode:0o700});mkdirSync(home,{mode:0o700});
const marker=workspace+'/.drain_request.json';
writeFileSync(marker,JSON.stringify({action:'drain',principal:'task14-owned-qa',requested_at:new Date().toISOString(),suppress_notification:true}),{mode:0o600});
const env={HOME:home,USER:process.env.USER,PATH:'/usr/bin:/bin:/usr/sbin:/sbin',TMPDIR:root,
 DATABASE_URL:'sqlite://'+dbPath,OMON_DISCORD_ENABLED:'false',OMON_LANES_ENABLED:'true',OMON_KIBITZER_ENABLED:'false',DASHBOARD_ENABLED:'false',OMON_WORKSPACE_ROOT:workspace,OMON_OMO_APPSERVER_URL:'ws://127.0.0.1:19742',RUST_LOG:'info',NO_COLOR:'1'};
for(const [k,v] of Object.entries(process.env))if(/^(DEFAULT_MODEL|OMON_DEFAULT_MODEL|OMON_MODEL_.*|OMON_OMO_FALLBACK_MODEL|FALLBACK_MODEL|DISCORD_PROFILE_ROUTES|DISCORD_CHANNEL_PROMPTS|OMON_OMO_APPSERVER_AUTH_TOKEN|OMON_OMO_AUTH_TOKEN)$/.test(k))env[k]=v;
env.OMON_OMO_APPSERVER_AUTH_TOKEN=credential;
const result={root,dbPath,binary,manifest,argv:[binary,'run'],env:Object.fromEntries(Object.entries(env).filter(([k])=>!k.includes('TOKEN'))),daemonEndpoint:env.OMON_OMO_APPSERVER_URL,startedAt:new Date().toISOString()};
writeFileSync(root+'/descriptor.json',JSON.stringify(result,null,2),{mode:0o600});
const signals=new EventEmitter();let logs='',db,child,observer,walObserver;
function bounded(check,label,ms=180000){return new Promise((resolve,reject)=>{
 const inspect=()=>{try{if(label==='real completed reply'&&logs.includes('agent runner failed'))throw new Error('terminal backend failure; see private task14.log');const value=check();if(value){finish();resolve(value);}}catch(e){finish();reject(e);}};
 const finish=()=>{clearTimeout(timer);signals.off('change',inspect);};
 const timer=setTimeout(()=>{try{const value=check();finish();if(value)resolve(value);else reject(new Error(label+' timed out; authoritative final query failed'));}catch(e){finish();reject(e);}},ms);
 signals.on('change',inspect);inspect();
});}
async function pump(stream){const decoder=new TextDecoder();for await(const bytes of stream){const text=decoder.decode(bytes,{stream:true});logs+=text;appendFileSync(logPath,text);signals.emit('change');}}
const ready=bounded(()=>logs.includes('headless gateway lead runtime ready'),'startup',30000);
observer=watch(root,()=>signals.emit('change'));
try{
 child=Bun.spawn([binary,'run'],{cwd:root,env,stdout:'pipe',stderr:'pipe'});result.pid=child.pid;
 const pumps=[pump(child.stdout),pump(child.stderr)];child.exited.then(()=>signals.emit('change'));
 console.log('TASK14_ROOT '+root+' PID '+child.pid);
 await ready;
 db=new Database(dbPath);db.exec('PRAGMA busy_timeout=5000');
 walObserver=watch(dbPath+'-wal',()=>signals.emit('change'));
 result.preconditions={databaseList:db.query('PRAGMA database_list').all(),journalMode:db.query('PRAGMA journal_mode').get(),events:db.query('SELECT COUNT(*) AS count FROM lane_events').get(),messages:db.query('SELECT COUNT(*) AS count FROM messages').get()};
 assert.equal(result.preconditions.databaseList[0].file,dbPath);assert.equal(result.preconditions.events.count,0);
 db.exec(`CREATE TABLE qa_actor_settlement (session_key TEXT PRIMARY KEY, output TEXT NOT NULL);
 CREATE TRIGGER qa_actor_settled AFTER UPDATE OF state_json ON sessions
 WHEN json_extract(NEW.state_json,'$.metadata.cron_agent_output') = 'TASK14_ACTUAL_REPLY'
 BEGIN INSERT OR REPLACE INTO qa_actor_settlement VALUES (NEW.session_key, json_extract(NEW.state_json,'$.metadata.cron_agent_output')); END;`);
 const lead='5:local|-|8:task14qa|-|5:owner';
 const snapshot=()=>({events:db.query('SELECT * FROM lane_events').all(),messages:db.query('SELECT role,content,sequence FROM messages ORDER BY sequence').all(),obligations:db.query('SELECT state,attempts,content FROM delivery_obligations').all(),sessions:db.query('SELECT session_key,state_json,resume_pending FROM sessions').all(),settlement:db.query('SELECT * FROM qa_actor_settlement').all()});
 result.initial=snapshot();
 const initialDrainLogs=logs.split('valid drain request detected by drain watcher').length;
 const blockedObserved=bounded(()=>logs.split('valid drain request detected by drain watcher').length>initialDrainLogs,'post-enqueue marker scan',15000);
 const complete=bounded(()=>{const s=snapshot();return s.messages.filter(m=>m.role==='assistant'&&m.content==='TASK14_ACTUAL_REPLY').length===1&&s.obligations.filter(o=>o.state==='delivered'&&o.content==='TASK14_ACTUAL_REPLY').length===1&&s.settlement.some(x=>x.session_key===lead)&&s.sessions.some(x=>x.session_key===lead&&x.resume_pending===0)&&logs.includes('headless local output accepted')?s:null;},'real completed reply',180000);
 complete.catch(()=>{});
 result.eventId=Number(db.query('INSERT INTO lane_events (lead_session_key,kind,body,created_at) VALUES (?,?,?,?) RETURNING id').get(lead,'done_claimed','Owned headless integration QA. Do not run tools, change files, or contact anyone. Reply with exactly TASK14_ACTUAL_REPLY. This is a local transcript delivery check.',new Date().toISOString()).id);
 await blockedObserved;result.blocked=snapshot();
 assert.equal(result.blocked.events[0].delivered_at,null);assert.equal(result.blocked.messages.length,0);assert.equal(child.exitCode,null);
 result.markerRemovedAt=new Date().toISOString();unlinkSync(marker);
 result.completed=await complete;
 assert.equal(result.completed.events[0].delivery_reason,'lead_turn');
 assert(result.completed.messages.some(m=>m.role==='user'&&m.content.startsWith('[system] ')));
 assert(logs.includes('lane event admitted to lead turn'));
 result.terminalProof={backend:'terminal result delivered with durable obligation',actor:'successful Completed branch flushed cron_agent_output into sessions; qa_actor_settlement trigger captured the write'};
 child.kill('SIGTERM');result.gatewayExit=await bounded(()=>child.exitCode!==null?{code:child.exitCode}:null,'joined gateway exit',40000).then(x=>x.code);
 await Promise.all(pumps);assert.equal(result.gatewayExit,0);assert(logs.includes('gateway lane service stopped'));
 result.final=snapshot();
 assert.equal(result.final.messages.filter(m=>m.role==='assistant'&&m.content==='TASK14_ACTUAL_REPLY').length,1);
 assert.equal(result.final.obligations.filter(o=>o.state==='delivered'&&o.content==='TASK14_ACTUAL_REPLY').length,1);
 assert.equal(result.final.sessions.find(x=>x.session_key===lead).resume_pending,0);
 assert(!logs.includes('agent turn interrupted'));
 result.criteriaPassed=true;
}catch(error){result.criteriaPassed=false;result.error=String(error.stack??error);if(db){try{result.authoritativeRecovery={events:db.query('SELECT * FROM lane_events').all(),messages:db.query('SELECT * FROM messages').all(),sessions:db.query('SELECT * FROM sessions').all()};}catch(e){result.recoveryError=String(e);}}}
finally{
 if(child&&child.exitCode===null){child.kill('SIGTERM');try{await bounded(()=>child.exitCode!==null?{code:child.exitCode}:null,'cleanup exit',40000);}catch{child.kill('SIGKILL');await child.exited;}}
 if(child){result.gatewayExit=child.exitCode;try{process.kill(child.pid,0);result.pidAbsent=false;}catch{result.pidAbsent=true;}}
 observer.close();walObserver?.close();if(db)db.close();result.finishedAt=new Date().toISOString();
 result.binarySha256=await sha(binary);result.manifestSha256=await sha(manifest);
 if(result.binarySha256!==release.binarySha256||result.manifestSha256!==release.manifestSha256){result.criteriaPassed=false;result.identityChanged=true;}
 result.exitCode=result.criteriaPassed&&result.gatewayExit===0&&result.pidAbsent?0:1;
 writeFileSync(root+'/task14.json',JSON.stringify(result,null,2),{mode:0o600});
 console.log('TASK14_RESULT '+JSON.stringify({root,exitCode:result.exitCode,gatewayExit:result.gatewayExit,pidAbsent:result.pidAbsent,error:result.error}));
 process.exitCode=result.exitCode;
}
