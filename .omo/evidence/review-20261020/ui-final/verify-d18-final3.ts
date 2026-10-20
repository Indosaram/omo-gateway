// D18 final replay: A / B / C, EACH ON A FRESH PAGE, single server owner.
//
// Consumes the implementation reply "createUnmountFlush StrictMode patch ready": the tree is
// fingerprinted before and after the run so the verdicts are attributable to one revision.
//
// Each case builds its own state from a brand-new JS context (close the page, open a new one), so no
// case can inherit the previous case's client store or retained draft:
//   type draft -> gate PATCH to 500 -> blur (save held) -> release (Not saved)
//   -> close the peek (unmount flush fires, held) -> reopen -> case-specific assertion
//
// Emits the authoritative result to results-d18-postfix.json (the subscribed file). The previous
// content of that file is preserved as results-d18-postfix.superseded-1720.json.
const BASE = 'http://127.0.0.1:5299'
const CTL = 'http://127.0.0.1:9131/__ctl'
const OUT = '/Users/indo/code/project/omon-gateway/.omo/evidence/review-20261020/ui-final'
const STAMP = new Date().toISOString().replace(/[:.]/g, '-')
const TOKEN = `st_01a1582e-${process.pid}-${STAMP}`
const TASK_PREFIX = 'st_01a1582e-'
const PATCH = '/api/tracker/issues/iss-draft'
const DRAFT = 'Draft kept across a failed save'

const hard = setTimeout(() => {
  console.error('HARD TIMEOUT 260s')
  process.exit(3)
}, 260_000)
hard.unref?.()

const ctl = async (path: string, init?: RequestInit) => (await fetch(`${CTL}${path}`, init)).json()
const post = (path: string, body: unknown) => ctl(path, { method: 'POST', body: JSON.stringify(body) })
const pending = async (): Promise<{ path: string; notes: string[] }[]> => await ctl('/pending')
const reqlog = async (): Promise<any[]> => await ctl('/log')
const mockState = async (): Promise<any> => await ctl('/state')

const results: any[] = []
const record = (id: string, check: string, ok: boolean, detail: string, phase = 'final') => {
  results.push({ id, check, ok, detail, phase })
  console.log(`${ok ? 'PASS' : 'FAIL'} ${id} ${check} :: ${detail}`)
}
function setupFail(check: string, detail: string): never {
  record('SETUP', check, false, detail, 'setup')
  throw new Error(`setup: ${check}`)
}
const bounded = async <T,>(label: string, ms: number, fn: () => Promise<T>): Promise<T> => {
  let t: any
  const g = new Promise<never>((_, rej) => (t = setTimeout(() => rej(new Error(`timeout ${label}`)), ms)))
  try {
    return await Promise.race([fn(), g])
  } finally {
    clearTimeout(t)
  }
}
const TREE_FILES = [
  '/Users/indo/code/project/omon-gateway/web/src/tracker/detail/descriptionDraft.ts',
  '/Users/indo/code/project/omon-gateway/web/src/tracker/detail/Editors.tsx',
]
async function treeFingerprint() {
  const out: Record<string, string> = {}
  for (const f of TREE_FILES) {
    const text = await Bun.file(f).text()
    out[f.split('/').pop()!] = `${text.length}:${Bun.hash(text).toString(16)}`
  }
  return out
}
const waitPending = async (path: string, ms: number) => {
  const t0 = Date.now()
  while (Date.now() - t0 < ms) {
    const h = (await pending()).find((x) => x.path === path)
    if (h) return h
    await Bun.sleep(25)
  }
  return null
}
const waitLog = async (pred: (e: any) => boolean, ms: number) => {
  const t0 = Date.now()
  while (Date.now() - t0 < ms) {
    const h = (await reqlog()).find(pred)
    if (h) return h
    await Bun.sleep(25)
  }
  return null
}

let page: any = null
let nav = 0
async function freshPage(label: string) {
  try {
    page?.close()
  } catch {}
  page = new Bun.WebView({ width: 1440, height: 900 })
  nav += 1
  await bounded(`nav ${label}`, 25000, () => page.navigate(`${BASE}/?n=${nav}#/tracker/team/OMO/all`))
  const t = await waitFor(`!!document.querySelector('[data-issue-row="OMO-104"]')`, 25000)
  if (t < 0) setupFail(`${label}: rows render on a fresh page`, `no OMO-104 row`)
  return page
}
async function waitFor(expr: string, ms: number) {
  const t0 = Date.now()
  while (Date.now() - t0 < ms) {
    if (await bounded('eval', 8000, () => page.evaluate(expr))) return Date.now() - t0
    await Bun.sleep(25)
  }
  return -1
}
const setInput = (sel: string, val: string) => `
  (() => {
    const el = document.querySelector(${JSON.stringify(sel)})
    if (!el) return false
    const proto = el.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype
    Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, ${JSON.stringify(val)})
    el.dispatchEvent(new Event('input', { bubbles: true }))
    return true
  })()`
const key = (init: any, target = 'document.activeElement || document.body') => `
  (() => { const el = ${target}; (el || document.body).dispatchEvent(new KeyboardEvent('keydown', Object.assign({ bubbles: true, cancelable: true }, ${JSON.stringify(init)}))); return true })()`
const hover = (ident: string) => `
  (() => { const r = document.querySelector('[data-issue-row=${JSON.stringify(ident)}]'); if (!r) return false; r.dispatchEvent(new MouseEvent('mousemove', { bubbles: true })); return true })()`
const DESC = `document.querySelector('[aria-label="Issue preview"] .ld-desc-view')?.innerText.trim() ?? ''`
const BARS = `document.querySelectorAll('[aria-label="Issue preview"] .ld-desc-pending').length`

async function openPeek() {
  await page.evaluate(hover('OMO-104'))
  await waitFor(`!!document.querySelector('.lic-row--focused')`, 5000)
  await page.evaluate(key({ key: ' ', code: 'Space' }))
  return waitFor(`!!document.querySelector('[aria-label="Issue preview"] .ld-desc-view')`, 8000)
}
async function closePeek() {
  await page.evaluate(`(() => { const b = document.querySelector('[aria-label="Issue preview"] [aria-label="Close"]'); if (!b) return false; b.click(); return true })()`)
  return waitFor(`!document.querySelector('[aria-label="Issue preview"]')`, 6000)
}
// Builds the exact state all three cases start from: a reopened peek holding a draft whose save
// failed, with the close-triggered retry either released (settled) or still held (in flight).
async function buildFailedDraft(label: string, keepRetryInFlight: boolean) {
  const opened = await openPeek()
  if (opened < 0) setupFail(`${label}: peek opens`, `openPeek=${opened}`)
  await page.evaluate(`(() => { const el = document.querySelector('[aria-label="Issue preview"] .ld-desc-view'); if (!el) return false; el.click(); return true })()`)
  const ta = await waitFor(`!!document.querySelector('textarea.ld-desc-textarea')`, 6000)
  if (ta < 0) setupFail(`${label}: editor opens`, `textarea=${ta}`)
  await page.evaluate(setInput('textarea.ld-desc-textarea', DRAFT))
  await post('/gate', { [PATCH]: 'fail500' })
  await page.evaluate(key({ key: 'Escape', code: 'Escape' }, `document.querySelector('textarea.ld-desc-textarea')`))
  const firstHeld = await waitPending(PATCH, 8000)
  const firstRel = await ctl(`/release?path=${encodeURIComponent(PATCH)}`)
  const notSaved = await waitFor(`document.querySelector('.ld-desc-status')?.innerText.includes('Not saved') ?? false`, 8000)
  const closed = await closePeek()
  const retryHeld = await waitPending(PATCH, 6000)
  let retryRel: any = null
  let quiesced = false
  if (keepRetryInFlight) {
    // leave it pending: case C measures the reopen while the write is in flight
  } else {
    retryRel = await ctl(`/release?path=${encodeURIComponent(PATCH)}`)
    for (let i = 0; i < 80; i++) {
      if ((await pending()).length === 0) {
        quiesced = true
        break
      }
      await Bun.sleep(50)
    }
  }
  const reopened = await openPeek()
  if (reopened < 0) setupFail(`${label}: peek reopens`, `reopen=${reopened}`)
  await waitFor(`(${DESC}).length > 0`, 5000)
  return { opened, ta, firstHeld, firstRel, notSaved, closed, retryHeld, retryRel, quiesced, reopened }
}

let treeStart: Record<string, string> = {}
try {
  // ---- single-owner prerequisites -----------------------------------------
  const ping = await bounded('ping', 4000, () => fetch(`${CTL}/ping`).then((r) => r.json()))
  if (ping?.mock !== 'ui-final-d18') setupFail('mock on 9131 is this harness', `ping=${JSON.stringify(ping)}`)
  let claim = await bounded('claim', 4000, () => post('/claim', { token: TOKEN }))
  if (!claim.ok) {
    const owner: string = claim.owner?.token ?? ''
    if (!owner.startsWith(TASK_PREFIX)) setupFail('not owned by another harness', `claim=${JSON.stringify(claim)}`)
    if ((await pending()).length > 0) setupFail('stale own claim is idle', 'pending non-empty')
    await post('/unclaim', { token: owner })
    claim = await bounded('claim', 4000, () => post('/claim', { token: TOKEN }))
    if (!claim.ok) setupFail('claim after clearing own stale claim', `claim=${JSON.stringify(claim)}`)
  }
  if ((await pending()).length > 0) setupFail('no foreign in-flight requests', 'pending non-empty')
  const proxied = await bounded('proxy', 10000, () => fetch(`${BASE}/api/tracker/issues?include_archived=1`).then((r) => r.json()))
  if (!(proxied.items ?? []).some((i: any) => i.id === 'iss-draft')) setupFail('vite 5299 proxies to this mock', 'iss-draft missing')
  if ((await ctl('/owner')).owner.token !== TOKEN) setupFail('ownership retained', 'owner mismatch')
  treeStart = await treeFingerprint()

  // ============================ case A (fresh page) ============================
  await ctl('/reset-state', { method: 'POST' })
  await ctl('/reset', { method: 'POST' })
  await freshPage('case A')
  const A = await buildFailedDraft('case A', false)
  const aFirst = { text: await page.evaluate(DESC), bars: await page.evaluate(BARS) }
  await Bun.sleep(400)
  const aAfter = { text: await page.evaluate(DESC), bars: await page.evaluate(BARS) }
  await Bun.write(`${OUT}/d18-final3-${STAMP}-a.png`, await page.screenshot())
  record(
    'D18',
    'case A (fresh page): a delayed failed save keeps the draft through close/reopen',
    A.firstHeld !== null && A.firstRel.released >= 1 && A.notSaved >= 0 && A.closed >= 0 && A.retryHeld !== null && A.retryRel.released >= 1 && A.quiesced && aFirst.text.includes(DRAFT) && aFirst.bars === 1 && aAfter.text.includes(DRAFT) && aAfter.bars === 1,
    `freshPage=true firstHeld=${JSON.stringify(A.firstHeld?.notes)} released=${A.firstRel.released} notSavedIn=${A.notSaved}ms closedIn=${A.closed}ms retryHeld=${A.retryHeld !== null} retryReleased=${A.retryRel?.released} quiesced=${A.quiesced} reopenedIn=${A.reopened}ms firstFrame="${aFirst.text}" firstBars=${aFirst.bars} after400="${aAfter.text}" bars=${aAfter.bars}`,
  )
  await closePeek()

  // ============================ case B (fresh page) ============================
  await ctl('/reset-state', { method: 'POST' })
  await ctl('/reset', { method: 'POST' })
  await freshPage('case B')
  const B = await buildFailedDraft('case B', false)
  const bBefore = { text: await page.evaluate(DESC), bars: await page.evaluate(BARS) }
  await post('/gate', { [PATCH]: false })
  await page.evaluate(`(() => { document.querySelectorAll('.lui-toast [aria-label="Dismiss"]').forEach(b => b.click()); return true })()`)
  await waitFor(`document.querySelectorAll('.lui-toast').length === 0`, 4000)
  await ctl('/reset')
  await page.evaluate(`
    (() => {
      window.__t = []
      const btn = [...document.querySelectorAll('.ld-desc-pending .ld-tab')].find(b => b.innerText.trim() === 'Retry')
      btn.addEventListener('click', (e) => window.__t.push({ k: 'click', trusted: e.isTrusted }), true)
      const of = window.fetch
      window.fetch = function (...a) { window.__t.push({ k: 'fetch', m: (a[1] && a[1].method) || 'GET', u: String(a[0]) }); return of.apply(this, a) }
      return true
    })()`)
  // native (trusted) input, not element.click()
  await bounded('trusted click', 15000, () => page.click('[aria-label="Issue preview"] .ld-desc-pending .ld-tab'))
  const bPatch = await waitLog((e) => e.method === 'PATCH' && e.path === PATCH && e.done, 8000)
  const bCleared = await waitFor(`!document.querySelector('[aria-label="Issue preview"] .ld-desc-pending')`, 6000)
  const bStored = (await mockState()).issues.find((i: any) => i.id === 'iss-draft').description
  const bPatches = (await reqlog()).filter((e) => e.method === 'PATCH' && e.path === PATCH)
  const bTrace = await page.evaluate(`window.__t ?? []`)
  const bTrusted = bTrace.some((t: any) => t.k === 'click' && t.trusted === true)
  await Bun.write(`${OUT}/d18-final3-${STAMP}-b.png`, await page.screenshot())
  record(
    'D18',
    'case B (fresh page, trusted click): Retry saves the retained draft exactly once',
    B.firstHeld !== null && bBefore.bars === 1 && bTrusted && bPatch !== null && bPatches.length === 1 && bCleared >= 0 && bStored === DRAFT,
    `freshPage=true barsBeforeClick=${bBefore.bars} viewBefore="${bBefore.text}" trustedClick=${bTrusted} patchCount=${bPatches.length} patch=${bPatch?.note} barClearedIn=${bCleared}ms stored="${bStored}" trace=${JSON.stringify(bTrace)}`,
  )
  await closePeek()

  // ============================ case C (fresh page) ============================
  await ctl('/reset-state', { method: 'POST' })
  await ctl('/reset', { method: 'POST' })
  await freshPage('case C')
  const C = await buildFailedDraft('case C', true)
  const cFirst = { text: await page.evaluate(DESC), bars: await page.evaluate(BARS) }
  const cRel = await ctl(`/release?path=${encodeURIComponent(PATCH)}`)
  let cQuiesced = false
  for (let i = 0; i < 80; i++) {
    if ((await pending()).length === 0) {
      cQuiesced = true
      break
    }
    await Bun.sleep(50)
  }
  await Bun.sleep(500)
  const cAfter = { text: await page.evaluate(DESC), bars: await page.evaluate(BARS) }
  const cStored = (await mockState()).issues.find((i: any) => i.id === 'iss-draft').description
  await Bun.write(`${OUT}/d18-final3-${STAMP}-c.png`, await page.screenshot())
  record(
    'D18',
    'case C (fresh page): reopening while the retry PATCH is in flight keeps the draft',
    C.firstHeld !== null && C.retryHeld !== null && C.reopened >= 0 && cFirst.text.includes(DRAFT) && cRel.released >= 1 && cQuiesced && cAfter.text.includes(DRAFT) && cAfter.bars === 1,
    `freshPage=true retryStillInFlightAtReopen=${JSON.stringify(C.retryHeld?.notes)} firstFrame="${cFirst.text}" firstBars=${cFirst.bars} releasedAfterReopen=${cRel.released} quiesced=${cQuiesced} afterSettle="${cAfter.text}" bars=${cAfter.bars} stored="${cStored}"`,
  )
  await post('/gate', { [PATCH]: false })
  await closePeek()

  // ---- attribution ---------------------------------------------------------
  const treeEnd = await treeFingerprint()
  const moved = Object.keys(treeStart).filter((k) => treeStart[k] !== treeEnd[k])
  if (moved.length > 0) setupFail('tree unchanged during the run', `changed=${JSON.stringify(moved)}`)
  record('SETUP', 'tree unchanged during the run', true, JSON.stringify(treeEnd), 'setup-ok')
} catch (err) {
  if (!results.some((r) => r.phase === 'setup')) record('HARNESS', 'run completed without an exception', false, String(err), 'harness')
  console.error(err)
} finally {
  try {
    page?.close()
  } catch {}
  const setup = results.filter((r) => r.phase === 'setup')
  const fails = results.filter((r) => r.phase !== 'setup' && r.phase !== 'setup-ok' && !r.ok).length
  const payload = {
    authoritative: setup.length === 0,
    stamp: STAMP,
    token: TOKEN,
    item: 'D18',
    mode: 'fresh page per case, single server owner',
    tree: treeStart,
    verdict: setup.length ? 'SETUP FAILED - no verdict' : fails === 0 ? 'PASS (A, B, C)' : `${fails} FAIL`,
    results,
  }
  await Bun.write(`${OUT}/results-d18-final3-${STAMP}.json`, JSON.stringify(payload, null, 1))
  // the subscribed file is only rewritten with a valid, fully-attributed run
  if (setup.length === 0) {
    const prev = Bun.file(`${OUT}/results-d18-postfix.json`)
    if (await prev.exists()) await Bun.write(`${OUT}/results-d18-postfix.superseded-1720.json`, prev)
    await Bun.write(`${OUT}/results-d18-postfix.json`, JSON.stringify(payload, null, 1))
    console.log('emitted -> results-d18-postfix.json (previous content preserved as .superseded-1720.json)')
  } else {
    console.log('NOT emitted to results-d18-postfix.json (setup failed) - kept in results-d18-final3 file')
  }
  console.log(setup.length ? `SETUP FAILED: ${setup.map((s) => s.check).join('; ')}` : `D18 final: ${fails} fail`)
  clearTimeout(hard)
  process.exit(setup.length ? 4 : fails === 0 ? 0 : 1)
}
