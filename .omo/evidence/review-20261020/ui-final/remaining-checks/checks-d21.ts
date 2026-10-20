// D21: the `updateWithLaneToast` path (issues/model.ts:168-183), invoked against the *real* mounted
// provider rather than a synthetic drag.
//
//   TZ=America/New_York bun run checks-d21.ts
//
// Why this and not the board drag: a 6-column board plus closestCorners collision made a trusted
// mouse drag land on whichever column was nearest (first "Duplicate", then "In Review"), so a drag
// would prove the harness's aim, not the product. The user asked for the actual code path instead.
//
// The provider value is not exported, so it is read out of the live React tree (fiber walk from
// #root). That is runtime inspection of the mounted app - no source file is edited, and the value
// used is the provider's own, so `updateIssues`, the store's duplicate-toast suppression
// (store.tsx:441-446) and the actionable toast are all the real ones.
const BASE = 'http://127.0.0.1:5399'
const CTL = 'http://127.0.0.1:9231/__ctl'
const OUT = import.meta.dir

const hard = setTimeout(() => {
  console.error('HARD TIMEOUT 180s')
  process.exit(3)
}, 180_000)
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

// Reads the TrackerProvider's context value out of the mounted React tree.
const PROVIDER_PROBE = `
(() => {
  const root = document.getElementById('root')
  if (!root) return { error: 'no #root' }
  const key = Object.keys(root).find((k) => k.startsWith('__reactContainer$') || k.startsWith('__reactFiber$'))
  if (!key) return { error: 'no react fiber key on #root', keys: Object.keys(root).slice(0, 8) }
  let start = root[key]
  // __reactContainer$ points at the container; walk to the first child fiber if needed.
  if (start && start.current) start = start.current
  const seen = new Set()
  const stack = [start]
  let visited = 0
  while (stack.length && visited < 20000) {
    const fiber = stack.pop()
    if (!fiber || seen.has(fiber)) continue
    seen.add(fiber)
    visited += 1
    const props = fiber.memoizedProps
    const value = props && props.value
    if (value && typeof value.updateIssues === 'function' && typeof value.reportError === 'function') {
      window.__d21t = value
      return { ok: true, visited, keys: Object.keys(value).length, hasUpdateWithLaneToastTarget: true }
    }
    if (fiber.child) stack.push(fiber.child)
    if (fiber.sibling) stack.push(fiber.sibling)
  }
  return { error: 'provider value not found', visited }
})()`

const consoleErrors: string[] = []
const view = new Bun.WebView({
  backend: 'chrome',
  width: 1600,
  height: 1000,
  console: (type: string, ...args: any[]) => {
    if (type === 'error') consoleErrors.push(args.map((a) => (typeof a === 'string' ? a : JSON.stringify(a))).join(' '))
  },
})

try {
  await post('/reset')
  await go(view, '/tracker/team/OMO/all')
  const painted = await waitFor(view, `!!document.querySelector('[data-issue-row="OMO-101"]')`, 25000)
  const probe = (await view.evaluate(PROVIDER_PROBE)) as any
  record('D21', 'the mounted app exposes its real TrackerProvider value', probe?.ok === true, `paintedRow=${painted}ms probe=${JSON.stringify(probe)}`)

  if (probe?.ok) {
    const state = await ctl('/state')
    const completedId = state.completed.id
    const laneBefore = state.issues.find((i: any) => i.id === 'iss-lane')

    // 1) the real path: model.updateWithLaneToast -> t.updateIssues -> store suppression -> toast
    await post('/reset')
    const invoke = (await view.evaluate(`
      (async () => {
        const model = await import('/src/tracker/issues/model.ts')
        const t = window.__d21t
        const before = document.querySelectorAll('.lui-toast').length
        const result = await model.updateWithLaneToast(t, ['iss-lane'], { state_id: ${JSON.stringify(completedId)} })
        await new Promise((r) => setTimeout(r, 400))
        const toasts = [...document.querySelectorAll('.lui-toast')].map((el) => ({
          text: el.innerText.replace(/\\n/g, ' '),
          role: el.getAttribute('role'),
        }))
        return { exported: typeof model.updateWithLaneToast, before, result, toasts, lastError: t.error ?? null }
      })()`)) as any
    const rejected = await waitLog(
      (e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-lane' && e.done,
      8000,
    )
    const after = (await ctl('/state')).issues.find((i: any) => i.id === 'iss-lane')
    await Bun.write(`${OUT}/d21-shared-store-toast.png`, await view.screenshot())
    record(
      'D21',
      'updateWithLaneToast on a lane_not_verified 409 raises exactly one actionable toast',
      invoke?.exported === 'function' &&
        invoke?.result === false &&
        invoke?.before === 0 &&
        invoke?.toasts?.length === 1 &&
        invoke.toasts[0].text.includes('lane must be verified') &&
        invoke.toasts[0].text.includes('Force close') &&
        invoke.toasts[0].role === 'alert' &&
        rejected?.note?.includes(completedId) &&
        after?.state_id === laneBefore.state_id,
      `toasts=${JSON.stringify(invoke?.toasts)} result=${invoke?.result} patch=${JSON.stringify(rejected?.note)} stateUnchanged=${after?.state_id === laneBefore.state_id}`,
    )

    // 1b) the store's own duplicate suppression is load-bearing. Calling t.updateIssues directly
    //     on the same 409 must raise NO toast (store.tsx:441-446 skips reportError for
    //     lane_not_verified and rethrows); combined with 1) that is exactly one toast, not two.
    await post('/reset')
    await view.evaluate(`(() => { document.querySelectorAll('.lui-toast [aria-label="Dismiss"]').forEach((b) => b.click()); return true })()`)
    const cleared = await waitFor(view, `document.querySelectorAll('.lui-toast').length === 0`, 5000)
    const direct = (await view.evaluate(`
      (async () => {
        const t = window.__d21t
        let error = null
        try {
          await t.updateIssues(['iss-lane'], { state_id: ${JSON.stringify(completedId)} })
        } catch (e) {
          error = { name: e?.constructor?.name, code: e?.code, status: e?.status, message: e?.message }
        }
        await new Promise((r) => setTimeout(r, 400))
        return { error, toasts: document.querySelectorAll('.lui-toast').length }
      })()`)) as any
    const directPatch = await waitLog((e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-lane' && e.done, 8000)
    record(
      'D21',
      'the store suppresses its own generic toast for lane_not_verified (one toast, not two)',
      cleared >= 0 && directPatch !== null && direct?.error?.code === 'lane_not_verified' && direct?.toasts === 0,
      `clearedIn=${cleared}ms rethrown=${JSON.stringify(direct?.error)} toastsAfterDirectUpdate=${direct?.toasts}`,
    )

    // 2) the actionable toast's own action still completes the close with force_note, once
    await post('/reset')
    await view.evaluate(`
      (async () => {
        const model = await import('/src/tracker/issues/model.ts')
        await model.updateWithLaneToast(window.__d21t, ['iss-lane'], { state_id: ${JSON.stringify(completedId)} })
        return true
      })()`)
    await waitFor(view, `!!document.querySelector('.lui-toast')`, 8000)
    await post('/reset')
    const forced = (await view.evaluate(`
      (async () => {
        const b = [...document.querySelectorAll('.lui-toast button')].find((x) => x.innerText.trim() === 'Force close')
        if (!b) return { clicked: false }
        b.click()
        await new Promise((r) => setTimeout(r, 800))
        return { clicked: true, toasts: document.querySelectorAll('.lui-toast').length }
      })()`)) as any
    const forcedPatch = await waitLog(
      (e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-lane' && e.done && e.note?.includes('force_note'),
      8000,
    )
    const stored = (await ctl('/state')).issues.find((i: any) => i.id === 'iss-lane')
    record(
      'D21',
      'Force close from that toast retries once with force_note and raises no second toast',
      forced?.clicked === true && forcedPatch !== null && stored?.state_id === completedId && forced?.toasts === 0,
      `patch=${JSON.stringify(forcedPatch?.note)} storedState=${stored?.state_id} toastsAfter=${forced?.toasts}`,
    )
  }

  record('ALL', 'no page console errors during the run', consoleErrors.length === 0, `consoleErrors=${JSON.stringify(consoleErrors.slice(0, 5))}`)
} catch (err) {
  record('RUN', 'harness completed without throwing', false, String(err))
}

const failed = rows.filter((r) => !r.ok)
await Bun.write(`${OUT}/results-d21.json`, JSON.stringify({ rows, failed: failed.length }, null, 2))
console.log(`\n${rows.length} checks, ${rows.length - failed.length} pass, ${failed.length} fail`)
process.exit(failed.length === 0 ? 0 : 1)
