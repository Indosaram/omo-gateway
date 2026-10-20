// Executable checks for D13 and D16. No browser, no network: both call the real modules.
//
//   TZ=America/New_York bun run checks-node.ts
//
// D13 exercises web/src/tracker/api.ts `call` with a stubbed global fetch (the lane's stubbed-fetch
// check that left no artifact), and replays the same cases against prefix-api.ts - the file exactly
// as it is at HEAD (`git show HEAD:web/src/tracker/api.ts`, sha256 b3cf528d...), i.e. the pre-fix
// code - so every assertion is shown to be load-bearing.
// D16 calls the real `dueShortcuts` (web/src/tracker/format.ts) and the real `applyFilters`
// (web/src/tracker/store.tsx) with the clock pinned to a moment whose +7-day range crosses the
// 2026-11-01 US fall-back, and compares against the HEAD formula for the same instants.
const here = import.meta.dir
const web = '/Users/indo/code/project/omon-gateway/web/src/tracker'

type Row = { id: string; check: string; ok: boolean; detail: string }
const rows: Row[] = []
function record(id: string, check: string, ok: boolean, detail: string) {
  rows.push({ id, check, ok, detail })
  console.log(`${ok ? 'PASS' : 'FAIL'} ${id} ${check} :: ${detail}`)
}

// ------------------------------------------------------------------ D13
type Spec = { status: number; statusText?: string; body: string }
let next: Spec = { status: 200, body: '{}' }
const realFetch = globalThis.fetch
globalThis.fetch = (async () => new Response(next.body === '' ? null : next.body, {
  status: next.status,
  statusText: next.statusText ?? '',
})) as unknown as typeof fetch

const api = await import(`${web}/api.ts`)
const prefix = await import(`${here}/prefix-api.ts`)

async function attempt(mod: any, spec: Spec): Promise<{ kind: string; status?: number; message?: string; code?: any }> {
  next = spec
  try {
    await mod.trackerApi.issue('iss-1')
    return { kind: 'resolved' }
  } catch (err: any) {
    return {
      kind: err?.constructor?.name ?? 'unknown',
      status: err?.status,
      message: err?.message,
      code: err?.code,
    }
  }
}

const cases: { name: string; spec: Spec; expect: (r: any) => boolean; expectPrefix: (r: any) => boolean; detail: (r: any) => string }[] = [
  {
    name: 'plain-text 403 (host/origin guard)',
    spec: { status: 403, statusText: 'Forbidden', body: 'origin not allowed' },
    expect: (r) => r.kind === 'TrackerApiError' && r.status === 403 && r.message === 'origin not allowed' && r.code === null,
    expectPrefix: (r) => r.kind === 'SyntaxError' && r.status === undefined,
    detail: (r) => `fixed=${JSON.stringify(r)}`,
  },
  {
    name: '502 HTML from a proxy',
    spec: { status: 502, statusText: 'Bad Gateway', body: '<html><body>502 Bad Gateway</body></html>' },
    expect: (r) => r.kind === 'TrackerApiError' && r.status === 502 && r.message === '<html><body>502 Bad Gateway</body></html>',
    expectPrefix: (r) => r.kind === 'SyntaxError' && r.status === undefined,
    detail: (r) => `fixed=${JSON.stringify(r)}`,
  },
  {
    name: 'JSON 409 with code',
    spec: { status: 409, body: '{"error":"lane must be verified before closing; use force close","code":"lane_not_verified"}' },
    expect: (r) => r.kind === 'TrackerApiError' && r.status === 409 && r.message === 'lane must be verified before closing; use force close' && r.code === 'lane_not_verified',
    expectPrefix: (r) => r.kind === 'TrackerApiError' && r.status === 409 && r.code === 'lane_not_verified',
    detail: (r) => `fixed=${JSON.stringify(r)}`,
  },
  {
    name: 'JSON 500',
    spec: { status: 500, body: '{"error":"tracker unavailable"}' },
    expect: (r) => r.kind === 'TrackerApiError' && r.status === 500 && r.message === 'tracker unavailable',
    expectPrefix: (r) => r.kind === 'TrackerApiError' && r.status === 500 && r.message === 'tracker unavailable',
    detail: (r) => `fixed=${JSON.stringify(r)}`,
  },
  {
    name: 'empty 403 body keeps the status line',
    spec: { status: 403, statusText: 'Forbidden', body: '' },
    expect: (r) => r.kind === 'TrackerApiError' && r.status === 403 && r.message === '403 Forbidden',
    expectPrefix: (r) => r.kind === 'TrackerApiError' && r.status === 403 && r.message === '403 Forbidden',
    detail: (r) => `fixed=${JSON.stringify(r)}`,
  },
]

for (const c of cases) {
  const fixed = await attempt(api, c.spec)
  record('D13', c.name, c.expect(fixed), c.detail(fixed))
  const before = await attempt(prefix, c.spec)
  record('D13', `${c.name} - pre-fix counterfactual`, c.expectPrefix(before), `prefix=${JSON.stringify(before)}`)
}

// a 200 that is not JSON must still surface as a parse error (the payload contract is unchanged)
next = { status: 200, body: 'not json' }
const okParse = await attempt(api, { status: 200, body: '{"items":[]}' })
const badParse = await attempt(api, { status: 200, body: 'not json' })
const empty204 = await attempt(api, { status: 204, body: '' })
record(
  'D13',
  'success path unchanged: JSON 200 resolves, non-JSON 200 throws SyntaxError, 204 resolves',
  okParse.kind === 'resolved' && badParse.kind === 'SyntaxError' && empty204.kind === 'resolved',
  `json200=${JSON.stringify(okParse)} nonJson200=${JSON.stringify(badParse)} status204=${JSON.stringify(empty204)}`,
)
globalThis.fetch = realFetch

// ------------------------------------------------------------------ D16
const tz = Intl.DateTimeFormat().resolvedOptions().timeZone
const format = await import(`${web}/format.ts`)
const store = await import(`${web}/store.tsx`)

type Shortcut = { id: string; label: string; date: string }
const dates = (list: Shortcut[]) => Object.fromEntries(list.map((s) => [s.id, s.date]))

const expectA = { today: '2026-10-28', tomorrow: '2026-10-29', eow: '2026-10-30', week: '2026-11-04', 'two-weeks': '2026-11-11', month: '2026-11-27' }
const expectB = { today: '2026-11-01', tomorrow: '2026-11-02', eow: '2026-11-06', week: '2026-11-08', 'two-weeks': '2026-11-15', month: '2026-12-01' }

const gotA = dates(format.dueShortcuts(new Date(2026, 9, 28, 12, 0, 0)))
const gotB = dates(format.dueShortcuts(new Date(2026, 10, 1, 12, 0, 0)))
record(
  'D16',
  'dueShortcuts from 2026-10-28 (range crosses the 2026-11-01 fall-back)',
  JSON.stringify(gotA) === JSON.stringify(expectA),
  `tz=${tz} got=${JSON.stringify(gotA)} want=${JSON.stringify(expectA)}`,
)
record(
  'D16',
  'dueShortcuts from 2026-11-01 (the day of the fall-back)',
  JSON.stringify(gotB) === JSON.stringify(expectB),
  `tz=${tz} got=${JSON.stringify(gotB)} want=${JSON.stringify(expectB)}`,
)

// HEAD formula, verbatim from `git diff web/src/tracker/format.ts`:
const DAY = 86_400_000
const headAdd = (now: Date, days: number) => format.toDateOnly(new Date(new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() + days * DAY))
const headA = headAdd(new Date(2026, 9, 28, 12, 0, 0), 7)
record(
  'D16',
  'pre-fix formula (HEAD `today.getTime() + days*DAY`) lands a day early',
  headA === '2026-11-03' && gotA.week === '2026-11-04',
  `headFormula+7=${headA} fixed+7=${gotA.week} (${headA !== gotA.week ? 'differs, assertion is load-bearing' : 'SAME - assertion not load-bearing'})`,
)

// due-this-week filter: applyFilters reads `new Date()` itself, so the clock is injected for the
// call. Calendar math on 2026-10-28 (Wed, EDT) must include a 2026-11-04 due date; +7*24h must not.
const RealDate = Date
// 00:30 local on 2026-10-28: the first hour of the day is where a +7*24h step lands on 23:30 of
// the previous date once the offset has changed, so this is the instant the filter fix is about.
const FIXED = RealDate.parse('2026-10-28T04:30:00Z')
class FixedDate extends RealDate {
  constructor(...args: any[]) {
    if (args.length === 0) super(FIXED)
    else super(...(args as [any]))
  }
  static now() {
    return FIXED
  }
}
// @ts-ignore - deliberate clock injection for the call under test
globalThis.Date = FixedDate

const mk = (identifier: string, due: string | null) => ({
  id: identifier,
  identifier,
  team_id: 'team-1',
  title: identifier,
  description: '',
  state_id: 'state-backlog',
  priority: 0,
  estimate: null,
  assignee_id: null,
  creator_id: 'me',
  project_id: null,
  milestone_id: null,
  cycle_id: null,
  parent_id: null,
  due_date: due,
  sort_order: 0,
  lane_key: null,
  created_at: '2026-10-01T00:00:00.000Z',
  updated_at: '2026-10-01T00:00:00.000Z',
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
})
const filterIssues = [
  mk('DUE-1002', '2026-10-27'),
  mk('DUE-1003', '2026-10-28'),
  mk('DUE-1103', '2026-11-03'),
  mk('DUE-1104', '2026-11-04'),
  mk('DUE-1105', '2026-11-05'),
  mk('DUE-NONE', null),
]
const fakeState = {
  statesById: new Map([['state-backlog', { id: 'state-backlog', type: 'backlog' }]]),
  me: { id: 'me' },
  cyclesById: new Map(),
}
const weekRows = store.applyFilters(filterIssues as any, { due: 'week' } as any, fakeState as any).map((i: any) => i.due_date)
globalThis.Date = RealDate
const headWeek = format.toDateOnly(new Date(FIXED + 7 * DAY))
record(
  'D16',
  'due-this-week filter includes exactly today..today+7 calendar days',
  JSON.stringify(weekRows) === JSON.stringify(['2026-10-28', '2026-11-03', '2026-11-04']) && headWeek === '2026-11-03',
  `tz=${tz} clock=2026-10-28T04:30:00Z rows=${JSON.stringify(weekRows)} headWeekAhead=${headWeek} fixedWeekAhead=2026-11-04`,
)

const failed = rows.filter((r) => !r.ok)
await Bun.write(`${here}/results-node.json`, JSON.stringify({ tz, rows, failed: failed.length }, null, 2))
console.log(`\n${rows.length} checks, ${rows.length - failed.length} pass, ${failed.length} fail`)
process.exit(failed.length === 0 ? 0 : 1)
