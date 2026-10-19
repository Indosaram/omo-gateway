#!/usr/bin/env bun
// Implementation only until parent releases the exact binary gate and fresh preparation.
import assert from 'node:assert/strict';
import { readFileSync, realpathSync, statSync, existsSync, writeFileSync } from 'node:fs';
import { createHash, randomBytes } from 'node:crypto';
import { join } from 'node:path';

const [preparedPath, helperSha256, ...extra] = process.argv.slice(2);
assert(extra.length === 0 && preparedPath?.startsWith('/') && /^[a-f0-9]{64}$/.test(helperSha256??''), 'Usage: lane-scope-probe.mjs <absolute-fresh-prepared.json> <scope-check.mjs-sha256>');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const fileHash = path => hash(readFileSync(path));
const preparedBytes = readFileSync(preparedPath);
const p = JSON.parse(preparedBytes);
assert.equal(p.schema, 'omon-lane-qa-prepared-v1');
assert(/^\/private\/tmp\/omon-lane-scratch-qa-[^/]+$/.test(p.root));
assert.equal(realpathSync(p.root), p.root);
assert.equal(realpathSync(preparedPath), join(p.root, 'prepared.json'));
assert.equal(statSync(p.root).mode & 0o777, 0o700);
assert.equal(p.repo, join(p.root, 'qa-repo'));
assert.equal(p.dbPath, join(p.root, 'qa.db'));
assert.equal(p.databaseUrl, `sqlite://${p.dbPath}`);
assert.equal(p.binary, '/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway');
assert.equal(p.sourceRoot, '/Users/indo/code/project/omon-gateway-wt/dori-work');
assert.equal(p.manifestPath, join(p.root, 'source-manifest.sha256'));
assert(Number.isFinite(Date.parse(p.createdAt)));
for (const path of [p.repo, join(p.root, 'home'), join(p.root, 'bin')]) {
  assert.equal(realpathSync(path), path);
  assert.equal(statSync(path).mode & 0o777, 0o700);
}
assert.equal(readFileSync(join(p.root, '.env'), 'utf8'), '');
assert.equal(fileHash(p.binary), p.binarySha256, 'Prepared binary changed');
assert.equal(realpathSync(join(p.root, 'bin/omo-gateway')), realpathSync(p.binary));
assert.equal(realpathSync(join(p.root, 'bin/typesafe-judge')), p.judge.path);
assert.equal(fileHash(p.judge.path), p.judge.sha256);
assert.equal(fileHash(p.manifestPath), p.manifestSha256);
const seen = new Set();
for (const line of readFileSync(p.manifestPath, 'utf8').trim().split('\n')) {
  const match = /^([a-f0-9]{64})  (.+)$/.exec(line); assert(match);
  const relative = match[2].replace(/^\.\//, '');
  assert(relative && !relative.startsWith('/') && !relative.split('/').some(s => s === '..' || s === ''));
  assert(!seen.has(relative)); seen.add(relative);
  assert.equal(fileHash(join(p.sourceRoot, relative)), match[1], `Source changed: ${relative}`);
}
for (const name of ['Cargo.toml', 'Cargo.lock', 'src/entry.rs', 'src/main.rs', 'src/lanes/runner.rs', 'src/agent/omo_protocol.rs']) assert(seen.has(name));
const expectedEnv = {
  HOME:join(p.root,'home'), USER:'lane-qa', LOGNAME:'lane-qa', TMPDIR:p.root,
  PATH:`${p.root}/bin:/usr/bin:/bin:/usr/sbin:/sbin`, XDG_CONFIG_HOME:`${p.root}/home/.config`,
  XDG_DATA_HOME:`${p.root}/home/.local/share`, XDG_CACHE_HOME:`${p.root}/home/.cache`,
  DATABASE_URL:p.databaseUrl, OMON_DISCORD_ENABLED:'false', OMON_LANES_ENABLED:'true',
  OMON_KIBITZER_ENABLED:'false', DASHBOARD_ENABLED:'false', OMON_WORKSPACE_ROOT:`${p.root}/workspaces`,
  OMON_LANE_KEYWORDS:'', OMON_OMO_APPSERVER_URL:'ws://127.0.0.1:19742',
  OMON_PIRATETALK_URL:'http://127.0.0.1:1', OMON_INBOX_KAKAO_ROOMS:'', RUST_LOG:'info', NO_COLOR:'1'
};
assert.deepEqual(p.env, expectedEnv);
function noDatabase() {
  for (const suffix of ['', '-wal', '-shm', '-journal']) assert(!existsSync(p.dbPath + suffix), 'Unexpected database file');
  assert(!existsSync(join(p.root, 'execution-started.json')), 'Root already consumed');
}
noDatabase();
const evidencePath = join(p.root, 'scope-probe.json');
const attemptId = `${Date.now()}-${randomBytes(6).toString('hex')}`;
const receiptPath = join(p.root, `scope-probe-receipts-${attemptId}.json`);
assert(!existsSync(evidencePath), 'Verified scope already exists');
const priorReceipt = join(p.root, 'scope-probe-receipts.json');
if (existsSync(priorReceipt)) {
  const prior = JSON.parse(readFileSync(priorReceipt, 'utf8'));
  assert.equal(prior.preparedSha256, hash(preparedBytes));
  assert(prior.receipts.every(entry => entry.kind === 'result' && entry.status === 'failed' && !entry.threadId && !entry.turnId), 'Prior runtime attempt cannot be replayed');
}
const nonce = randomBytes(24).toString('hex');
const marker = 'OMON_SCOPE_FILE=';
const helperPath = join(p.root,'scope-check.mjs');
assert.equal(realpathSync(helperPath),helperPath);
assert.equal(fileHash(helperPath),helperSha256,'Scope helper identity changed');
const commandArgv = ['/Users/indo/.bun/bin/bun','--no-env-file',helperPath];
const command = commandArgv.join(' ');
const nestedMarker = `OMON_BASH_RESULT_${nonce}=`;
const wrapperCode = `const result = await tool.bash(${JSON.stringify({command,timeout:20,run_in_background:false})}); print(${JSON.stringify(nestedMarker)} + JSON.stringify(result));`;
const prompt = `Owned one-step scope probe only. Bash is hidden and must be invoked via eval. Invoke eval exactly once with language js and the exact code below, unchanged. Await its foreground result, then end the turn. This exact eval wrapper is permitted; no other eval execution, delegation, lane/DB commands, files edits, network tools, Discord, credentials inspection, or other execution tool is permitted. Incidental discovery/read/bookkeeping is permitted but never evidence. Do not fabricate observations. The command reads cwd and one environment equality boolean, hashes an executable, then runs only --help with explicit scratch environment.\n\n${wrapperCode}`;
const endpoint = p.env.OMON_OMO_APPSERVER_URL;
// Match OmoConfig precedence, including present-but-empty primary masking fallback.
const token = (process.env.OMON_OMO_APPSERVER_AUTH_TOKEN ?? process.env.OMON_OMO_AUTH_TOKEN)?.trim();
assert(!token || !/[\r\n]/.test(token), 'Invalid private auth header');
const headers = token ? {Authorization:`Bearer ${token}`} : {};
const receipts = [];
const pending = new Map();
const startedItems = new Map();
// Pure discovery/bookkeeping never supplies observations. Names are exact,
// not a permissive suffix match that could admit an arbitrary execution tool.
const incidentalTools = new Set(['todo','tool_search','read','grep','glob','ls','ast_grep_search',
  'lsp_diagnostics','lsp_symbols','lsp_goto_definition','lsp_find_references',
  'lsp_prepare_rename','tool_schema']);
let ws, nextId = 1, threadId, turnId, terminal, observation, failure;
let submitted = false, closing = false, fatal;
let stage = 'validated_inputs';
let wake;
function sanitized(error) {
  let message = String(error?.message ?? error ?? 'Unknown error');
  if (token) message = message.replaceAll(token, '[REDACTED]');
  message = message.replace(/(?:Authorization|Bearer|headers?)\s*[:=]?[^\r\n]*/gi, '[REDACTED HEADER]');
  return {name:String(error?.name ?? 'Error'),code:typeof error?.code === 'string' || typeof error?.code === 'number' ? error.code : null,message:message.slice(0,1000)};
}
function signal() { wake?.(); wake = undefined; }
function fail(error) { fatal ??= error; signal(); for (const entry of pending.values()) entry.reject(error); }
function bounded(promise, ms, label) {
  let timer;
  return Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => {
    receipts.push({kind:'timeout',stage,label,ms}); reject(new Error(label));
  }, ms); })]).finally(() => clearTimeout(timer));
}
function rpc(method, params, ms = 15000) {
  const id = nextId++;
  const response = new Promise((resolve, reject) => pending.set(id, {resolve,reject}));
  receipts.push({kind:'rpc', id, method});
  ws.send(JSON.stringify({jsonrpc:'2.0', id, method, params}));
  return bounded(response, ms, `${method} deadline`).finally(() => pending.delete(id));
}
async function waitTerminal(ms) {
  const end = Date.now() + ms;
  while (!terminal) {
    if (fatal) throw fatal;
    const changed = new Promise(resolve => { wake = resolve; });
    await bounded(changed, Math.max(1,end-Date.now()), 'Owned turn completion deadline');
  }
  return terminal;
}
function onMessage(event) {
  try {
    assert(typeof event.data === 'string' && event.data.length <= 1024 * 1024, 'Invalid frame');
    const value = JSON.parse(event.data);
    if (value.method && value.id !== undefined) {
      ws.send(JSON.stringify({jsonrpc:'2.0',id:value.id,result:{allow:false,decision:'decline'}}));
      if (value.params?.threadId === threadId) fail(new Error('Unexpected approval or server tool request'));
      return;
    }
    if (!value.method && value.id !== undefined) {
      const entry = pending.get(value.id);
      if (entry) value.error ? entry.reject(new Error('RPC rejected; private server error omitted')) : entry.resolve(value.result);
      return;
    }
    const params = value.params;
    if (!submitted || params?.threadId !== threadId) return;
    if (value.method === 'turn/started') {
      assert(typeof params.turn?.id === 'string');
      if (turnId) assert.equal(params.turn.id, turnId);
      else turnId = params.turn.id;
      receipts.push({kind:'turn_started',threadId,turnId}); signal(); return;
    }
    const eventTurnId = params.turnId ?? params.turn?.id;
    if (!turnId || eventTurnId !== turnId) return;
    if (value.method === 'item/started' && params.item?.id) {
      startedItems.set(params.item.id, params.item);
      receipts.push({kind:'item_started',threadId,turnId,itemId:params.item.id,type:params.item.type,tool:params.item.tool});
    }
    if (value.method === 'item/completed') {
      const item = params.item;
      if (['agentMessage','userMessage','reasoning'].includes(item?.type)) return;
      const start = startedItems.get(item?.id);
      assert(start && start.type === item.type, 'Missing correlated item start');
      if (item.type === 'dynamicToolCall' && incidentalTools.has(item.tool)) {
        assert.equal(start.tool,item.tool);
        receipts.push({kind:'incidental_only',threadId,turnId,itemId:item.id,tool:item.tool,status:item.status}); return;
      }
      let output;
      let shellExitCode = item.exitCode ?? null;
      if (item.type === 'commandExecution') {
        assert.equal(item.command, command, 'Unexpected command'); assert.equal(start.command, command);
        assert.equal(item.cwd, p.repo, 'Tool cwd mismatch'); assert.equal(item.exitCode, 0);
        output = item.aggregatedOutput;
      } else {
        assert.equal(item.type, 'dynamicToolCall', 'Unexpected tool type');
        assert.equal(item.tool, 'eval', 'Unexpected dynamic execution tool');
        assert.equal(start.tool, item.tool); assert.deepEqual(start.arguments, item.arguments);
        assert.equal(item.arguments?.language, 'js');
        assert.equal(item.arguments?.code, wrapperCode, 'Exact serialized eval wrapper required');
        assert(!item.arguments.action || item.arguments.action === 'run');
        assert(!item.arguments.reset);
        assert.equal(item.success, true);
        assert(Array.isArray(item.contentItems) && item.contentItems.every(c => c.type === 'inputText'));
        let nestedText = item.contentItems.map(c => c.text).join('');
        if (nestedText.includes('[Output truncated:')) {
          const full = /\[Full output: ([^\]\n]+)\]/.exec(nestedText);
          assert(full, 'Truncated eval output missing full artifact');
          const sessions = realpathSync('/Users/indo/.omo/agent/sessions') + '/';
          const canonical = realpathSync(full[1]);
          assert(canonical.startsWith(sessions) && canonical.includes(threadId) && /-artifacts\/eval-[^/]+\.log$/.test(canonical), 'Unowned full output path');
          assert(statSync(canonical).size <= 1024 * 1024, 'Oversized full output');
          nestedText = readFileSync(canonical, 'utf8');
          receipts.push({kind:'full_output_followed',threadId,turnId,itemId:item.id,path:canonical,sha256:hash(nestedText)});
        }
        const nestedLines = nestedText.split('\n').filter(line => line.startsWith(nestedMarker));
        assert.equal(nestedLines.length,1,'Missing unique serialized bash result');
        const nested = JSON.parse(nestedLines[0].slice(nestedMarker.length));
        assert(!nested.hasError && !nested.details?.background, 'Nested bash failed or detached');
        assert.equal(nested.details?.status,'completed','Nested bash did not complete');
        shellExitCode = nested.details?.exitCode ?? nested.details?.code ?? null;
        if (shellExitCode !== null) assert.equal(shellExitCode,0);
        assert.equal(typeof nested.text,'string'); output = nested.text;
      }
      assert.equal(item.status, 'completed');
      assert(!observation, 'Duplicate tool result');
      const lines = (output ?? '').split('\n').filter(line => line.startsWith(marker));
      assert.equal(lines.length, 1, 'Missing unique machine tool output');
      observation = JSON.parse(lines[0].slice(marker.length));
      assert.equal(observation.controlChild?.exitCode, 0, 'Missing successful machine child exit');
      receipts.push({kind:'tool_result',threadId,turnId,itemId:item.id,type:item.type,exitCode:shellExitCode,childExitCode:observation.controlChild.exitCode,outputSha256:hash(output)});
    }
    if (value.method === 'turn/completed') { terminal = params.turn; signal(); }
  } catch (error) { fail(error); }
}
async function connect() {
  stage = 'websocket_construct';
  receipts.push({kind:'connection_attempt',stage,endpoint,authPresent:Boolean(token)});
  ws = new WebSocket(endpoint, {headers});
  ws.addEventListener('message', onMessage);
  ws.addEventListener('error', event => {
    const error = event.error ?? new Error(event.message || 'WebSocket error');
    receipts.push({kind:'websocket_error',stage,error:sanitized(error)}); fail(error);
  });
  ws.addEventListener('close', event => {
    receipts.push({kind:'websocket_close',stage,code:event.code,wasClean:event.wasClean,reason:sanitized({message:event.reason}).message});
    if (!closing) fail(new Error(`WebSocket closed code ${event.code}`));
  });
  stage = 'websocket_open';
  await bounded(new Promise((resolve,reject) => {
    ws.addEventListener('open',() => { receipts.push({kind:'websocket_open',stage}); resolve(); },{once:true});
    ws.addEventListener('error',event => reject(event.error ?? new Error(event.message || 'WebSocket connection failed')),{once:true});
  }),15000,'Connection deadline');
  stage = 'initialize';
  await rpc('initialize',{capabilities:{experimentalApi:true},clientInfo:{name:'omon-lane-scope-probe',title:'Owned scope probe',version:'1'}});
}
try {
  await connect();
  stage = 'thread_start';
  const started = await rpc('thread/start',{cwd:p.repo,runtimeWorkspaceRoots:[p.repo]});
  assert.equal(started.cwd,p.repo); assert.deepEqual(started.runtimeWorkspaceRoots,[p.repo]);
  threadId = started.thread?.id; assert(typeof threadId === 'string' && threadId.length);
  receipts.push({kind:'thread_started',threadId,cwd:started.cwd,runtimeWorkspaceRoots:started.runtimeWorkspaceRoots});
  submitted = true; // Message subscription is already active before turn/start.
  stage = 'turn_start';
  const accepted = await rpc('turn/start',{threadId,input:[{type:'text',text:prompt}]},45000);
  assert(typeof accepted.turn?.id === 'string');
  if (turnId) assert.equal(accepted.turn.id,turnId); else turnId = accepted.turn.id;
  stage = 'turn_observations';
  await waitTerminal(120000);
  assert.equal(terminal.status,'completed'); assert(observation, 'No observed machine tool result');
  assert.equal(observation.cwd,p.repo); assert.equal(typeof observation.inheritedDatabaseUrlEqual,'boolean');
  const child = observation.controlChild;
  assert.equal(child.executable,p.binary); assert.equal(child.cwd,p.repo); assert.equal(child.databaseUrl,p.databaseUrl);
  assert.equal(child.databaseUrlEqual,true); assert.equal(child.envOverride,true); assert.equal(child.exitCode,0);
  assert.equal(child.sha256,p.binarySha256); assert.deepEqual(child.argv,[p.binary,'--help']);
  assert(child.stdoutBytes > 0 && child.stderrBytes === 0);
  assert(!fatal); noDatabase(); assert.equal(fileHash(p.binary),p.binarySha256);
  assert.equal(fileHash(helperPath),helperSha256,'Scope helper changed during probe');
} catch (error) { failure = error; receipts.push({kind:'failure',stage,error:sanitized(error)}); }
finally {
  if (submitted && turnId && !terminal) {
    try {
      if (ws?.readyState !== WebSocket.OPEN) { fatal = undefined; closing = true; ws?.close(); closing = false; await connect(); }
      fatal = undefined;
      await rpc('turn/interrupt',{threadId,turnId},10000);
      await waitTerminal(10000);
      receipts.push({kind:'owned_turn_interrupt',threadId,turnId,status:terminal.status});
    } catch { receipts.push({kind:'owned_turn_interrupt_unconfirmed',threadId,turnId}); failure ??= new Error('Owned turn cancellation unconfirmed'); }
  }
  if (submitted && !turnId) { receipts.push({kind:'owned_turn_identity_unconfirmed',threadId}); failure ??= new Error('Submitted turn identity unconfirmed'); }
  closing = true; ws?.close();
  for (const entry of pending.values()) entry.reject(new Error('Probe client closed'));
  pending.clear(); wake = undefined;
  receipts.push({kind:'result',stage,status:failure?'failed':'observed',threadId,turnId,sharedDaemonUntouched:true});
  writeFileSync(receiptPath,JSON.stringify({schema:'omon-lane-qa-scope-receipts-v1',preparedSha256:hash(preparedBytes),receipts},null,2),{mode:0o600,flag:'wx'});
}
if (failure) { console.error('Scope probe failed; no verified artifact. Inspect private receipts for cancellation status.'); process.exitCode = 1; }
else {
  const scope = {status:'verified',endpoint,root:p.root,dbPath:p.dbPath,cwd:observation.cwd,
    runtimeWorkspaceRoots:[p.repo],threadId,observedAt:new Date().toISOString(),
    inheritedDatabaseUrlEqual:observation.inheritedDatabaseUrlEqual,controlChild:observation.controlChild};
  writeFileSync(evidencePath,JSON.stringify({schema:'omon-lane-qa-scope-probe-v1',scope,
    preparedSha256:hash(preparedBytes),helper:{path:helperPath,sha256:helperSha256,argv:commandArgv},receipt:{path:receiptPath,sha256:fileHash(receiptPath)}},null,2),{mode:0o600,flag:'wx'});
  console.log(JSON.stringify({result:'SCOPE_OBSERVED_RUNTIME_QA_UNRUN',path:evidencePath,sha256:fileHash(evidencePath),threadId,turnId}));
}
