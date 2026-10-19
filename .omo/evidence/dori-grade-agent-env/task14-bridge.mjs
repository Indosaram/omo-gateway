import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,realpathSync,readFileSync,writeFileSync,watch,symlinkSync,existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {EventEmitter} from 'node:events';
import {Database} from 'bun:sqlite';
const base='/Users/indo/code/project/omon-gateway-wt/dori-work';
const binary=base+'/target-w4/debug/omo-gateway';
const manifest=base+'/.omo/evidence/dori-grade-agent-env/duplicate-final-20261011T184722Z-recovery-post-manifest.sha256';
const lead='5:local|-|8:bridge14|-|5:owner',marker='BRIDGE14_ACTUAL_REPLY';
const hash=p=>createHash('sha256').update(readFileSync(p)).digest('hex');
function settled(s){
 const a=s.messages.filter(x=>x.role==='assistant');
 if(a.length!==1||!a[0].content.includes(marker)||!s.events.some(e=>e.kind==='question'&&e.delivery_reason==='lead_turn')||!s.messages.some(x=>x.role==='user'&&x.content.startsWith('[system] ')&&x.content.includes('qa-1 question:')))return null;
 if(!s.obligations.some(o=>o.state==='delivered'&&o.content===a[0].content)||!s.sessions.some(x=>x.session_key===lead&&x.resume_pending===0&&JSON.parse(x.state_json).metadata?.cron_agent_output===a[0].content)||!s.settlement.some(x=>x.output===a[0].content))return null;
 return s;
}
if(process.argv[2]==='selftest'){
 assert.equal(settled({messages:[],events:[],obligations:[],sessions:[],settlement:[]}),null);
 const body=marker+' A preserve failure evidence and durable settlement';
 const s={messages:[{role:'assistant',content:body},{role:'user',content:'[system] qa-1 question: pick A or B'}],events:[{kind:'question',delivery_reason:'lead_turn'}],obligations:[{state:'delivered',content:body}],sessions:[{session_key:lead,resume_pending:0,state_json:JSON.stringify({metadata:{cron_agent_output:body}})}],settlement:[{output:body}]};
 assert.equal(settled(s),s);assert.equal(settled({...s,settlement:[]}),null);assert.equal(settled({...s,events:[{kind:'question',delivery_reason:'jev_skip'}]}),null);assert.equal(settled({...s,obligations:[{state:'delivered',content:'Done'}]}),null);console.log('BRIDGE_SELFTEST pass');process.exit(0);
}
assert.equal(process.argv[2],'execute');
assert.equal(hash(binary),'f81c077ee6a1252d647b7fc8f45758f8499b687dc9d2bb84d57bb3b512f15912');
assert.equal(hash(manifest),'a9570ea35fb0d696bffd5f7c5154f5f01a3663fd6c59ff26f0cd0acbedb3d06d');
const token=readFileSync('/Users/indo/.omo/agent/app-server/ws-token','utf8').trim();
const privateEnv={};
if(process.argv[3])for(const line of readFileSync(process.argv[3],'utf8').split('\n')){const m=/^(?:export )?(TYPESAFE_API_KEY|TYPESAFE_URL|TYPESAFE_MODEL)=(.*)$/.exec(line);if(m)privateEnv[m[1]]=m[2].replace(/^(['"])(.*)\1$/,'$2');}
for(const k of ['TYPESAFE_API_KEY','TYPESAFE_URL','TYPESAFE_MODEL']){privateEnv[k]??=process.env[k];assert(privateEnv[k],`Missing private ${k}`);}
await new Promise((resolve,reject)=>{const ws=new WebSocket('ws://127.0.0.1:19742',{headers:{Authorization:`Bearer ${token}`}});const t=setTimeout(()=>{ws.close();reject(new Error('initialize deadline'));},15000);ws.onopen=()=>ws.send(JSON.stringify({id:1,method:'initialize',params:{clientInfo:{name:'bridge14',version:'1'}}}));ws.onmessage=e=>{const v=JSON.parse(e.data);if(v.id===1){clearTimeout(t);ws.close();v.error?reject(new Error('initialize rejected')):resolve();}};ws.onerror=()=>{clearTimeout(t);reject(new Error('initialize transport'));};});
const root=realpathSync(mkdtempSync('/private/tmp/omon-task14-bridge-')),dbPath=root+'/bridge.db';
for(const dir of ['home','bin','workspace'])mkdirSync(root+'/'+dir,{mode:0o700});
symlinkSync('/Users/indo/.cargo/bin/typesafe-judge',root+'/bin/typesafe-judge');
const env={HOME:root+'/home',USER:process.env.USER,PATH:root+'/bin:/usr/bin:/bin:/usr/sbin:/sbin',TMPDIR:root,DATABASE_URL:'sqlite://'+dbPath,OMON_DISCORD_ENABLED:'false',OMON_LANES_ENABLED:'true',OMON_KIBITZER_ENABLED:'false',DASHBOARD_ENABLED:'false',OMON_WORKSPACE_ROOT:root+'/workspace',OMON_OMO_APPSERVER_URL:'ws://127.0.0.1:19742',OMON_OMO_APPSERVER_AUTH_TOKEN:token,RUST_LOG:'info',NO_COLOR:'1',...privateEnv};
for(const [k,v]of Object.entries(process.env))if(/^(DEFAULT_MODEL|OMON_DEFAULT_MODEL|OMON_MODEL_.*|OMON_OMO_FALLBACK_MODEL|FALLBACK_MODEL|DISCORD_PROFILE_ROUTES|DISCORD_CHANNEL_PROMPTS)$/.test(k))env[k]=v;
const r={root,dbPath,lead,binarySha256:hash(binary),manifestSha256:hash(manifest),helperSha256:hash(import.meta.path),startedAt:new Date().toISOString(),commands:[]};
const save=(name,obj)=>writeFileSync(root+'/'+name,JSON.stringify(obj,null,2),{mode:0o600});save('descriptor.json',{...r,envKeys:Object.keys(env),authPresent:true});
const bus=new EventEmitter();let logs='',db,gateway,wal,pumps;const dirwatch=watch(root,()=>bus.emit('change'));
const redact=text=>[token,privateEnv.TYPESAFE_API_KEY].reduce((s,k)=>s.split(k).join('[redacted]'),text);
async function command(argv){const c=Bun.spawn(argv,{cwd:root,env,stdout:'pipe',stderr:'pipe'});let t;try{const [out,err,code]=await Promise.race([Promise.all([new Response(c.stdout).text(),new Response(c.stderr).text(),c.exited]),new Promise((_,rej)=>{t=setTimeout(()=>{c.kill('SIGKILL');rej(new Error('CLI deadline'));},30000);})]);r.commands.push({argv,code,out:redact(out),err:redact(err)});assert.equal(code,0);return out;}finally{clearTimeout(t);}}
function observe(check,label,ms){return new Promise((resolve,reject)=>{let t;const done=()=>{clearTimeout(t);bus.off('change',inspect);};const inspect=()=>{try{const v=check();if(v!==null&&v!==undefined&&v!==false){done();resolve(v);}}catch(e){done();reject(e);}};bus.on('change',inspect);t=setTimeout(()=>{inspect();done();reject(new Error(label+' deadline; authority queried'));},ms);inspect();});}
async function pump(stream){const d=new TextDecoder();for await(const b of stream){logs+=redact(d.decode(b,{stream:true}));writeFileSync(root+'/bridge.log',logs,{mode:0o600});bus.emit('change');}}
const snapshot=()=>({lane:db.query("SELECT * FROM lanes WHERE key='qa-1'").get(),events:db.query('SELECT * FROM lane_events').all(),transcript:db.query('SELECT * FROM lane_transcript').all(),messages:db.query('SELECT role,content,platform_message_id FROM messages WHERE session_key=?').all(lead),obligations:db.query('SELECT * FROM delivery_obligations WHERE session_key=?').all(lead),sessions:db.query('SELECT session_key,state_json,resume_pending FROM sessions WHERE session_key=?').all(lead),settlement:db.query('SELECT * FROM qa_bridge_settlement').all()});
try{
 await command(['/usr/bin/git','init',root+'/repo']);
 const ready=observe(()=>logs.includes('headless gateway lead runtime ready')?true:null,'startup',30000);gateway=Bun.spawn([binary,'run'],{cwd:root,env,stdout:'pipe',stderr:'pipe'});r.pid=gateway.pid;pumps=Promise.all([pump(gateway.stdout),pump(gateway.stderr)]);gateway.exited.then(()=>bus.emit('change'));console.log('BRIDGE_ROOT '+root+' PID '+gateway.pid);await ready;
 db=new Database(dbPath);db.exec('PRAGMA busy_timeout=5000');wal=watch(dbPath+'-wal',()=>bus.emit('change'));
 const descriptors=await command(['/usr/sbin/lsof','-a','-p',String(gateway.pid),'-Fn']);const paths=descriptors.split('\n').filter(x=>x.startsWith('n')).map(x=>x.slice(1));assert(paths.includes(dbPath));assert(!paths.some(x=>x.endsWith('.db')&&x!==dbPath));r.descriptorPaths=paths;
 r.pragma=db.query('PRAGMA database_list').all();assert.equal(r.pragma[0].file,dbPath);assert.equal(db.query('SELECT COUNT(*) n FROM lane_events').get().n,0);r.journal=db.query('PRAGMA journal_mode').get();
 db.exec(`CREATE TABLE qa_bridge_settlement(session_key TEXT PRIMARY KEY,output TEXT); CREATE TRIGGER qa_bridge_done AFTER UPDATE OF state_json ON sessions WHEN NEW.session_key='${lead}' AND instr(json_extract(NEW.state_json,'$.metadata.cron_agent_output'),'${marker}')>0 BEGIN INSERT OR REPLACE INTO qa_bridge_settlement VALUES(NEW.session_key,json_extract(NEW.state_json,'$.metadata.cron_agent_output')); END;`);
 r.initial=snapshot();
 const question='Owner acceptance decision needed: pick A or B. A preserves failure evidence and requires durable actor settlement before success; B accepts admission alone as completion. This is an explicit isolated QA decision, not a production incident. Lead must choose A or B and explain its acceptance reasoning, include BRIDGE14_ACTUAL_REPLY, and use no tools or external actions.';
 writeFileSync(root+'/brief.md','Isolated QA. No tools, delegation, goals, credentials, Discord, network writes, files or claim commands. Emit exactly [REPORT] qa-1 | question | '+question+' and end the turn. Stay paused until the owner answers. Do not repeat or resume yourself.',{mode:0o600});
 const completed=observe(()=>{const s=snapshot();if(s.events.some(e=>e.delivery_reason==='jev_skip'))throw new Error('real Jev policy skip; bridge nonpass');if(logs.includes('agent runner failed'))throw new Error('terminal backend failure');return settled(s);},'bridge completion',240000);completed.catch(()=>{});
 await command([binary,'lane','start','qa-1','--title','Question bridge QA','--brief',root+'/brief.md','--done','file unused-bridge-signal','--cwd',root+'/repo','--lead',lead]);
 r.completed=await completed;assert.equal(r.completed.lane.status,'blocked');assert.equal(r.completed.lane.turns_run,1);assert(r.completed.transcript.some(x=>x.role==='agent'&&x.text.includes('[REPORT] qa-1 | question |')));assert(r.completed.transcript.some(x=>x.role==='outcome'&&x.text==='completed'));assert(logs.includes('lane event admitted to lead turn'));assert(logs.includes('headless local output accepted'));r.passBeforeCleanup=true;
}catch(e){r.error=redact(String(e.stack??e));if(db)try{r.recovery=snapshot();}catch(x){r.recoveryError=String(x);}}
finally{
 if(db&&gateway?.exitCode===null&&snapshot().lane&&snapshot().lane.status!=='closed')try{await command([binary,'lane','close','qa-1','--force','--note','forced: bridge QA cleanup']);}catch(e){r.cleanupError=String(e);}
 if(gateway?.exitCode===null){gateway.kill('SIGTERM');let t;await Promise.race([gateway.exited,new Promise(resolve=>{t=setTimeout(()=>{gateway.kill('SIGKILL');resolve();},40000);})]);clearTimeout(t);await gateway.exited;}if(pumps)await pumps;
 r.gatewayExit=gateway?.exitCode;if(gateway){try{process.kill(gateway.pid,0);r.pidAbsent=false;}catch{r.pidAbsent=true;}}
 if(db)r.final=snapshot();r.pass=Boolean(r.passBeforeCleanup&&settled(r.final)&&r.gatewayExit===0&&r.pidAbsent&&!r.cleanupError&&!logs.includes('agent turn interrupted'));
 r.postBinarySha256=hash(binary);r.postManifestSha256=hash(manifest);r.pass&&=r.postBinarySha256===r.binarySha256&&r.postManifestSha256===r.manifestSha256;
 const socket=logs.match(/socket=(\/tmp\/omon-lanes-[^\s]+\.sock)/)?.[1];r.socketAbsent=socket?!existsSync(socket):null;r.pass&&=r.socketAbsent===true;
 wal?.close();dirwatch.close();db?.close();r.exitCode=r.pass?0:1;r.finishedAt=new Date().toISOString();save('bridge.json',r);console.log('BRIDGE_RESULT '+JSON.stringify({root,exitCode:r.exitCode,gatewayExit:r.gatewayExit,pidAbsent:r.pidAbsent,error:r.error}));process.exitCode=r.exitCode;
}
