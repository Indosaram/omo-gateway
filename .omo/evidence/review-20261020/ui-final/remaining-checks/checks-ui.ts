// Browser checks for D17 / D20 / D21 / D22, driven through the real tracker DOM.
//
//   TZ=America/New_York bun run checks-ui.ts
//
// Prerequisites (started by run.sh, both local and disposable):
//   bun run mock.ts                     -> http://127.0.0.1:9231 (mock tracker API)
//   bunx vite --config vite.config.mts  -> http://127.0.0.1:5399 (web/src served from the frozen tree)
//
// Every wait is deadline-bounded and waits for the exact state it asserts; the only fixed value in
// the file is the clock override used by D22, which is the behaviour under test.
const BASE = 'http://127.0.0.1:5399'
const CTL = 'http://127.0.0.1:9231/__ctl'
const OUT = import.meta.dir

const hard = setTimeout(() => {
  console.error('HARD TIMEOUT 300s')
  process.exit(3)
}, 300_000)
hard.unref?.()

const ctl = async (path: string, init?: RequestInit) => (await fetch(`${CTL}${path}`, init)).json()
const post = (path: string, body: unknown = {}) => ctl(path, { method: 'POST', body: JSON.stringify(body) })
const reqlog = async (): Promise<any[]> => await ctl('/log')

type Row = { id: string; check: string; ok: boolean; detail: string }
const rows: Row[] = []
function record(id: string, check: string, ok: boolean, detail: string) {
  rows.push({ id, check, ok, detail })
  console.log(`${ok ? 'PASS' : 'FAIL'} ${id} ${check} :: ${detail}`)
}

async function bounded<T>(label: string, ms: number, fn: () => Promise<T>): Promise<T> {
  let timer: any
  const guard = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error(`timeout ${label} ${ms}ms`)), ms)
  })
  try {
    return await Promise.race([fn(), guard])
  } finally {
    clearTimeout(timer)
  }
}

let navCount = 0
async function go(view: any, hash: string) {
  navCount += 1
  await bounded(`navigate ${hash}`, 25000, () => view.navigate(`${BASE}/?n=${navCount}#${hash}`))
}

async function waitFor(view: any, expr: string, ms: number) {
  const t0 = Date.now()
  while (Date.now() - t0 < ms) {
    if (await bounded('evaluate', 8000, () => view.evaluate(expr))) return Date.now() - t0
    await Bun.sleep(25)
  }
  return -1
}

async function waitLog(pred: (e: any) => boolean, ms: number) {
  const t0 = Date.now()
  while (Date.now() - t0 < ms) {
    const hit = (await reqlog()).find(pred)
    if (hit) return hit
    await Bun.sleep(25)
  }
  return null
}

const shot = async (view: any, name: string) => Bun.write(`${OUT}/${name}`, await view.screenshot())

const consoleErrors: string[] = []
const view = new Bun.WebView({
  backend: 'chrome',
  width: 1440,
  height: 900,
  console: (type: string, ...args: any[]) => {
    if (type === 'error') consoleErrors.push(args.map((a) => (typeof a === 'string' ? a : JSON.stringify(a))).join(' '))
  },
})

try {
  // ================================================================= D17
  // What the backend accepts, per lane status, from src/lanes/cli.rs, src/lanes/state.rs and
  // src/storage/lanes.rs::request_close:
  //   send   -> cli.rs:217-221  only working|blocked|not_done
  //   object -> change_claim(false) targets NotDone (cli.rs:697-704); state.rs:61 NotDone is
  //             reachable only from done_claimed, and from blocked the target becomes working
  //   close  -> storage/lanes.rs:190-195  verified_done for a plain close; force close is accepted
  //             from every status except `closed`, and execute_lane_action (tracker/lanes.rs:620-632)
  //             composes the required `forced:` note
  //   reopen -> cli.rs:272-278  only closed|failed
  const BACKEND_ACCEPTED: Record<string, string[]> = {
    queued: ['close'],
    working: ['send', 'close'],
    blocked: ['send', 'object', 'close'],
    done_claimed: ['object', 'close'],
    not_done: ['send', 'close'],
    verified_done: ['close'],
    closed: ['reopen'],
    failed: ['close', 'reopen'],
  }
  // The table as it is at HEAD (`git show HEAD:web/src/tracker/detail/AgentPanel.tsx`), used only
  // to show that the comparison below is load-bearing.
  const HEAD_ALLOWED: Record<string, string[]> = {
    queued: ['send', 'close'],
    working: ['send', 'close'],
    blocked: ['send', 'close'],
    done_claimed: ['object', 'send'],
    not_done: ['send', 'close'],
    verified_done: ['close', 'object'],
    closed: ['reopen'],
    failed: ['reopen'],
  }
  const LABEL_TO_ACTION: Record<string, string> = {
    'Send message': 'send',
    Object: 'object',
    'Close lane': 'close',
    Reopen: 'reopen',
  }
  const violations = (offered: Record<string, string[]>) =>
    Object.entries(offered).flatMap(([status, actions]) =>
      actions.filter((a) => !(BACKEND_ACCEPTED[status] ?? []).includes(a)).map((a) => `${status}:${a}`),
    )

  const offered: Record<string, string[]> = {}
  const statuses = Object.keys(BACKEND_ACCEPTED)
  for (const status of statuses) {
    await post('/lane', { status })
    await go(view, '/tracker/issue/OMO-101')
    const painted = await waitFor(view, `document.querySelectorAll('.ld-agent-actions button').length > 0`, 20000)
    const labels = (await view.evaluate(
      `[...document.querySelectorAll('.ld-agent-actions button')].map(b => b.innerText.trim())`,
    )) as string[]
    offered[status] = labels.map((l) => LABEL_TO_ACTION[l] ?? `unknown(${l})`)
    if (painted < 0) offered[status] = ['<panel did not paint>']
  }
  const domViolations = violations(offered)
  const headViolations = violations(HEAD_ALLOWED)
  record(
    'D17',
    'lane panel offers only actions the backend accepts, for all 8 lane statuses',
    domViolations.length === 0 && Object.values(offered).every((a) => a.length > 0) && headViolations.length === 3,
    `offered=${JSON.stringify(offered)} violations=${JSON.stringify(domViolations)} headTableViolations=${JSON.stringify(headViolations)}`,
  )

  // ================================================================= D20
  // shortenPath: SessionAgentPanel.tsx:31-34. A path under any user's home must shorten; the
  // HEAD version matched only /Users/indo.
  const cwdCases = ['/Users/alice/code/omon', '/home/ci/work', '/opt/omon/app', '/Users']
  const headRegex = /^\/Users\/indo(?=\/|$)/
  const shortened: { cwd: string; shown: string; title: string }[] = []
  for (const cwd of cwdCases) {
    await post('/cwd', { cwd })
    await go(view, '/tracker/issue/OMO-102')
    const painted = await waitFor(view, `!!document.querySelector('.ld-agent-grid dd.mono[title]')`, 20000)
    const shown = await view.evaluate(`
      (() => {
        const dt = [...document.querySelectorAll('.ld-agent-grid dt')].find(d => d.innerText.trim() === 'Directory')
        const dd = dt?.nextElementSibling
        return dd ? { text: dd.innerText.trim(), title: dd.getAttribute('title') } : null
      })()`)
    shortened.push({ cwd, shown: painted >= 0 ? (shown as any)?.text ?? '<not painted>' : '<not painted>', title: (shown as any)?.title ?? '' })
  }
  const expectShown = ['~/code/omon', '~/work', '/opt/omon/app', '/Users']
  const headShown = cwdCases.map((c) => c.replace(headRegex, '~'))
  record(
    'D20',
    'session panel shortens any user home, keeps other absolute paths and the full path in title',
    shortened.every((s, i) => s.shown === expectShown[i] && s.title === s.cwd) &&
      headShown[0] === '/Users/alice/code/omon' &&
      headShown[1] === '/home/ci/work',
    `shown=${JSON.stringify(shortened)} headRegexWould=${JSON.stringify(headShown)}`,
  )

  // ================================================================= D21
  // model.ts:168-183 updateWithLaneToast: a lane_not_verified 409 must produce exactly one toast,
  // the actionable one. The store's duplicate (store.tsx:441-446) is suppressed. The other caller of
  // that suppression (pickers.tsx useApply) is already covered by ui-final/results-final.json D27;
  // this drives the board-move caller, which D27 never touched.
  await post('/reset')
  await go(view, '/tracker/team/OMO/all')
  await waitFor(view, `!!document.querySelector('[data-issue-row="OMO-101"]')`, 20000)
  await view.evaluate(`(() => { const b = document.querySelector('[data-layout="board"]'); if (!b) return false; b.click(); return true })()`)
  const boardIn = await waitFor(view, `!!document.querySelector('[data-issue-card="OMO-101"]')`, 15000)
  const target = (await ctl('/state')).completed
  // Bring the Done column into view first: releasing outside the viewport makes closestCorners pick
  // whichever column happens to be nearest, which is how the first attempt landed on "Duplicate".
  await view.evaluate(`(() => { const c = document.querySelector('[data-board-column=${JSON.stringify(target.id)}]'); if (!c) return false; c.scrollIntoView({ block: 'nearest', inline: 'center' }); return true })()`)
  await Bun.sleep(300)
  const rects = await view.evaluate(`
    (() => {
      const card = document.querySelector('[data-issue-card="OMO-101"]')
      const col = document.querySelector('[data-board-column=${JSON.stringify(target.id)}]')
      if (!card || !col) return null
      const a = card.getBoundingClientRect(), b = col.getBoundingClientRect()
      const vw = window.innerWidth, vh = window.innerHeight
      return {
        card: { x: a.x + a.width / 2, y: a.y + a.height / 2 },
        col: { x: b.x + b.width / 2, y: b.y + Math.min(b.height / 2, 120) },
        inViewport: a.x >= 0 && a.y >= 0 && a.right <= vw && a.bottom <= vh && b.x >= 0 && b.right <= vw,
      }
    })()`)
  let dragged = false
  if (rects) {
    const { card, col, inViewport } = rects as any
    dragged = inViewport === true
    // trusted input: the same mechanism as ui-final/verify-d18-b-trusted.ts
    await view.cdp('Input.dispatchMouseEvent', { type: 'mousePressed', x: card.x, y: card.y, button: 'left', clickCount: 1 })
    await view.cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: card.x + 12, y: card.y + 6, button: 'left', buttons: 1 })
    for (const f of [0.25, 0.5, 0.75, 1]) {
      await view.cdp('Input.dispatchMouseEvent', {
        type: 'mouseMoved',
        x: Math.round(card.x + (col.x - card.x) * f),
        y: Math.round(card.y + (col.y - card.y) * f),
        button: 'left',
        buttons: 1,
      })
      await Bun.sleep(60)
    }
    await view.cdp('Input.dispatchMouseEvent', { type: 'mouseReleased', x: col.x, y: col.y, button: 'left', clickCount: 1 })
  }
  const rejected = await waitLog((e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-lane' && e.done, 12000)
  const toastIn = await waitFor(view, `!!document.querySelector('.lui-toast')`, 8000)
  const toasts = (await view.evaluate(
    `[...document.querySelectorAll('.lui-toast')].map(t => ({ text: t.innerText.replace(/\\n/g, ' '), role: t.getAttribute('role') }))`,
  )) as any[]
  await shot(view, 'd21-board-move-one-toast.png')
  record(
    'D21',
    'board move into a completed state: lane_not_verified shows exactly one actionable toast',
    boardIn >= 0 && dragged && rejected !== null && rejected?.note?.includes(target.id) && toastIn >= 0 && toasts.length === 1 && toasts[0].text.includes('Force close'),
    `boardIn=${boardIn}ms dragged=${dragged} doneColumn=${target.id} patch=${JSON.stringify(rejected?.note)} toastIn=${toastIn}ms toasts=${JSON.stringify(toasts)}`,
  )

  // ================================================================= D22
  // TimelineView.tsx:59-64 steps by calendar day. The browser clock is the machine's (Asia/Seoul, no
  // DST), so the timezone is overridden to America/New_York - the zone whose fall-back on
  // 2026-11-01 falls inside the timeline's today-7 .. today+28 window.
  await view.cdp('Emulation.setTimezoneOverride', { timezoneId: 'America/New_York' })
  await go(view, '/tracker/team/OMO/all')
  await waitFor(view, `!!document.querySelector('.lic-root, .lic-toolbar, [data-layout-current]')`, 20000)
  const tzApplied = await view.evaluate(`Intl.DateTimeFormat().resolvedOptions().timeZone`)
  await view.evaluate(`localStorage.setItem('tracker.timeline.team:OMO:all', 'true')`)
  await go(view, '/tracker/team/OMO/all')
  const tlIn = await waitFor(view, `document.querySelectorAll('.lic-tl-day').length > 30`, 20000)
  const ticks = (await view.evaluate(`[...document.querySelectorAll('.lic-tl-day')].map(d => d.innerText.trim())`)) as string[]
  const months = (await view.evaluate(`[...document.querySelectorAll('.lic-tl-month')].map(d => d.innerText.trim())`)) as string[]
  await shot(view, 'd22-timeline-dst-ny.png')
  // Expected calendar sequence, computed in the page (so it uses the same clock and timezone):
  const expected = (await view.evaluate(`
    (() => {
      const DAY = 86400000
      const startOfDay = (ms) => { const d = new Date(ms); d.setHours(0,0,0,0); return d.getTime() }
      const today = startOfDay(Date.now())
      const rangeStart = startOfDay(today - 7 * DAY - 3 * DAY)
      const days = Math.round((startOfDay(today + 28 * DAY + 7 * DAY) - rangeStart) / DAY)
      const out = []
      for (let i = 0; i < days; i++) { const d = new Date(rangeStart); d.setDate(d.getDate() + i); out.push(String(d.getDate())) }
      return out
    })()`)) as string[]
  // The HEAD formula, for the same range: `new Date(rangeStart + i * DAY)`.
  const headTicks = (await view.evaluate(`
    (() => {
      const DAY = 86400000
      const startOfDay = (ms) => { const d = new Date(ms); d.setHours(0,0,0,0); return d.getTime() }
      const today = startOfDay(Date.now())
      const rangeStart = startOfDay(today - 7 * DAY - 3 * DAY)
      const days = Math.round((startOfDay(today + 28 * DAY + 7 * DAY) - rangeStart) / DAY)
      const out = []
      for (let i = 0; i < days; i++) out.push(String(new Date(rangeStart + i * DAY).getDate()))
      return out
    })()`)) as string[]
  const firstMismatch = ticks.findIndex((v, i) => v !== expected[i])
  const headMismatch = headTicks.findIndex((v, i) => v !== expected[i])
  record(
    'D22',
    'timeline day labels are consecutive calendar days across the 2026-11-01 fall-back',
    tzApplied === 'America/New_York' && tlIn >= 0 && ticks.length === expected.length && firstMismatch === -1 && headMismatch >= 0,
    `tz=${tzApplied} ticks=${ticks.length} months=${JSON.stringify(months)} firstMismatch=${firstMismatch} headFormulaFirstMismatch=${headMismatch} headTicksAroundMismatch=${JSON.stringify(headTicks.slice(Math.max(0, headMismatch - 2), headMismatch + 2))} ticksAround=${JSON.stringify(ticks.slice(Math.max(0, headMismatch - 2), headMismatch + 2))}`,
  )

  record('ALL', 'no page console errors during the run', consoleErrors.length === 0, `consoleErrors=${JSON.stringify(consoleErrors.slice(0, 5))}`)
} catch (err) {
  record('RUN', 'harness completed without throwing', false, String(err))
}

const failed = rows.filter((r) => !r.ok)
await Bun.write(`${OUT}/results-ui.json`, JSON.stringify({ base: BASE, rows, failed: failed.length }, null, 2))
console.log(`\n${rows.length} checks, ${rows.length - failed.length} pass, ${failed.length} fail`)
process.exit(failed.length === 0 ? 0 : 1)
