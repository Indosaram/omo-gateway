// Isolated mock of the tracker API for the D11/D12/D14/D15/D18/D19/D24/D26/D27 UI pass.
// Shapes are copied from the Rust handlers (src/tracker/{model,issues,api}.rs) and web/src/tracker/types.ts:
// every issue payload carries `subscriber_ids`, and a lane rejection is HTTP 409
// {"error": "...", "code": "lane_not_verified"}.
//
// Reads are seeded once from the live gateway's bootstrap (read-only) and then served from memory.
// Requests on gated paths stay pending until an explicit release, so no assertion depends on a delay.
type Entry = { t: number; ms: number; done: boolean; method: string; path: string; note?: string }
type Mode = 'ok' | 'fail500' | 'fail409'
type Pending = { note: string; release: (mode: Mode) => void }

const LOG: Entry[] = []
const PENDING = new Map<string, Pending[]>()
const GATES = new Map<string, Mode>()
const CLAIM_TTL_MS = 150_000
const OWNER: { token: string; since: number } = { token: '', since: 0 }

const iso = (ms: number) => new Date(ms).toISOString()
const now = Date.now()

const liveBoot: any = await (async () => {
  try {
    const r = await fetch('http://127.0.0.1:9119/api/tracker/bootstrap')
    if (r.ok) return await r.json()
  } catch {}
  return await Bun.file('/tmp/boot-live.json').json()
})()

const omo = liveBoot.teams.find((t: any) => t.key === 'OMO') ?? liveBoot.teams.find((t: any) => !t.system)
const otherUser =
  liveBoot.users.find((u: any) => u.id !== liveBoot.me.id && u.kind === 'human') ?? liveBoot.users.find((u: any) => u.id !== liveBoot.me.id)
const omoStates = liveBoot.states
  .filter((s: any) => s.team_id === omo.id)
  .sort((a: any, b: any) => a.position - b.position)
const stateOf = (type: string) => omoStates.find((s: any) => s.type === type) ?? omoStates[0]
const backlog = stateOf('backlog')
const completed = stateOf('completed')
const started = stateOf('started')

function makeIssue(over: Record<string, any>) {
  return {
    id: 'x',
    team_id: omo.id,
    number: 1,
    identifier: 'OMO-1',
    title: 'Issue',
    description: '',
    state_id: backlog.id,
    priority: 0,
    estimate: null,
    assignee_id: null,
    creator_id: otherUser.id,
    project_id: null,
    milestone_id: null,
    cycle_id: null,
    parent_id: null,
    due_date: null,
    sort_order: 0,
    lane_key: null,
    created_at: iso(now - 3_600_000),
    updated_at: iso(now - 3_600_000),
    started_at: null,
    completed_at: null,
    canceled_at: null,
    archived_at: null,
    snoozed_until: null,
    label_ids: [],
    subscriber_ids: [],
    sub_issue_count: 0,
    sub_issue_completed: 0,
    comment_count: 0,
    lane: null,
    session_ref: null,
    session: null,
    ...over,
  }
}

function seedIssues() {
  return [
    // Subscribed by me, but neither creator nor assignee: the Subscribed tab must use subscriber_ids.
    makeIssue({
      id: 'iss-sub',
      number: 101,
      identifier: 'OMO-101',
      title: 'Watched only by subscription',
      description: 'Original subscription description',
      subscriber_ids: [liveBoot.me.id],
      creator_id: otherUser.id,
      assignee_id: otherUser.id,
      sort_order: 1,
    }),
    // Created by me, not subscribed: must NOT appear under Subscribed (the pre-fix approximation did).
    makeIssue({
      id: 'iss-created',
      number: 102,
      identifier: 'OMO-102',
      title: 'Created but not subscribed',
      description: 'Original created description',
      subscriber_ids: [],
      creator_id: liveBoot.me.id,
      assignee_id: otherUser.id,
      sort_order: 2,
    }),
    // Lane-backed: closing it is rejected with 409 lane_not_verified.
    makeIssue({
      id: 'iss-lane',
      number: 103,
      identifier: 'OMO-103',
      title: 'Lane backed issue',
      description: 'Original lane description',
      lane_key: 'qa-lane-1',
      lane: { key: 'qa-lane-1', status: 'working', model: 'test/model', turns_run: 2, last_activity_at: iso(now - 600_000), thread_channel_id: null },
      subscriber_ids: [liveBoot.me.id],
      assignee_id: liveBoot.me.id,
      sort_order: 3,
    }),
    // Description draft target (no lane, PATCH is gated then failed).
    makeIssue({
      id: 'iss-draft',
      number: 104,
      identifier: 'OMO-104',
      title: 'Draft target issue',
      description: 'Original draft description',
      subscriber_ids: [liveBoot.me.id],
      sort_order: 4,
    }),
    // PATCH always fails with a plain 500 (single generic toast path).
    makeIssue({
      id: 'iss-500',
      number: 105,
      identifier: 'OMO-105',
      title: 'Server error issue',
      description: 'Original error description',
      sort_order: 5,
    }),
  ]
}

const state = {
  scenario: 'mixed' as 'mixed' | 'system-only',
  issues: seedIssues(),
}

function bootstrap() {
  if (state.scenario === 'system-only') {
    const teams = liveBoot.teams.filter((t: any) => t.system)
    const ids = new Set(teams.map((t: any) => t.id))
    return { ...liveBoot, teams, states: liveBoot.states.filter((s: any) => ids.has(s.team_id)) }
  }
  return liveBoot
}

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json', 'cache-control': 'no-store' },
  })

function hold(path: string, note: string): Promise<Mode> | null {
  const mode = GATES.get(path)
  if (!mode) return null
  return new Promise<Mode>((release) => {
    const list = PENDING.get(path) ?? []
    list.push({ note, release })
    PENDING.set(path, list)
  })
}

function releaseAll(path: string) {
  const list = PENDING.get(path) ?? []
  PENDING.delete(path)
  const mode = GATES.get(path) ?? 'ok'
  for (const p of list) p.release(mode)
  return list.length
}

const server = Bun.serve({
  hostname: '127.0.0.1',
  port: 9131,
  idleTimeout: 120,
  async fetch(req) {
    const url = new URL(req.url)
    const path = url.pathname
    const method = req.method

    // ---- control plane -----------------------------------------------------
    // ---- ownership claim ---------------------------------------------------
    // Two harnesses sharing this mock reset each other's log/pending/state and produce garbage
    // verdicts. A run claims the mock with a unique token; a later run with a different token is
    // refused instead of silently corrupting the first one's evidence.
    if (path === '/__ctl/claim') {
      const body = (await req.json().catch(() => ({}))) as { token?: string }
      const token = body.token ?? ''
      if (!token) return json({ ok: false, error: 'token required' }, 400)
      if (OWNER.token && OWNER.token !== token && Date.now() - OWNER.since < CLAIM_TTL_MS) {
        return json({ ok: false, owner: OWNER, error: 'owned by another harness' }, 409)
      }
      OWNER.token = token
      OWNER.since = Date.now()
      return json({ ok: true, owner: OWNER })
    }
    if (path === '/__ctl/unclaim') {
      // Only the QA task that owns this harness may clear a stale claim of its own earlier attempt.
      // A foreign harness's token does not carry this prefix, so this cannot steal a live run.
      const body = (await req.json().catch(() => ({}))) as { token?: string }
      const stale = OWNER.token.startsWith('st_01a1582e-')
      if (OWNER.token && !stale && OWNER.token !== (body.token ?? '')) {
        return json({ ok: false, error: 'owned by another harness', owner: OWNER }, 409)
      }
      OWNER.token = ''
      OWNER.since = 0
      return json({ ok: true, cleared: true })
    }
    if (path === '/__ctl/owner') return json({ owner: OWNER, ttlMs: CLAIM_TTL_MS })
    if (path === '/__ctl/ping') return json({ mock: 'ui-final-d18', token: OWNER.token })

    if (path === '/__ctl/log') return json(LOG)
    if (path === '/__ctl/reset') {
      LOG.length = 0
      return json({ ok: true })
    }
    if (path === '/__ctl/pending') {
      return json([...PENDING.entries()].map(([p, list]) => ({ path: p, notes: list.map((x) => x.note) })))
    }
    if (path === '/__ctl/release') {
      const target = url.searchParams.get('path') ?? ''
      return json({ released: releaseAll(target), mode: GATES.get(target) ?? 'ok' })
    }
    if (path === '/__ctl/gate') {
      const body = (await req.json()) as Record<string, Mode | false>
      for (const [k, v] of Object.entries(body)) {
        if (v) GATES.set(k, v)
        else GATES.delete(k)
      }
      return json([...GATES.entries()])
    }
    if (path === '/__ctl/scenario') {
      const body = (await req.json()) as { name: 'mixed' | 'system-only' }
      state.scenario = body.name
      return json({ scenario: state.scenario })
    }
    if (path === '/__ctl/state') return json({ scenario: state.scenario, issues: state.issues, omo, backlog, started, completed })
    if (path === '/__ctl/reset-state') {
      state.scenario = 'mixed'
      state.issues = seedIssues()
      LOG.length = 0
      PENDING.clear()
      return json({ ok: true, owner: OWNER.token })
    }

    const entry: Entry = { t: Date.now(), ms: 0, done: false, method, path }
    LOG.push(entry)
    if (LOG.length > 500) LOG.shift()
    const finish = (note?: string) => {
      entry.ms = Date.now() - entry.t
      entry.done = true
      if (note) entry.note = note
    }
    const readBody = async (): Promise<any> => {
      try {
        return await req.json()
      } catch {
        return {}
      }
    }

    // ---- tracker reads -----------------------------------------------------
    if (path === '/api/tracker/bootstrap') {
      finish()
      return json(bootstrap())
    }
    if (path === '/api/tracker/issues') {
      // A gated list read answers with the rows as they were when the request arrived: the stale
      // payload the guard has to discard once a write settles.
      const snapshot = JSON.stringify({ items: state.issues })
      const mode = await hold(path, `snapshot@${entry.t}`)
      finish(`gated=${mode ?? 'no'}`)
      if (mode === 'fail500') return json({ error: 'boom' }, 500)
      return new Response(snapshot, { status: 200, headers: { 'content-type': 'application/json', 'cache-control': 'no-store' } })
    }
    if (path === '/api/tracker/notifications') {
      finish()
      return json({ items: [] })
    }
    if (path === '/api/tracker/search') {
      finish()
      return json({ issues: [], projects: [] })
    }
    if (path === '/api/tracker/projects') {
      finish()
      return json({ items: [] })
    }
    if (path === '/api/tracker/sessions') {
      finish()
      return json({ machines: [], sessions: [] })
    }
    if (path === '/api/tracker/views') {
      finish()
      return json({ items: [] })
    }

    const issueMatch = path.match(/^\/api\/tracker\/issues\/([^/]+)$/)
    if (issueMatch && method === 'GET') {
      const idOrIdent = decodeURIComponent(issueMatch[1])
      const issue = state.issues.find((i) => i.id === idOrIdent || i.identifier === idOrIdent)
      if (!issue) {
        finish()
        return json({ error: 'issue not found', code: 'not_found' }, 404)
      }
      // A gated detail read answers with the issue as it was when the request arrived, which is
      // exactly the payload a read issued before a write would return after the write settles.
      const snapshot = JSON.stringify({
        issue,
        children: [],
        parent: null,
        relations: [],
        comments: [],
        activity: [],
        links: [],
        subscriber_ids: issue.subscriber_ids,
        lane_detail: null,
        session_detail: null,
      })
      // Only writes are gated: holding the GET on the same path starves the UI of the detail it
      // needs to render, which changes the flow the case is trying to observe.
      const mode = method === 'GET' ? null : await hold(path, `detail-snapshot@${entry.t}`)
      finish(`gated=${mode ?? 'no'}`)
      if (mode === 'fail500') return json({ error: 'boom' }, 500)
      return new Response(snapshot, { status: 200, headers: { 'content-type': 'application/json', 'cache-control': 'no-store' } })
      // eslint-disable-next-line no-unreachable
      return json({
        issue,
        children: [],
        parent: null,
        relations: [],
        comments: [],
        activity: [],
        links: [],
        subscriber_ids: issue.subscriber_ids,
        lane_detail: null,
        session_detail: null,
      })
    }

    if (issueMatch && method === 'PATCH') {
      const patch = await readBody()
      const issue = state.issues.find((i) => i.id === decodeURIComponent(issueMatch[1]))
      const mode = await hold(path, `patch=${JSON.stringify(patch)}`)
      finish(`patch=${JSON.stringify(patch)}`)
      if (!issue) return json({ error: 'issue not found', code: 'not_found' }, 404)
      if (mode === 'fail500' || issue.id === 'iss-500') return json({ error: 'tracker unavailable' }, 500)
      // src/tracker/issues.rs:438 - lane must be verified before closing; use force close.
      if (issue.lane_key && patch.state_id && !patch.force_note) {
        return json({ error: 'lane must be verified before closing; use force close', code: 'lane_not_verified' }, 409)
      }
      if (mode === 'fail409') return json({ error: 'conflict', code: 'conflict' }, 409)
      Object.assign(issue, patch, { updated_at: iso(Date.now()) })
      return json(issue)
    }

    if (path === '/api/tracker/issues' && method === 'POST') {
      const input = await readBody()
      finish(`create=${JSON.stringify(input).slice(0, 120)}`)
      return json(makeIssue({ id: `iss-new-${state.issues.length}`, identifier: `OMO-${200 + state.issues.length}`, title: input.title ?? '' }), 201)
    }

    if (path === '/api/tracker/issues/batch') {
      const input = await readBody()
      finish(`batch=${JSON.stringify(input).slice(0, 120)}`)
      const items = (input.ids ?? []).map((id: string) => state.issues.find((i) => i.id === id)).filter(Boolean)
      return json({ items })
    }

    if (path === '/api/tracker/favorites' || path === '/api/tracker/users' || path === '/api/tracker/labels') {
      finish('noop')
      return json({ items: [], favorited: false, favorite: null })
    }

    // ---- dashboard endpoints: proxied to the live gateway -------------------
    // The D15 check leaves the tracker for `#/overview`, which mounts the dashboard; it needs the
    // real payload shapes, so these are forwarded to the running gateway (read-only GETs).
    if (path.startsWith('/api/') && !path.startsWith('/api/tracker')) {
      try {
        const upstream = await fetch(`http://127.0.0.1:9119${path}${url.search}`, {
          method,
          headers: { 'content-type': 'application/json' },
          body: method === 'GET' || method === 'HEAD' ? undefined : await req.text(),
        })
        const text = await upstream.text()
        finish(`proxy=${upstream.status}`)
        return new Response(text, {
          status: upstream.status,
          headers: { 'content-type': upstream.headers.get('content-type') ?? 'application/json' },
        })
      } catch (err) {
        finish(`proxy-failed=${String(err).slice(0, 60)}`)
        return json({ error: 'dashboard proxy failed' }, 502)
      }
    }

    finish('unhandled')
    return json({ error: `unhandled ${method} ${path}` }, 404)
  },
})

console.log(
  `mock tracker api on http://127.0.0.1:${server.port} team=${omo.key} me=${liveBoot.me.id} other=${otherUser.id} backlog=${backlog.id} completed=${completed.id}`,
)
