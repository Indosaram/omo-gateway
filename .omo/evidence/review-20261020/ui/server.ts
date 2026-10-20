// Isolated mock of the tracker API for UI verification of D23/D25/D28/D29.
// Reads are seeded from the live gateway bootstrap; every write stays in memory here.
// Requests on gated paths stay pending until an explicit release signal, so the UI assertions
// never depend on a fixed hold time.
type Entry = { t: number; ms: number; done: boolean; method: string; path: string; note?: string }

const LOG: Entry[] = []
const PENDING = new Map<string, { note: string; release: () => void }[]>()
const GATES = new Set<string>()

const now = Date.now()
const iso = (ms: number) => new Date(ms).toISOString()

const liveBoot = await (async () => {
  try {
    const r = await fetch('http://127.0.0.1:9119/api/tracker/bootstrap')
    if (r.ok) return await r.json()
  } catch {}
  return await Bun.file('/tmp/boot.json').json()
})()

const team = liveBoot.teams.find((t: any) => t.key === 'OMO') ?? liveBoot.teams[0]

function seedProject() {
  return {
    id: 'proj-1',
    name: 'Atlas',
    description: '',
    summary: 'Verification project',
    icon: 'box',
    color: '#5e6ad2',
    status: 'planned',
    priority: 2,
    lead_id: 'me',
    start_date: null,
    target_date: null,
    sort_order: 1,
    created_at: iso(now - 86_400_000),
    updated_at: iso(now - 86_400_000),
    archived_at: null,
    team_ids: [team.id],
    member_ids: [],
    issue_count: 0,
    completed_issue_count: 0,
    latest_health: null,
  }
}

const state: { projects: any[]; milestones: any[]; cycles: any[]; updates: any[]; seq: number } = {
  projects: [seedProject()],
  milestones: [],
  cycles: [],
  updates: [],
  seq: 0,
}

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json', 'cache-control': 'no-store' },
  })

// A gated request registers itself as pending and resolves only on release.
function gate(path: string, note: string): Promise<void> | null {
  if (!GATES.has(path)) return null
  return new Promise<void>((release) => {
    const list = PENDING.get(path) ?? []
    list.push({ note, release })
    PENDING.set(path, list)
  })
}

function release(path: string) {
  const list = PENDING.get(path) ?? []
  PENDING.delete(path)
  for (const p of list) p.release()
  return list.length
}

const server = Bun.serve({
  hostname: '127.0.0.1',
  port: 9121,
  idleTimeout: 120,
  async fetch(req) {
    const url = new URL(req.url)
    const path = url.pathname
    const method = req.method

    // ---- control plane -----------------------------------------------------
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
      return json({ released: release(target) })
    }
    if (path === '/__ctl/gate') {
      const body = (await req.json()) as Record<string, boolean>
      for (const [k, v] of Object.entries(body)) {
        if (v) GATES.add(k)
        else GATES.delete(k)
      }
      return json([...GATES])
    }
    if (path === '/__ctl/state') return json(state)
    if (path === '/__ctl/reset-state') {
      state.projects = [seedProject()]
      state.milestones = []
      state.cycles = []
      state.updates = []
      state.seq = 0
      LOG.length = 0
      PENDING.clear()
      return json(state)
    }

    const entry: Entry = { t: Date.now(), ms: 0, done: false, method, path }
    LOG.push(entry)
    if (LOG.length > 500) LOG.shift()
    const finish = (note?: string) => {
      entry.ms = Date.now() - entry.t
      entry.done = true
      if (note) entry.note = note
    }
    const readBody = async () => {
      try {
        return await req.json()
      } catch {
        return {}
      }
    }

    // ---- reads -------------------------------------------------------------
    if (path === '/api/tracker/bootstrap') {
      finish()
      return json({ ...liveBoot, projects: state.projects, cycles: state.cycles, milestones: state.milestones })
    }
    if (path === '/api/tracker/issues') {
      finish()
      return json({ items: [] })
    }
    if (path === '/api/tracker/notifications') {
      finish()
      return json({ items: [] })
    }
    if (path === '/api/tracker/search') {
      const q = url.searchParams.get('q') ?? ''
      const wait = gate('/api/tracker/search', `q=${q}`)
      if (wait) await wait
      finish(`q=${q}`)
      return json({ issues: [], projects: state.projects.filter((p) => p.name.toLowerCase().includes(q.toLowerCase())) })
    }
    if (path === '/api/tracker/projects') {
      finish()
      return json({ items: state.projects })
    }
    const detail = path.match(/^\/api\/tracker\/projects\/([^/]+)$/)
    if (detail && method === 'GET') {
      const p = state.projects.find((x) => x.id === detail[1])
      finish()
      if (!p) return json({ error: 'not found' }, 404)
      return json({
        project: p,
        updates: state.updates.filter((u) => u.project_id === p.id),
        milestones: state.milestones.filter((m) => m.project_id === p.id),
      })
    }
    if (detail && method === 'PATCH') {
      const patch = await readBody()
      const p = state.projects.find((x) => x.id === detail[1])
      finish(`patch=${JSON.stringify(patch)}`)
      if (!p) return json({ error: 'not found' }, 404)
      Object.assign(p, patch, { updated_at: iso(Date.now()) })
      return json(p)
    }

    // ---- writes ------------------------------------------------------------
    const ms = path.match(/^\/api\/tracker\/projects\/([^/]+)\/milestones$/)
    if (ms && method === 'POST') {
      const input = await readBody()
      const wait = gate('/api/tracker/milestone', `name=${input.name}`)
      if (wait) await wait
      state.seq += 1
      const created = {
        id: `ms-${state.seq}`,
        project_id: ms[1],
        name: input.name,
        target_date: input.target_date ?? null,
        sort_order: state.seq,
      }
      state.milestones.push(created)
      finish(`name=${input.name}`)
      return json(created)
    }
    if (path === '/api/tracker/cycles' && method === 'POST') {
      const input = await readBody()
      state.seq += 1
      const created = {
        id: `cyc-${state.seq}`,
        team_id: input.team_id,
        number: state.seq,
        name: input.name ?? null,
        description: null,
        starts_at: input.starts_at,
        ends_at: input.ends_at,
        completed_at: null,
      }
      state.cycles.push(created)
      finish(`start=${input.starts_at} end=${input.ends_at}`)
      return json(created)
    }

    finish('unhandled')
    return json({ error: `unhandled ${method} ${path}` }, 404)
  },
})

console.log(`mock tracker api on http://127.0.0.1:${server.port} team=${team.key}`)
