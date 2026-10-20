// Isolated mock tracker API for the remaining-checks pass (D17 / D20 / D21 / D22).
//
// Port 9231, separate from ui-final's 9131, so no earlier harness can be disturbed.
// Bootstrap is served from boot-snapshot.json, a read-only snapshot taken from the live gateway
// (GET http://127.0.0.1:9119/api/tracker/bootstrap, http=200, 7192 bytes,
// sha256 bfb521f23383a4fb00a5d2ad6ffffdb5fc6fc3ba796d9fe2cabfc763bc5511b5). No live service is
// contacted at run time; nothing here writes to the gateway.
//
// Shapes are copied from ui-final/server.ts, which copied them from the Rust handlers
// (src/tracker/{model,issues,threads}.rs) and web/src/tracker/types.ts.
const here = import.meta.dir
const liveBoot: any = await Bun.file(`${here}/boot-snapshot.json`).json()

const omo = liveBoot.teams.find((t: any) => !t.system)
const me = liveBoot.me
const other = liveBoot.users.find((u: any) => u.id !== me.id && u.kind === 'human') ?? liveBoot.users.find((u: any) => u.id !== me.id)
const omoStates = liveBoot.states.filter((s: any) => s.team_id === omo.id).sort((a: any, b: any) => a.position - b.position)
const stateOf = (type: string) => omoStates.find((s: any) => s.type === type) ?? omoStates[0]
const backlog = stateOf('backlog')
const started = stateOf('started')
const completed = stateOf('completed')
// src/tracker/lanes.rs:433-449 decides on the target state *type*: any completed-type state needs a
// verified lane (or a force_note), so every completed state of the team rejects the same way.
const completedIds = new Set(omoStates.filter((s: any) => s.type === 'completed').map((s: any) => s.id))

const iso = (ms: number) => new Date(ms).toISOString()
const now = Date.now()
const DAY = 86_400_000

type Entry = { t: number; done: boolean; method: string; path: string; note?: string }
const LOG: Entry[] = []

// Lane status the lane panel must render. Set per check through /__ctl/lane.
let laneStatus = 'working'
// cwd the session panel must shorten. Set per check through /__ctl/cwd.
let sessionCwd = '/Users/alice/code/omon'

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
    creator_id: other.id,
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

const laneSummary = () => ({
  key: 'qa-lane',
  status: laneStatus,
  model: 'test/model',
  turns_run: 2,
  last_activity_at: iso(now - 600_000),
  thread_channel_id: null,
})

function seedIssues() {
  return [
    // Lane-backed: the lane panel renders ALLOWED[laneStatus] for this one.
    makeIssue({
      id: 'iss-lane',
      number: 101,
      identifier: 'OMO-101',
      title: 'Lane backed issue',
      description: 'Lane description',
      state_id: started.id,
      lane_key: 'qa-lane',
      lane: laneSummary(),
      assignee_id: me.id,
      subscriber_ids: [me.id],
      sort_order: 1,
    }),
    // Session-backed: the session panel renders shortenPath(session.cwd) for this one.
    makeIssue({
      id: 'iss-session',
      number: 102,
      identifier: 'OMO-102',
      title: 'Session backed issue',
      description: 'Session description',
      state_id: started.id,
      assignee_id: me.id,
      subscriber_ids: [me.id],
      sort_order: 2,
    }),
    // Plain rows, dated around the DST change, so the timeline has spans and groups.
    makeIssue({
      id: 'iss-plain',
      number: 103,
      identifier: 'OMO-103',
      title: 'Plain issue',
      description: 'Plain description',
      state_id: backlog.id,
      due_date: '2026-11-04',
      sort_order: 3,
    }),
    makeIssue({
      id: 'iss-plain-2',
      number: 104,
      identifier: 'OMO-104',
      title: 'Another plain issue',
      description: 'Plain description',
      state_id: backlog.id,
      due_date: '2026-11-12',
      sort_order: 4,
    }),
  ]
}

let issues = seedIssues()

function laneDetail() {
  return {
    lane: {
      key: 'qa-lane',
      title: 'Lane backed issue',
      status: laneStatus,
      cwd: '/tmp/qa-lane',
      brief_path: '/tmp/brief.md',
      model: 'test/model',
      done_signals: 'tests pass',
      claim_evidence: null,
      objection: null,
      turns_run: 2,
      last_turn_outcome: null,
      last_report: null,
      last_activity_at: iso(now - 600_000),
      thread_channel_id: null,
      created_at: iso(now - DAY),
    },
    history: [],
    transcript: [],
    events: [],
  }
}

function sessionDetail() {
  return {
    machine: 'workstation',
    session_id: 'ses_qa_1',
    state: 'working',
    streaming: false,
    message_count: 3,
    pending_messages: 0,
    cwd: sessionCwd,
    updated_at: iso(now - 120_000),
    questions: [],
    last_user: 'please shorten the path',
    last_assistant: 'done',
  }
}

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json', 'cache-control': 'no-store' },
  })

const server = Bun.serve({
  hostname: '127.0.0.1',
  port: 9231,
  idleTimeout: 120,
  async fetch(req) {
    const url = new URL(req.url)
    const path = url.pathname
    const method = req.method

    // ---- control plane -----------------------------------------------------
    if (path === '/__ctl/ping') return json({ mock: 'remaining-checks', laneStatus, sessionCwd })
    if (path === '/__ctl/log') return json(LOG)
    if (path === '/__ctl/reset') {
      LOG.length = 0
      laneStatus = 'working'
      sessionCwd = '/Users/alice/code/omon'
      issues = seedIssues()
      return json({ ok: true })
    }
    if (path === '/__ctl/lane') {
      const body = (await req.json().catch(() => ({}))) as { status?: string }
      laneStatus = body.status ?? 'working'
      for (const issue of issues) if (issue.lane_key) issue.lane = laneSummary()
      return json({ laneStatus })
    }
    if (path === '/__ctl/cwd') {
      const body = (await req.json().catch(() => ({}))) as { cwd?: string }
      sessionCwd = body.cwd ?? ''
      return json({ sessionCwd })
    }
    if (path === '/__ctl/state') return json({ issues, laneStatus, sessionCwd, omo, backlog, started, completed })

    const entry: Entry = { t: Date.now(), done: false, method, path }
    LOG.push(entry)
    const finish = (note?: string) => {
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
      return json(liveBoot)
    }
    if (path === '/api/tracker/issues') {
      finish()
      return json({ items: issues })
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
    if (path === '/api/tracker/favorites' || path === '/api/tracker/users' || path === '/api/tracker/labels') {
      finish('noop')
      return json({ items: [], favorited: false, favorite: null })
    }

    const issueMatch = path.match(/^\/api\/tracker\/issues\/([^/]+)$/)
    if (issueMatch && method === 'GET') {
      const idOrIdent = decodeURIComponent(issueMatch[1])
      const issue = issues.find((i) => i.id === idOrIdent || i.identifier === idOrIdent)
      finish()
      if (!issue) return json({ error: 'issue not found', code: 'not_found' }, 404)
      return json({
        issue,
        children: [],
        parent: null,
        relations: [],
        comments: [],
        activity: [],
        links: [],
        subscriber_ids: issue.subscriber_ids,
        lane_detail: issue.lane_key ? laneDetail() : null,
        session_detail: issue.id === 'iss-session' ? sessionDetail() : null,
      })
    }

    if (issueMatch && method === 'PATCH') {
      const patch = await readBody()
      const issue = issues.find((i) => i.id === decodeURIComponent(issueMatch[1]))
      finish(`patch=${JSON.stringify(patch)}`)
      if (!issue) return json({ error: 'issue not found', code: 'not_found' }, 404)
      // src/tracker/lanes.rs:434-449 - closing a lane-linked issue whose lane is not verified is a
      // 409 lane_not_verified; the same text src/tracker/issues.rs:441-445 returns.
      if (issue.lane_key && patch.state_id && completedIds.has(patch.state_id) && !patch.force_note) {
        return json({ error: 'lane must be verified before closing; use force close', code: 'lane_not_verified' }, 409)
      }
      Object.assign(issue, patch, { updated_at: iso(Date.now()) })
      if (issue.lane) issue.lane = laneSummary()
      return json(issue)
    }

    if (path === '/api/tracker/issues/batch') {
      const input = await readBody()
      finish(`batch=${JSON.stringify(input).slice(0, 120)}`)
      const items = (input.ids ?? []).map((id: string) => issues.find((i) => i.id === id)).filter(Boolean)
      return json({ items })
    }

    finish('unhandled')
    return json({ error: `unhandled ${method} ${path}` }, 404)
  },
})

console.log(
  `remaining-checks mock on http://127.0.0.1:${server.port} team=${omo.key} me=${me.id} backlog=${backlog.id} completed=${completed.id}`,
)
