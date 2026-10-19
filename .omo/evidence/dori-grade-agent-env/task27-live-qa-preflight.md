# Todo27 actual gateway QA preflight

Status: READY FOR MINIMAL READ-ONLY QA; execution UNRUN. Product remains frozen while actual8/9 lifecycle monitoring continues. This document is the only edit. No help command, gateway list, SSH, network, model, Cargo, DB, service or git operation was run here.

## Exact binary and supported invocation

Binary: `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway`.
Required SHA-256: `60fce5ae8276944e9930bc2c41aa2ff039ad3daa29c05facd79e796cc621bc32`.
Build provenance: `fixture-repair-20261011T161103Z-{binary.sha256,handoff.md,post-manifest.sha256}` in this directory; manifest hash `45200128b3b5cd63fbc956b36e01daaa82f817b63607d176ef6751d89f486718`. Hashes are read receipts here, not a new binary verification. Recheck the binary hash in memory immediately before execution; mismatch stops QA without a rebuild.

The supported argv is exactly `[binary, "sessions", "list", "--json"]`. `src/entry.rs:99-110` loads dotenv then dispatches Sessions and exits before tracing/gateway startup. `run_sessions` uses only `SessionRegistry::from_environment`, listing and serialization. It never calls connect_profile_database, run_gateway, lane or dashboard paths. Thus owner DB is not opened on this branch. A fresh scratch cwd under `/private/tmp` prevents upward dotenv discovery of the repository's .env; explicit scratch DATABASE_URL is an additional guard, not permission to open a DB. Verify scratch DB/-wal/-shm/-journal remain absent afterward. No `--db` flag exists on sessions list.

Use a new private scratch root, distinct from actual8/9 roots. Retain owner HOME intentionally for existing local OMO session discovery and SSH authentication; replacing HOME with scratch could hide the sessions under test. Pass only the environment below, never spread the parent's environment. No provider tokens, model variables, Discord variables or SSH_AUTH_SOCK are copied. File-backed existing SSH authentication is attempted noninteractively; if it needs an agent or password, report that actual failure rather than adding credentials.

## Existing command/auth evidence

Read `/Users/indo/.bun/bin/omo`: an existing shell launcher execs `/Users/indo/.bun/bin/bun` and the installed omo-ai CLI. Filesystem resolution found that launcher and `/usr/bin/ssh`; neither was executed. Prior task-27.json/log report raw local listing exit0/9 records and separate raw omarchy SSH listing exit0/3 records. These are historical availability/auth receipts, not present reachability and not gateway execution. Do not reuse their counts as expected actual counts.

Current names-only seed file is `["omarchy"]`. Existing SSH config defines omarchy and no RemoteCommand for that alias, but it does contain RemoteForward directives. QA must disable those via an owned PATH shim; do not invoke the similarly named interactive tmux aliases. Do not copy identity paths, credentials, host addresses or session IDs into public receipts. A missing key, host-key entry, local omo state or remote omo command is an observed blocker, not grounds for provisioning anything.

## Owned setup and exact executable commands

At execution time create a new root with `mktemp -d /private/tmp/omon-task27-qa.XXXXXX`, chmod 700, and record its absolute path. Set `QA` to that returned path. All helper files below must be created with apply_patch, replacing `<QA>` with that exact owned root; do not write them with shell redirection. No helper belongs in product source or the actual8/9 scratch root.

Create `<QA>/ssh` with this exact content using apply_patch, then `chmod 700 "$QA/ssh"`:

```sh
#!/bin/sh
exec /usr/bin/ssh -o ClearAllForwardings=yes -o StrictHostKeyChecking=yes -o UpdateHostKeys=no -o ControlMaster=no -o ControlPath=none "$@"
```

This preserves the gateway's separate remote argv and existing alias/auth configuration, disables remote forwards and connection multiplexing, and refuses host-key creation/update. It is not a fake listing command.

Create `<QA>/run.mjs` with the following content using apply_patch. Replace only the marker `OWNED_QA_ROOT` with the actual root. This runner performs one actual happy gateway list, then one actual failure gateway list. It captures raw stdout/stderr only in memory, emits projected receipts, bounds each run, tracks owned child PIDs before they can disappear, and cleans up only those children.

```js
import { createHash } from 'node:crypto';
import { readFile, access } from 'node:fs/promises';
const qa = 'OWNED_QA_ROOT';
const binary = '/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway';
const expected = '60fce5ae8276944e9930bc2c41aa2ff039ad3daa29c05facd79e796cc621bc32';
if (createHash('sha256').update(await readFile(binary)).digest('hex') !== expected) throw Error('binary identity mismatch');
const base = {
  HOME: '/Users/indo',
  PATH: `${qa}:/Users/indo/.bun/bin:/usr/bin:/bin:/usr/sbin:/sbin`,
  TMPDIR: qa,
  DATABASE_URL: `sqlite://${qa}/forbidden.db`,
};
const owned = new Set();
const observedChildren = new Set();
const alive = pid => { try { process.kill(pid, 0); return true; } catch (e) { if (e.code === 'ESRCH') return false; throw e; } };
async function descendants(pid) {
  const p = Bun.spawn(['/bin/ps', '-axo', 'pid=,ppid='], { stdout: 'pipe', stderr: 'pipe' });
  const text = await new Response(p.stdout).text();
  await new Response(p.stderr).text();
  if (await p.exited !== 0) throw Error('child inspection failed');
  const rows = text.trim().split('\n').map(s => s.trim().split(/\s+/).map(Number));
  const tree = new Set([pid]);
  let changed;
  do { changed = false; for (const [child, parent] of rows) if (tree.has(parent) && !tree.has(child)) { tree.add(child); changed = true; } } while (changed);
  tree.delete(pid);
  return [...tree];
}
async function run(hosts, scenario) {
  const start = performance.now();
  const p = Bun.spawn([binary, 'sessions', 'list', '--json'], {
    cwd: qa, env: { ...base, OMON_SESSION_HOSTS: hosts }, stdin: 'ignore', stdout: 'pipe', stderr: 'pipe',
  });
  owned.add(p.pid);
  const out = new Response(p.stdout).text();
  const err = new Response(p.stderr).text();
  let timer;
  const inspect = setInterval(async () => {
    try { for (const child of await descendants(p.pid)) observedChildren.add(child); }
    catch { /* The final inspection below is authoritative. */ }
  }, 100);
  try {
    const code = await Promise.race([p.exited, new Promise((_, reject) => {
      timer = setTimeout(() => reject(Error('gateway exceeded 25 seconds')), 25000);
    })]);
    const elapsed_ms = performance.now() - start;
    const [stdout, stderr] = await Promise.all([out, err]);
    const data = JSON.parse(stdout);
    if (!Array.isArray(data.sessions) || !Array.isArray(data.errors)) throw Error('aggregate shape mismatch');
    for (const s of data.sessions) {
      if (Object.keys(s).sort().join(',') !== 'controls,cwd,machine,name,session_id,status,updated_at') throw Error('session projection mismatch');
    }
    const counts = Object.fromEntries(['local', 'omarchy'].map(m => [m, data.sessions.filter(s => s.machine === m).length]));
    const errors = data.errors.map(e => ({ machine: e.machine, code: e.code, message: e.message }));
    const happy = code === 0 && counts.local >= 1 && counts.omarchy >= 1;
    const isolated = scenario === 'unreachable' && code === 0 && counts.local >= 1 && elapsed_ms <= 25000 && errors.some(e => e.machine === '127.0.0.1' && e.code === 3);
    console.log(JSON.stringify({ scenario, binary_sha256: expected, argv: [binary, 'sessions', 'list', '--json'],
      cwd: qa, exit_code: code, elapsed_ms, counts,
      sessions: data.sessions.map(({ machine, name, cwd }) => ({ machine, name, cwd })), errors,
      stderr_bytes: Buffer.byteLength(stderr), passed: scenario === 'happy' ? happy : isolated }));
    if (!(scenario === 'happy' ? happy : isolated)) throw Error(`${scenario} acceptance failed`);
  } finally {
    clearTimeout(timer); clearInterval(inspect);
    if (alive(p.pid)) {
      for (const child of await descendants(p.pid)) {
        observedChildren.add(child);
        if (alive(child)) process.kill(child, 'SIGKILL');
      }
    }
    if (alive(p.pid)) p.kill();
    await p.exited;
    owned.delete(p.pid);
  }
}
let listener;
let failed = false;
try {
  await run('omarchy', 'happy');
  // An owned TCP endpoint accepts but never emits an SSH banner: no remote target is contacted.
  listener = Bun.listen({ hostname: '127.0.0.1', port: 0, socket: { data() {}, open() {}, close() {}, error() {} } });
  // Provision ssh-failure using apply_patch before execution, substituting the owned port is not needed:
  // the wrapper reads TASK27_STALL_PORT from this process's explicitly supplied environment.
  base.TASK27_STALL_PORT = String(listener.port);
  await run('omarchy,127.0.0.1', 'unreachable');
} catch (error) {
  failed = true;
  console.log(JSON.stringify({ qa_error: error.message }));
} finally {
  if (listener) listener.stop(true);
  for (const pid of owned) if (alive(pid)) process.kill(pid, 'SIGKILL');
  const remaining = [...observedChildren].filter(alive);
  const dbFiles = [];
  for (const suffix of ['', '-wal', '-shm', '-journal']) {
    try { await access(`${qa}/forbidden.db${suffix}`); dbFiles.push(suffix || 'db'); }
    catch (e) { if (e.code !== 'ENOENT') throw e; }
  }
  console.log(JSON.stringify({ cleanup: { remaining_owned_children: remaining.length, scratch_db_files: dbFiles, listener_closed: true } }));
  if (remaining.length || dbFiles.length) failed = true;
}
if (failed) process.exitCode = 1;
```

For the failure phase the same `<QA>/ssh` wrapper must recognize the loopback machine, retaining real SSH rather than returning a stub error. Use this complete wrapper instead of the short happy-only version when preparing the two-phase runner:

```sh
#!/bin/sh
if [ "$5" = "127.0.0.1" ]; then
  exec /usr/bin/ssh -F /dev/null -o ClearAllForwardings=yes -o StrictHostKeyChecking=yes -o UpdateHostKeys=no -o ControlMaster=no -o ControlPath=none -p "$TASK27_STALL_PORT" "$@"
fi
exec /usr/bin/ssh -o ClearAllForwardings=yes -o StrictHostKeyChecking=yes -o UpdateHostKeys=no -o ControlMaster=no -o ControlPath=none "$@"
```

Gateway SSH arguments are `-o BatchMode=yes -o ConnectTimeout=5 <machine> omo thread list --all-scope --json`, so `$5` is the machine. The loopback SSH client stalls on the owned endpoint's banner and fails under ConnectTimeout=5; no DNS lookup, remote authentication, host-key write, model or session control occurs for that failure target. This proves the plan's isolated unreachable host / <=25-second behavior, not the registry's exact 20-second cancellation deadline. Existing fake-clock tests cover the latter; OS kill-on-timeout remains a separate live receipt if required.

Exact runner invocation, from a local tool monitor at execution time:

```sh
cd "$QA" && /usr/bin/env -i HOME=/private/tmp PATH=/usr/bin:/bin:/usr/sbin:/sbin TMPDIR="$QA" /Users/indo/.bun/bin/bun --no-env-file "$QA/run.mjs"
```

The parent runner's HOME is scratch-independent; only the deliberately narrow gateway child env uses owner HOME. Pass no shell-expanded session text. Do not launch a gateway service. Do not run omo separately to refresh historical captures. No local --help is needed: current clap source establishes the command.

## Receipts, expected exits and cleanup

Happy receipt: new `task-27-live-gateway.json`, preserving historical task-27.json unchanged. Record executable hash, exact argv, isolated cwd, actual exit/duration, local/omarchy counts and machine/name/cwd projection only. Expected exit0, >=1 record from each machine; idle sessions may disappear, so failure to meet that is an actual QA failure, not permission to create or message sessions.

Failure receipt: new `task-27-live-unreachable.json`; record actual exit/duration, successful-machine counts and sanitized host errors. Expected partial-success exit0, local records retained, loopback host error code3, elapsed <=25 seconds. Prefer retaining omarchy records as well, but record any real remote error separately. This is a deliberately unreachable loopback SSH handshake, not a claim that an owner remote alias is down. Raw upstream output, session IDs and stderr are never persisted; stderr byte count suffices.

Emit and retain cleanup evidence even when acceptance fails. Await gateway children, stop the owned listener, inspect tracked child PIDs, and record no scratch DB artifacts. The runner only signals children still attached to its live owned gateway PID; a surviving previously observed orphan causes cleanup failure and is not blindly killed. Before any manual orphan cleanup, recheck executable identity and ownership; a reused PID is not owned. Do not kill arbitrary omo/ssh PIDs, daemon PIDs or actual8/9 processes. Preserve the private scratch root if cleanup or receipt validation fails; otherwise remove only its named helper files and then its empty directories using owned-path deletion. Write retained JSON receipts with apply_patch after parsing monitor output, not by redirecting raw stdout.

Current gating evidence is attributed to the parent: registry7 + entry/session1 + CLI6 passing, strict clippy0 and frozen exact binary. These were not rerun here. Remaining live evidence is the two gateway invocations above plus their cleanup receipts. No confirmed todo27 claim follows from this preflight or the prior raw remote captures.
