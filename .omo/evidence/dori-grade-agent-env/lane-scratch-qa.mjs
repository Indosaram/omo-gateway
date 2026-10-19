#!/usr/bin/env bun
// Source preparation only. See lane-scratch-qa.md before coordinator-released execution.
import { Database } from 'bun:sqlite';
import { mkdtempSync, mkdirSync, realpathSync, symlinkSync, watch, existsSync, readFileSync, writeFileSync, statSync, accessSync, constants } from 'node:fs';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { EventEmitter } from 'node:events';
import assert from 'node:assert/strict';
import { existingQaSink } from './lane-scratch-qa.sink.mjs';
const binary = '/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway';
const sourceRoot = '/Users/indo/code/project/omon-gateway-wt/dori-work';
const [mode, inputPath, thirdArg, fourthArg, ...extraArgs] = process.argv.slice(2);
const hash = value => createHash('sha256').update(value).digest('hex');
const fileHash = path => hash(readFileSync(path));
const absolute = path => typeof path === 'string' && path.startsWith('/');
assert(extraArgs.length === 0 && ['prepare','validate','execute','execute-8-9','execute-9-recovery'].includes(mode) && absolute(inputPath),
  'Usage: prepare <absolute-source-manifest> <absolute-typesafe-judge> <binary-sha256> | validate/execute/execute-8-9 <absolute-prepared.json> <absolute-release.json>');
function checkManifest(path, expected) {
  const bytes=readFileSync(path);if(expected)assert.equal(hash(bytes),expected,'Source manifest hash mismatch');
  const lines=bytes.toString('utf8').trim().split('\n'), seen=new Set();
  assert(lines.length>0,'Empty source manifest');
  for(const line of lines){
    const match=/^([a-f0-9]{64})  (.+)$/.exec(line);assert(match,'Invalid source manifest entry');
    const relative=match[2].replace(/^\.\//,'');
    assert(relative && !relative.startsWith('/') && !relative.split('/').some(part=>part==='..'||part===''), 'Unsafe source manifest path');
    assert(!seen.has(relative),'Duplicate source manifest path');seen.add(relative);
    assert.equal(fileHash(join(sourceRoot,relative)),match[1],`Source changed: ${relative}`);
  }
  for(const path of ['Cargo.toml','Cargo.lock','src/entry.rs','src/main.rs','src/lanes/runner.rs','src/agent/omo_protocol.rs'])assert(seen.has(path),`Missing manifest input ${path}`);
  return hash(bytes);
}
function checkJudge(path) {
  assert(absolute(path),'Judge path must be absolute');accessSync(path,constants.X_OK);
  const bytes=readFileSync(path), magic=bytes.subarray(0,8);
  // This harness uses the inspected native arm64 judge, not a PATH-dependent script interpreter.
  assert(magic.equals(Buffer.from([0xcf,0xfa,0xed,0xfe,0x0c,0,0,1])),'Judge must be the inspected native arm64 Mach-O executable');
  return {path:realpathSync(path),sha256:hash(bytes)};
}
let prepared;
if(mode==='prepare'){
  assert(absolute(thirdArg) && /^[a-f0-9]{64}$/.test(fourthArg??''),'Prepare requires absolute judge and pinned binary SHA-256');
  assert.equal(fileHash(binary),fourthArg,'Binary hash mismatch');
  const manifestSha256=checkManifest(inputPath),judge=checkJudge(thirdArg);
  prepared={schema:'omon-lane-qa-prepared-v1',root:realpathSync(mkdtempSync('/private/tmp/omon-lane-scratch-qa-')),
    binary,binarySha256:fourthArg,sourceRoot,manifestSha256,judge,createdAt:new Date().toISOString()};
}else{
  assert(absolute(thirdArg) && fourthArg===undefined,'Validate/execute requires prepared and structured release receipts');
  prepared=JSON.parse(readFileSync(inputPath,'utf8'));
  assert.equal(prepared.schema,'omon-lane-qa-prepared-v1','Invalid prepared receipt');
}
const root=prepared.root;
assert(typeof root==='string' && /^\/private\/tmp\/omon-lane-scratch-qa-[^/]+$/.test(root),'Invalid scratch root');
assert.equal(realpathSync(root),root,'Scratch root must be canonical');
assert.equal(statSync(root).mode&0o777,0o700,'Scratch root must be private');
const home = join(root,'home'), repo = join(root,'qa-repo'), bin = join(root,'bin');
const dbPath = join(root,'qa.db');
const env = {HOME:home, USER:'lane-qa', LOGNAME:'lane-qa', TMPDIR:root,
  PATH:`${bin}:/usr/bin:/bin:/usr/sbin:/sbin`, XDG_CONFIG_HOME:join(home,'.config'),
  XDG_DATA_HOME:join(home,'.local/share'), XDG_CACHE_HOME:join(home,'.cache'),
  DATABASE_URL:`sqlite://${dbPath}`, OMON_DISCORD_ENABLED:'false', OMON_LANES_ENABLED:'true',
  OMON_KIBITZER_ENABLED:'false', DASHBOARD_ENABLED:'false', OMON_WORKSPACE_ROOT:join(root,'workspaces'),
  OMON_LANE_KEYWORDS:'', OMON_OMO_APPSERVER_URL:'ws://127.0.0.1:19742',
  // Discord-free startup bypasses the collector; these supported settings are not a kill switch.
  OMON_PIRATETALK_URL:'http://127.0.0.1:1', OMON_INBOX_KAKAO_ROOMS:'', RUST_LOG:'info', NO_COLOR:'1'};
const preparedPath=join(root,'prepared.json'),manifestPath=join(root,'source-manifest.sha256');
for (const name of ['zele','imsg','omo']) assert(!Bun.which(name,{PATH:env.PATH}),`${name} must not be reachable`);
if(mode==='prepare'){
  for(const path of [home,repo,bin])mkdirSync(path,{mode:0o700});
  writeFileSync(join(root,'.env'),'',{mode:0o600,flag:'wx'});
  symlinkSync(binary,join(bin,'omo-gateway'));symlinkSync(prepared.judge.path,join(bin,'typesafe-judge'));
  writeFileSync(manifestPath,readFileSync(inputPath),{mode:0o600,flag:'wx'});
  prepared={...prepared,dbPath,databaseUrl:env.DATABASE_URL,repo,env,manifestPath};
  writeFileSync(preparedPath,JSON.stringify(prepared,null,2),{mode:0o600,flag:'wx'});
  console.log(JSON.stringify({result:'PREPARED_RUNTIME_UNRUN',preparedPath,root,dbPath,binarySha256:prepared.binarySha256,manifestSha256:prepared.manifestSha256}));
  process.exit(0);
}
assert.equal(realpathSync(inputPath),preparedPath,'Must reuse exact prepared receipt');
assert.equal(prepared.binary,binary);assert.equal(prepared.sourceRoot,sourceRoot);
assert.equal(prepared.dbPath,dbPath);assert.equal(prepared.databaseUrl,env.DATABASE_URL);assert.equal(prepared.repo,repo);
assert.deepEqual(prepared.env,env,'Prepared environment changed');assert.equal(prepared.manifestPath,manifestPath);
assert.equal(readFileSync(join(root,'.env'),'utf8'),'','Scratch dotenv must remain empty');
for(const path of [home,repo,bin])assert.equal(realpathSync(path),path,'Scratch directories must not be redirected');
assert.equal(realpathSync(join(bin,'omo-gateway')),realpathSync(binary));
assert.deepEqual(checkJudge(join(bin,'typesafe-judge')),prepared.judge,'Judge identity changed');
assert.equal(fileHash(binary),prepared.binarySha256,'Binary hash mismatch');
checkManifest(manifestPath,prepared.manifestSha256);
if(mode!=='execute-9-recovery'){
for(const suffix of ['','-wal','-shm','-journal'])assert(!existsSync(dbPath+suffix),'Prepared root must have no database files');
assert(!existsSync(join(root,'execution-started.json')),'Prepared root already consumed');
}else assert(existsSync(join(root,'execution-started.json')),'Recovery requires consumed root');
const releaseReceipt=thirdArg, release=JSON.parse(readFileSync(releaseReceipt,'utf8'));
assert.equal(release.schema,'omon-lane-qa-release-v1','Invalid release receipt schema');
assert.equal(release.preparedSha256,fileHash(preparedPath),'Prepared receipt identity mismatch');
for(const key of ['root','dbPath','databaseUrl','binary','binarySha256','manifestSha256'])assert.equal(release[key],prepared[key],`Release ${key} mismatch`);
assert.equal(release.gate?.status,'released','Gate not released');
function checkEvidence(evidence, scratchOnly=false){
  assert(evidence && absolute(evidence.path) && /^[a-f0-9]{64}$/.test(evidence.sha256??''),'Missing hashed evidence');
  if(scratchOnly)assert(realpathSync(evidence.path).startsWith(`${root}/`),'Scope evidence must be in exact root');
  assert.equal(fileHash(evidence.path),evidence.sha256,'Evidence hash mismatch');
}
checkEvidence(release.gate.evidence);
const scope=release.backendScope;
if(scope?.kind==='local-authenticated-preflight-v1'){
  assert.equal(mode,'validate','Local preflight is only for dedicated task12 helper validation');
  assert.equal(scope.status,'observed');assert.equal(scope.purpose,'task12-question');
  for(const key of ['root','dbPath','binarySha256','manifestSha256'])assert.equal(scope[key],prepared[key]);
  assert.equal(scope.preparedSha256,fileHash(preparedPath));assert.equal(scope.cwd,repo);
  assert.equal(scope.endpoint,env.OMON_OMO_APPSERVER_URL);
  assert(Number.isFinite(Date.parse(scope.observedAt))&&Date.parse(scope.observedAt)>=Date.parse(prepared.createdAt));
  checkEvidence(scope.evidence,true);
  const artifact=JSON.parse(readFileSync(scope.evidence.path,'utf8'));
  assert.equal(artifact.schema,'omon-lane-local-preflight-v1');
  const {evidence,...fields}=scope;assert.deepEqual(artifact.preflight,fields);
  assert.equal(scope.localChild?.executable,binary);assert.deepEqual(scope.localChild?.argv,[binary,'--help']);
  assert.equal(scope.localChild?.cwd,repo);assert.deepEqual(scope.localChild?.explicitEnv,env);
  assert.equal(scope.localChild?.databaseUrl,env.DATABASE_URL);assert.equal(scope.localChild?.exitCode,0);
  assert.equal(scope.protocol?.authenticated,true);assert.equal(scope.protocol?.method,'initialize');
  assert.equal(typeof scope.protocol?.requestId,'string');assert(scope.protocol.requestId.length);
  assert.equal(typeof scope.protocol?.response?.userAgent,'string');
  assert.equal(scope.remoteEnvironmentObserved,false);assert.equal(scope.remoteTurnCreated,false);
  const rpc=artifact.receipts.find(r=>r.kind==='authenticated_initialize');
  assert.equal(rpc?.request?.id,scope.protocol.requestId);assert.equal(rpc?.response?.id,scope.protocol.requestId);
  assert.equal(rpc?.request?.method,'initialize');assert.deepEqual(rpc?.response?.result,scope.protocol.response);
  assert(!rpc?.response?.error);
  const child=artifact.receipts.find(r=>r.kind==='local_child');
  assert.equal(child?.exitCode,0);assert.equal(child?.stdoutSha256,scope.localChild.stdoutSha256);
  assert.equal(child?.stderrSha256,scope.localChild.stderrSha256);
  for(const digest of [child.stdoutSha256,child.stderrSha256])assert(/^[a-f0-9]{64}$/.test(digest));
}else{
assert(scope && scope.status==='verified','Missing verified backend scope probe');
assert.equal(scope.endpoint,env.OMON_OMO_APPSERVER_URL);assert.equal(scope.cwd,repo);
assert.deepEqual(scope.runtimeWorkspaceRoots,[repo]);assert.equal(scope.root,root);assert.equal(scope.dbPath,dbPath);
assert(typeof scope.threadId==='string' && scope.threadId.length>0,'Missing probe thread ID');
assert(Number.isFinite(Date.parse(scope.observedAt)) && Date.parse(scope.observedAt)>=Date.parse(prepared.createdAt),'Probe predates preparation');
checkEvidence(scope.evidence,true);
assert.equal(scope.controlChild?.executable,binary);assert.equal(scope.controlChild?.databaseUrl,env.DATABASE_URL);
assert.equal(scope.controlChild?.databaseUrlEqual,true);assert.equal(scope.controlChild?.envOverride,true);
assert.equal(scope.controlChild?.exitCode,0);assert.equal(scope.controlChild?.cwd,repo);
assert.equal(typeof scope.inheritedDatabaseUrlEqual,'boolean','Record inherited env equality separately; do not invent propagation');
const probe=JSON.parse(readFileSync(scope.evidence.path,'utf8'));
assert.equal(probe.schema,'omon-lane-qa-scope-probe-v1','Invalid scope probe artifact');
const {evidence:scopeEvidence,...scopeFields}=scope;
assert.deepEqual(probe.scope,scopeFields,'Release scope does not match recorded probe');
}
const expectedHash=prepared.binarySha256;
const preservedRun=join(sourceRoot,'.omo/evidence/dori-grade-agent-env',`task-8-9-live-${root.split('-').at(-1)}-${Date.now()}`);
if(mode==='validate'){
  console.log(JSON.stringify({result:'RELEASE_CONTRACT_VALID_RUNTIME_UNRUN',root,dbPath}));process.exit(0);
}
const requiredCredentials=['TYPESAFE_API_KEY','TYPESAFE_URL','TYPESAFE_MODEL',...(mode==='execute'?['DISCORD_BOT_TOKEN']:[])];
for (const key of requiredCredentials) assert(process.env[key], `Missing existing private ${key}`);
for (const key of [...requiredCredentials,'OMON_OMO_APPSERVER_AUTH_TOKEN','OMON_OMO_AUTH_TOKEN']) if (process.env[key]) env[key]=process.env[key];
for (const name of ['zele','imsg','omo']) assert(!Bun.which(name,{PATH:env.PATH}),`${name} must not be reachable`);
assert.equal(env.DATABASE_URL,`sqlite://${root}/qa.db`);
writeFileSync(join(root,mode==='execute-9-recovery'?'task9-recovery-started.json':'execution-started.json'),JSON.stringify({releaseSha256:fileHash(releaseReceipt),at:new Date().toISOString()}),{mode:0o600,flag:'wx'});
const records=[], children=new Set(), observers=new Set(), bus=new EventEmitter();
const childExits=new Map();
mkdirSync(preservedRun,{mode:0o700});
let gateway, gatewayDrain, db, writable, verified=false, binding, sink, failure, logText='';
const artifact=(name,value)=>{const path=join(root,name);writeFileSync(path,typeof value==='string'?value:JSON.stringify(value,null,2),{mode:0o600});return path;};
const record=value=>{records.push({...value,at:new Date().toISOString()});artifact('receipts.json',records);};
function spawn(argv, options={}) {
  console.log(JSON.stringify({surface:'CLI',argv,cwd:root,DATABASE_URL:env.DATABASE_URL}));
  const child=Bun.spawn(argv,{cwd:root,env,stdout:'pipe',stderr:'pipe',...options});
  children.add(child);
  childExits.set(child,child.exited.then(code=>{
    record({kind:'process_exit',pid:child.pid,argv,code});
    children.delete(child);return code;
  }));
  return child;
}
async function awaitGatewayReady(startup, child, drained) {
  let ready=false;
  await Promise.race([startup.then(value=>{ready=true;return value;}),childExits.get(child).then(async code=>{
    if(ready)return;
    await drained;
    record({kind:'gateway_startup_exit',pid:child.pid,code,logSha256:hash(logText)});
    throw new Error(`Gateway exited before readiness: ${code}`);
  })]);
}
async function command(argv, expected=0) {
  const child=spawn(argv);
  const [out,err,code]=await Promise.all([new Response(child.stdout).text(),new Response(child.stderr).text(),child.exited]);
  const id=`command-${records.length}`;
  artifact(`${id}.stdout`,out||'(empty)\n');artifact(`${id}.stderr`,err||'(empty)\n');
  record({id,argv,code,stdoutBytes:out.length,stderrBytes:err.length});assert.equal(code,expected,id);return out;
}
function query(sql,...args){assert(verified&&db,'Scratch DB not verified');return db.query(sql).all(...args);}
const row=()=>query("SELECT key,status,turns_run,omo_thread_id,model,last_turn_outcome,thread_channel_id,parent_channel_id FROM lanes WHERE key='qa-1'")[0];
const generation=()=>query("SELECT COALESCE(MAX(id),0) AS n FROM lane_history WHERE lane_key='qa-1' AND reason='reopened'")[0].n;
function rejectCompletedClaimError(r){
  if(r?.last_turn_outcome!=='completed'||r.status==='done_claimed')return;
  const rejected=query("SELECT body FROM lane_events WHERE lane_key='qa-1' AND kind='blocked' ORDER BY id DESC LIMIT 1")[0];
  if(rejected && /(?:exited with (?:code )?2|required|unexpected argument)/i.test(rejected.body))throw new Error(`Completed backend rejected claim: ${rejected.body}`);
}
async function lane(args,code=0){assert(verified);return command([binary,'lane',...args],code);}
function observe(label,predicate,ms){
  let watcher,walWatcher,timer,finished=false,resolve,reject;
  const promise=new Promise((a,b)=>{resolve=a;reject=b;});promise.catch(()=>{});
  const cancel=()=>end(new Error(`${label}: cancelled`));
  function end(error,value){if(finished)return;finished=true;watcher?.close();walWatcher?.close();clearTimeout(timer);bus.off('line',check);observers.delete(cancel);error?reject(error):resolve(value);}
  function check(){try{const value=predicate();if(value)end(null,value);}catch(error){end(error);}}
  watcher=watch(root,(_event,name)=>{if(name&&String(name).startsWith('qa.db')){if(!walWatcher&&existsSync(dbPath+'-wal'))walWatcher=watch(dbPath+'-wal',check);check();}});
  if(existsSync(dbPath+'-wal'))walWatcher=watch(dbPath+'-wal',check);
  bus.on('line',check);timer=setTimeout(()=>end(new Error(`${label}: deadline exceeded`)),ms);observers.add(cancel);check();return promise;
}
async function drain(stream,name){
  const decoder=new TextDecoder();let pending='';
  for await(const chunk of stream){pending+=decoder.decode(chunk,{stream:true});let index;
    while((index=pending.indexOf('\n'))!==-1){const line=pending.slice(0,index);pending=pending.slice(index+1);
      logText+=`${name}: ${line}\n`;artifact('gateway.private.log',logText);bus.emit('line',line);}}
  pending+=decoder.decode();
  if(pending){logText+=`${name}: ${pending}\n`;artifact('gateway.private.log',logText);bus.emit('line',pending);}
}
const start=(key,brief,done='file done.txt')=>['start',key,'--title','QA lane','--brief',brief,'--done',done,'--cwd',repo,'--lead','4:lane|-|4:lead|-|9:lane:lead'];
const claim=`Use eval to invoke this explicit control child, not shell env prefixes or inherited DB settings: const child=Bun.spawn(${JSON.stringify([binary,'lane','claim-done','qa-1','--evidence','done.txt contains qa complete; live file signal checked'])},{cwd:${JSON.stringify(repo)},env:{...process.env,DATABASE_URL:${JSON.stringify(env.DATABASE_URL)}},stdout:'pipe',stderr:'pipe'}); const result=await Promise.all([new Response(child.stdout).text(),new Response(child.stderr).text(),child.exited]); print(JSON.stringify({controlExecutable:${JSON.stringify(binary)},databaseUrlEqual:true,exitCode:result[2],stdout:result[0],stderr:result[1]})); End the turn after exit. Do not override model configuration.`;
const local='Isolated QA only: no delegation, persistent goals, commits, network writes, source edits or work outside this QA repo. Do not inspect credentials/home config.';
function restoreBinding(){
  if(!binding)return;
  assert(verified&&writable);
  const result=writable.query(`UPDATE lanes SET thread_channel_id=?,parent_channel_id=? WHERE key='qa-1'
    AND thread_channel_id=? AND parent_channel_id=?
    AND (SELECT COALESCE(MAX(id),0) FROM lane_history WHERE lane_key='qa-1' AND reason='reopened')=?`)
    .run(binding.thread,binding.parent,sink.thread,sink.parent,binding.generation);
  assert.equal(result.changes,1,'Scratch binding restoration CAS failed');
  record({kind:'scratch_binding_restored',key:'qa-1',rows:result.changes,generation:binding.generation});binding=undefined;
}
try{
  record({kind:'provenance',mode,binary,sha256:expectedHash,releaseReceipt,scratch:root,envNames:Object.keys(env).sort()});
  if(mode!=='execute-9-recovery')await command(['/usr/bin/git','-C',root,'init','qa-repo']);
  const ready=await command(['/usr/bin/curl','-i','--fail','--max-time','10','http://127.0.0.1:19742/readyz']);
  assert(/HTTP\/\S+ 200/.test(ready));
  const startup=observe('opened scratch database and lane service ready',()=>logText.includes('gateway database opened')&&logText.includes(dbPath)&&logText.includes('gateway lane service ready'),30000);
  gateway=spawn([binary,'run']);
  gatewayDrain=Promise.all([drain(gateway.stdout,'stdout'),drain(gateway.stderr,'stderr')]);
  gatewayDrain.catch(error=>record({kind:'log_error',message:error.message}));
  await awaitGatewayReady(startup,gateway,gatewayDrain);
  const descriptors=await command(['/usr/sbin/lsof','-a','-p',String(gateway.pid),'-Fn']);
  const paths=descriptors.split('\n').filter(s=>s.startsWith('n')).map(s=>s.slice(1));
  assert(paths.includes(dbPath));assert(!paths.some(p=>p.includes('omon_gateway.db')));
  db=new Database(dbPath,{readonly:true});
  const pragma=db.query('PRAGMA database_list').all();
  assert.equal(realpathSync(pragma.find(v=>v.name==='main').file),dbPath);
  verified=true;record({kind:'database_verified',pid:gateway.pid,path:dbPath,pragma,descriptorSha256:hash(descriptors)});
  let first;
  const brief=mode==='execute-9-recovery'?join(root,'brief8.md'):artifact('brief8.md',`${local}\nCreate done.txt containing qa complete and a newline. Emit [REPORT] qa-1 | milestone | done.txt created. ${claim}\n`);
  if(mode!=='execute-9-recovery'){
  const turn8=observe('8 turn file report claim',()=>{
    const r=row();rejectCompletedClaimError(r);return r?.status==='done_claimed'&&r.turns_run>=1&&r.omo_thread_id&&existsSync(join(repo,'done.txt'))&&query("SELECT id FROM lane_transcript WHERE lane_key='qa-1' AND role='agent' AND text LIKE '%[REPORT] qa-1 | milestone |%'").length&&r;
  },600000);
  await lane(start('qa-1',brief));first=await turn8;
  const file8=readFileSync(join(repo,'done.txt'),'utf8');
  artifact('task-8.json',{row:first,doneFile:{text:file8,sha256:hash(file8)},reports:query("SELECT * FROM lane_transcript WHERE lane_key='qa-1'")});
  record({scenario:'8-real',row:first});await lane(['log','qa-1']);
  await lane(start('qa-2',brief,'command ["true"]'),2);
  assert.equal(query("SELECT count(*) AS n FROM lanes WHERE key='qa-2'")[0].n,0);record({scenario:'8-invalid',exit:2,rows:0});
  await lane(['close','qa-1','--force','--note','forced: qa']);assert.equal(row().status,'closed');
  }else{
    first=row();assert.equal(first.status,'closed');assert.equal(first.turns_run,1);
    assert.equal(query("SELECT count(*) AS n FROM lane_history WHERE lane_key='qa-1' AND to_status='done_claimed'")[0].n,1);
    assert(query("SELECT id FROM lane_transcript WHERE lane_key='qa-1' AND role='agent' AND text LIKE '%[REPORT] qa-1 | milestone |%'").length);
    assert.equal(readFileSync(join(repo,'done.txt'),'utf8'),'qa complete\n');
    artifact('task-8-recovered.json',{row:first,history:query("SELECT * FROM lane_history WHERE lane_key='qa-1'"),reports:query("SELECT * FROM lane_transcript WHERE lane_key='qa-1'"),status:'real claim persisted; original observer timeout preserved'});
    await lane(start('qa-2',brief,'command ["true"]'),2);
    assert.equal(query("SELECT count(*) AS n FROM lanes WHERE key='qa-2'")[0].n,0);record({scenario:'8-invalid',exit:2,rows:0});
  }
  const before=readFileSync(join(repo,'done.txt'),'utf8');
  const floor=query("SELECT COALESCE(MAX(id),0) AS n FROM lane_transcript WHERE lane_key='qa-1'")[0].n;
  const turn9=observe('9 same thread append',()=>{const r=row();rejectCompletedClaimError(r);return r?.status==='done_claimed'&&r.turns_run>=1&&readFileSync(join(repo,'done.txt'),'utf8').startsWith(before)&&readFileSync(join(repo,'done.txt'),'utf8').length>before.length&&r;},600000);
  const ownerText=`add a line to done.txt, preserving existing content. Then claim again. ${claim}`;
  artifact('task-9-owner-text.txt',ownerText);
  await lane(['reopen','qa-1','--text',ownerText]);
  const reopened=await turn9;assert.equal(reopened.omo_thread_id,first.omo_thread_id);
  const continuation=query("SELECT id FROM lane_transcript WHERE lane_key='qa-1' AND role='agent' AND id>? AND text LIKE '%done.txt%'",floor);assert(continuation.length);
  const file9=readFileSync(join(repo,'done.txt'),'utf8');
  artifact('task-9.json',{ownerText,row:reopened,sameThread:reopened.omo_thread_id===first.omo_thread_id,before:{text:before,sha256:hash(before)},after:{text:file9,sha256:hash(file9)},history:query("SELECT * FROM lane_history WHERE lane_key='qa-1'"),transcript:query("SELECT * FROM lane_transcript WHERE lane_key='qa-1'")});
  record({scenario:'9-real',row:reopened,transcriptIds:continuation.map(v=>v.id)});const liveLog9=await lane(['log','qa-1']);artifact('task-9.log',liveLog9);
  await lane(['reopen','nope'],2);record({scenario:'9-nonexistent',exit:2});
  await lane(['close','qa-1','--force','--note','forced: qa']);
  if(mode==='execute'){
  const question=observe('12 question completed paused turn',()=>{
    const r=row(),events=query("SELECT id,kind FROM lane_events WHERE lane_key='qa-1' AND kind='question'");
    return r?.status==='blocked'&&r.turns_run===1&&r.last_turn_outcome==='completed'&&events.length===1&&{row:r,events};
  },120000);
  await lane(['reopen','qa-1','--text',`${local} Emit exactly [REPORT] qa-1 | question | pick A or B, then end the turn without tools or claim. Wait for the lead's answer.`]);
  const paused=await question;const gen=generation();record({scenario:'12-question',...paused,generation:gen});
  // Outcome and turns_run are committed together after the backend turn returns.
  const boundary=query("SELECT id FROM lane_transcript WHERE lane_key='qa-1' AND role='outcome' AND text='completed' ORDER BY id DESC LIMIT 1");assert.equal(boundary.length,1);
  sink=existingQaSink({token:env.DISCORD_BOT_TOKEN,env,cwd:root,artifact,record,spawn});
  await sink.prepare(); // Exact read-only destination GET + authorized lifecycle unarchive BEFORE binding.
  assert.equal(generation(),gen);assert.equal(row().status,'blocked');
  const columns=query('PRAGMA table_info(lanes)').map(c=>c.name);
  for(const column of ['key','status','thread_channel_id','parent_channel_id','turns_run','last_turn_outcome'])assert(columns.includes(column),`Missing actual schema column ${column}`);
  // This is the only writable database handle: never a URL/env lookup, never repository DB.
  assert.equal(realpathSync(dbPath),dbPath);assert(dbPath.startsWith(`${root}/`));
  writable=new Database(dbPath,{readwrite:true,create:false});
  assert.equal(realpathSync(writable.query('PRAGMA database_list').all().find(v=>v.name==='main').file),dbPath);
  const previous=row();assert.equal(previous.thread_channel_id,null);assert.equal(previous.parent_channel_id,null);
  const postFloor=query("SELECT COALESCE(MAX(id),0) AS n FROM lane_transcript WHERE lane_key='qa-1'")[0].n;
  const changed=writable.query(`UPDATE lanes SET thread_channel_id=?,parent_channel_id=? WHERE key='qa-1'
    AND status='blocked' AND turns_run=1 AND last_turn_outcome='completed'
    AND thread_channel_id IS NULL AND parent_channel_id IS NULL
    AND (SELECT COALESCE(MAX(id),0) FROM lane_history WHERE lane_key='qa-1' AND reason='reopened')=?`)
    .run(sink.thread,sink.parent,gen);
  assert.equal(changed.changes,1,'Blocked completed-turn binding CAS failed');
  binding={thread:previous.thread_channel_id,parent:previous.parent_channel_id,generation:gen};
  record({kind:'scratch_binding',rows:1,key:'qa-1',thread:sink.thread,parent:sink.parent,generation:gen,outcomeId:boundary[0].id});
  const milestone=`QA lane milestone ${createHash('sha256').update(root).digest('hex').slice(0,12)}`;
  const posts=()=>query("SELECT id,role,text FROM lane_transcript WHERE lane_key='qa-1' AND role='thread_post' AND id>? ORDER BY id",postFloor);
  const delivery=observe('12 one curated milestone delivered',()=>{
    const rows=posts();assert(rows.length<=1,'Unexpected additional curated post');if(!rows.length)return false;
    const post=JSON.parse(rows[0].text);assert.equal(post.thread,sink.thread);assert.equal(post.body,milestone);
    assert.notEqual(post.state,'uncertain','Uncertain send must not be retried');return post.state==='sent'&&rows;
  },120000);
  const answered=observe('12 object answer completed claim',()=>{const r=row();return r?.status==='done_claimed'&&r.turns_run===2&&r.last_turn_outcome==='completed'&&/^A$/m.test(readFileSync(join(repo,'done.txt'),'utf8'))&&r;},600000);
  await lane(['object','qa-1','--reason',`A. Append A on a new line in done.txt. Emit exactly one report line: [REPORT] qa-1 | milestone | ${milestone}. Do not repeat that report in later messages or emit any other report. Then claim and end the turn. ${claim}`]);
  const delivered=await delivery;await sink.readMilestoneReceipt(delivered,milestone);
  const final=await answered;assert.equal(final.omo_thread_id,first.omo_thread_id);assert.equal(posts().length,1);
  restoreBinding();await sink.restore();
  record({scenario:'12-object',row:final,questionEvents:query("SELECT id,kind FROM lane_events WHERE kind='question'")});await lane(['log','qa-1']);
  record({kind:'scope',deadErrorCase:'Unrun here; coordinated test_lane_events receipt required'});
  }
}catch(error){failure=error;record({kind:'failure',message:error.message});}
finally{
  for(const cancel of [...observers])cancel();
  if(verified&&gateway?.exitCode===null){
    for(const r of query("SELECT key FROM lanes WHERE status!='closed'")){
      try{await lane(['close',r.key,'--force','--note','forced: qa cleanup']);}catch(error){failure??=error;record({kind:'cleanup_failure',message:error.message});}
    }
  }
  // Stop only owned processes before restoring binding on any incomplete turn.
  for(const child of [...children]){
    child.kill('SIGTERM');const timeout=setTimeout(()=>child.kill('SIGKILL'),10000);await child.exited;clearTimeout(timeout);
    let absent=false;try{process.kill(child.pid,0);}catch(error){if(error.code==='ESRCH')absent=true;else throw error;}
    record({kind:'owned_process_cleanup',pid:child.pid,exit:await childExits.get(child),absent});if(!absent)failure??=new Error('Owned PID remains');
  }
  if(gateway){
    await gatewayDrain;
    let absent=false;try{process.kill(gateway.pid,0);}catch(error){if(error.code==='ESRCH')absent=true;else throw error;}
    record({kind:'gateway_final_exit',pid:gateway.pid,code:await childExits.get(gateway),absent});
    if(!absent)failure??=new Error('Owned gateway PID remains');
  }
  try{restoreBinding();}catch(error){failure??=error;record({kind:'binding_cleanup_failure',message:error.message});}
  try{await sink?.restore();}catch(error){failure??=error;record({kind:'archive_cleanup_failure',message:error.message});}
  writable?.close();db?.close();record({kind:'cleanup',retainedScratch:root,sharedDaemonUntouched:true});
  const {readdirSync,copyFileSync,chmodSync}=await import('node:fs');
  for(const name of readdirSync(root))if(statSync(join(root,name)).isFile()&&!/^qa\.db/.test(name)){
    const target=join(preservedRun,name);copyFileSync(join(root,name),target,constants.COPYFILE_EXCL);chmodSync(target,0o600);
  }
  writeFileSync(join(preservedRun,'done.txt'),existsSync(join(repo,'done.txt'))?readFileSync(join(repo,'done.txt')):'(not created)\n',{mode:0o600,flag:'wx'});
  console.log(JSON.stringify({result:failure?'FAILURE':'SCENARIOS_COMPLETED_NOT_FULL_PLAN_PASS',mode,artifacts:root}));process.exitCode=failure?1:0;
}
