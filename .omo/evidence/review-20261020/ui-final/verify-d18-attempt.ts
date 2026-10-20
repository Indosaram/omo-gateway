// D18 replay with port ownership + fail-fast prerequisites + a DOM event trace.
//
// A previous attempt produced a meaningless result because a second harness was driving the same
// mock (9131) and Vite (5299): the log was reset, a foreign PATCH appeared, and the pending bar the
// case needs was gone. This runner refuses to start unless it owns the mock, and re-checks ownership
// before and after every case, so a collision becomes a SETUP failure instead of a fake verdict.
//
// Writes a unique file per attempt: results-d18-attempt-<stamp>.json. Never overwrites earlier
// evidence.
const BASE = 'http://127.0.0.1:5299'
const CTL = 'http://127.0.0.1:9131/__ctl'
const OUT = '/Users/indo/code/project/omon-gateway/.omo/evidence/review-20261020/ui-final'
const STAMP = new Date().toISOString().replace(/[:.]/g, '-')
const TOKEN = `st_01a1582e-${process.pid}-${STAMP}`
const TASK_PREFIX = 'st_01a1582e-'

const hard = setTimeout(() => {
  console.error('HARD TIMEOUT 200s')
  process.exit(3)
}, 200_000)
hard.unref?.()

const ctl = async (path: string, init?: RequestInit) => (await fetch(`${CTL}${path}`, init)).json()
const post = (path: string, body: unknown) => ctl(path, { method: 'POST', body: JSON.stringify(body) })
const pending = async (): Promise<{ path: string; notes: string[] }[]> => await ctl('/pending')
const release = (path: string) => ctl(`/release?path=${encodeURIComponent(path)}`)
const reqlog = async (): Promise<any[]> => await ctl('/log')
const mockState = async (): Promise<any> => await ctl('/state')

const results: { id: string; check: string; ok: boolean; detail: string; phase: string }[] = []
function record(id: string, check: string, ok: boolean, detail: string, phase = 'attempt') {
  results.push({ id, check, ok, detail, phase })
  console.log(`${ok ? 'PASS' : 'FAIL'} ${id} ${check} :: ${detail}`)
}
function setupFail(check: string, detail: string) {
  record('SETUP', check, false, detail, 'setup')
  console.error(`SETUP FAILURE: ${check} :: ${detail}`)
  throw new Error(`setup: ${check}`)
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

// ---------------------------------------------------------------- tree fingerprint
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

// ---------------------------------------------------------------- prerequisites
async function assertPrerequisites() {
  // 1. the mock answers and is the harness mock, not something else on the port
  const ping = await bounded('ping', 4000, () => fetch(`${CTL}/ping`).then((r) => r.json()))
  if (ping?.mock !== 'ui-final-d18') setupFail('mock on 9131 is this harness', `ping=${JSON.stringify(ping)}`)

  // 2. claim it; a live foreign claim means another harness is mid-run
  let claim = await bounded('claim', 4000, () => post('/claim', { token: TOKEN }))
  if (!claim.ok) {
    const ownerToken: string = claim.owner?.token ?? ''
    const ownStale = ownerToken.startsWith(TASK_PREFIX)
    if (!ownStale) setupFail('mock is not owned by another harness', `claim=${JSON.stringify(claim)}`)
    const p = await pending()
    if (p.length > 0) setupFail('stale own claim has no live requests', `pending=${JSON.stringify(p)}`)
    await bounded('unclaim', 4000, () => post('/unclaim', { token: ownerToken }))
    claim = await bounded('claim', 4000, () => post('/claim', { token: TOKEN }))
    if (!claim.ok) setupFail('claim after clearing own stale claim', `claim=${JSON.stringify(claim)}`)
  }

  // 3. no foreign in-flight requests
  const p0 = await pending()
  if (p0.length > 0) setupFail('no foreign in-flight requests', `pending=${JSON.stringify(p0)}`)

  // 4. our seeded state, not somebody else's
  const st = await mockState()
  const ids = (st.issues ?? []).map((i: any) => i.id)
  const needed = ['iss-sub', 'iss-created', 'iss-lane', 'iss-draft', 'iss-500']
  if (!needed.every((n) => ids.includes(n)) || st.scenario !== 'mixed') {
    setupFail('mock holds this harness seed', `scenario=${st.scenario} ids=${JSON.stringify(ids)}`)
  }

  // 5. Vite on 5299 really proxies to this mock (fingerprint the seeded issues through the proxy)
  const proxied = await bounded('proxy', 10000, () =>
    fetch(`${BASE}/api/tracker/issues?include_archived=1`).then((r) => r.json()),
  )
  const proxiedIds = (proxied.items ?? []).map((i: any) => i.id)
  if (!needed.every((n) => proxiedIds.includes(n))) {
    setupFail('vite 5299 proxies to this mock', `proxiedIds=${JSON.stringify(proxiedIds)}`)
  }

  // 6. ownership still ours after the probes
  const owner = await ctl('/owner')
  if (owner.owner.token !== TOKEN) setupFail('ownership retained', `owner=${JSON.stringify(owner)}`)

  // 7. the tree the run will attribute its verdicts to
  const tree = await treeFingerprint()
  return tree
}

async function assertStillOwner(where: string) {
  const owner = await ctl('/owner')
  if (owner.owner.token !== TOKEN) setupFail(`ownership at ${where}`, `owner=${JSON.stringify(owner)}`)
  const p = await pending()
  const foreign = p.filter((x) => !x.notes.some((n) => n.startsWith('probe')))
  if (p.length > 2) setupFail(`no request pile-up at ${where}`, `pending=${JSON.stringify(p)} foreign=${JSON.stringify(foreign)}`)
}

// ---------------------------------------------------------------- helpers
let navCount = 0
const go = async (v: any, hash: string) => {
  navCount += 1
  await bounded(`navigate ${hash}`, 25000, () => v.navigate(`${BASE}/?n=${navCount}#${hash}`))
}
const hashTo = async (v: any, hash: string) =>
  await bounded('hash', 8000, () => v.evaluate(`(() => { window.location.hash = ${JSON.stringify(hash)}; return true })()`))

async function waitFor(view: any, expr: string, ms: number) {
  const t0 = Date.now()
  while (Date.now() - t0 < ms) {
    if (await bounded('evaluate', 8000, () => view.evaluate(expr))) return Date.now() - t0
    await Bun.sleep(25)
  }
  return -1
}
async function waitPending(path: string, ms: number) {
  const t0 = Date.now()
  while (Date.now() - t0 < ms) {
    const hit = (await pending()).find((x) => x.path === path)
    if (hit) return hit
    await Bun.sleep(25)
  }
  return null
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
const setInput = (selector: string, value: string) => `
  (() => {
    const el = document.querySelector(${JSON.stringify(selector)})
    if (!el) return false
    const proto = el.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype
    Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, ${JSON.stringify(value)})
    el.dispatchEvent(new Event('input', { bubbles: true }))
    return true
  })()`
const key = (init: Record<string, unknown>, target = 'document.activeElement || document.body') => `
  (() => {
    const el = ${target}
    ;(el || document.body).dispatchEvent(new KeyboardEvent('keydown', Object.assign({ bubbles: true, cancelable: true }, ${JSON.stringify(init)})))
    return true
  })()`
const hoverRow = (ident: string) => `
  (() => {
    const row = document.querySelector('[data-issue-row=${JSON.stringify(ident)}]')
    if (!row) return false
    row.dispatchEvent(new MouseEvent('mousemove', { bubbles: true }))
    return true
  })()`

const PATCH = '/api/tracker/issues/iss-draft'
const DRAFT = 'Draft kept across a failed save'
let webview: any = new Bun.WebView({ width: 1440, height: 900 })
// A fresh page is a fresh JS context: module-level draft retention does NOT survive it, which is
// exactly what case C needs after case B has saved and mutated the client store.
async function freshPage(label: string) {
  if (webview) {
    try {
      webview.close()
    } catch {}
  }
  webview = new Bun.WebView({ width: 1440, height: 900 })
  navCount += 1
  await bounded(`navigate ${label}`, 25000, () => webview.navigate(`${BASE}/?n=${navCount}#/tracker/team/OMO/all`))
  return webview
}
// Best-effort observation only: after an HMR edit Vite can serve this module under a `?t=` URL, so
// this dynamic import may be a SECOND module instance whose map is empty. Verdicts therefore never
// depend on it; the user-visible DOM is the assertion.
const retainedDraft = () => webview.evaluate(`import('/src/tracker/detail/descriptionDraft.ts').then(m => m.retainedDraft('iss-draft') ?? null)`)
const moduleUrls = () =>
  webview.evaluate(`performance.getEntriesByType('resource').map(e => e.name).filter(n => n.includes('descriptionDraft'))`)
const descViewText = `document.querySelector('[aria-label="Issue preview"] .ld-desc-view')?.innerText.trim() ?? ''`
const pendingBar = `document.querySelectorAll('[aria-label="Issue preview"] .ld-desc-pending').length`

const openPeek = async () => {
  await webview.evaluate(hoverRow('OMO-104'))
  await waitFor(webview, `!!document.querySelector('.lic-row--focused')`, 5000)
  await webview.evaluate(key({ key: ' ', code: 'Space' }))
  return waitFor(webview, `!!document.querySelector('[aria-label="Issue preview"] .ld-desc-view')`, 8000)
}
const closePeek = async () => {
  await webview.evaluate(`(() => { const b = document.querySelector('[aria-label="Issue preview"] [aria-label="Close"]'); if (!b) return false; b.click(); return true })()`)
  return waitFor(webview, `!document.querySelector('[aria-label="Issue preview"]')`, 6000)
}

// Transient instrumentation: what the browser really delivers to the Retry button, whether the
// handler runs, and whether anything crosses the save() boundary (a fetch/PATCH).
const traceScript = (marker: string) => `
  (() => {
    const log = (window.__trace = window.__trace || [])
    const t0 = performance.now()
    const push = (k, extra) => log.push(Object.assign({ t: Math.round(performance.now() - t0), k }, extra || {}))
    const btn = [...document.querySelectorAll('.ld-desc-pending .ld-tab')].find(b => b.innerText.trim() === 'Retry')
    if (!btn) { push('no-retry-node'); return 'no-retry-node' }
    const r = btn.getBoundingClientRect()
    const cx = Math.round(r.left + r.width / 2), cy = Math.round(r.top + r.height / 2)
    const hit = document.elementFromPoint(cx, cy)
    push('node', { tag: btn.tagName, cls: btn.className, rect: [Math.round(r.left), Math.round(r.top), Math.round(r.width), Math.round(r.height)], disabled: btn.disabled, pointerEvents: getComputedStyle(btn).pointerEvents })
    push('hit-test', { at: [cx, cy], hitTag: hit?.tagName ?? null, hitCls: hit?.className ?? null, hitIsButton: hit === btn || btn.contains(hit) })
    const parent = btn.parentElement
    push('parent', { tag: parent?.tagName ?? null, cls: parent?.className ?? null, parentPointerEvents: parent ? getComputedStyle(parent).pointerEvents : null })
    for (const type of ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click']) {
      btn.addEventListener(type, (e) => push('listener:' + type, { trusted: e.isTrusted, target: e.target?.tagName ?? null }), true)
    }
    // Every fetch the page makes after this point: the save() boundary shows up here.
    const of = window.fetch
    window.fetch = function (...a) {
      const u = String(a[0]); const m = (a[1] && a[1].method) || 'GET'
      if (u.includes('/issues/')) push('fetch', { m, u })
      return of.apply(this, a)
    }
    const bar = document.querySelector('.ld-desc-pending')
    if (bar) new MutationObserver(() => push('bar-mutation', { bars: document.querySelectorAll('.ld-desc-pending').length })).observe(bar.parentElement, { childList: true, subtree: true })
    return 'instrumented'
  })()`

let treeStart: Record<string, string> = {}
try {
  treeStart = await assertPrerequisites()

  // ------------------------------------------------ case A (regression guard)
  await ctl('/reset-state', { method: 'POST' })
  await ctl('/reset', { method: 'POST' })
  await assertStillOwner('case A start')
  await freshPage('case A')
  await waitFor(webview, `!!document.querySelector('[data-issue-row="OMO-104"]')`, 25000)
  const peekIn = await openPeek()
  if (peekIn < 0) setupFail('case A: peek opens', `peekIn=${peekIn}`)
  await webview.evaluate(`(() => { const el = document.querySelector('[aria-label="Issue preview"] .ld-desc-view'); if (!el) return false; el.click(); return true })()`)
  const textareaIn = await waitFor(webview, `!!document.querySelector('textarea.ld-desc-textarea')`, 6000)
  if (textareaIn < 0) setupFail('case A: editor opens', `textareaIn=${textareaIn}`)
  await webview.evaluate(setInput('textarea.ld-desc-textarea', DRAFT))
  await post('/gate', { [PATCH]: 'fail500' })
  await webview.evaluate(key({ key: 'Escape', code: 'Escape' }, `document.querySelector('textarea.ld-desc-textarea')`))
  const aHeld = await waitPending(PATCH, 8000)
  const aRel = await release(PATCH)
  const aNotSaved = await waitFor(webview, `document.querySelector('.ld-desc-status')?.innerText.includes('Not saved') ?? false`, 8000)
  const keptInEditor = await webview.evaluate(`document.querySelector('textarea.ld-desc-textarea')?.value ?? ''`)
  const retryShown = await webview.evaluate(`[...document.querySelectorAll('.ld-tab')].some(b => b.innerText.trim() === 'Retry')`)
  const closedIn = await closePeek()
  const aRetryHeld = await waitPending(PATCH, 6000)
  const aRetryRel = await release(PATCH)
  let quiesced = false
  for (let i = 0; i < 60 && !quiesced; i++) {
    quiesced = (await pending()).length === 0
    if (!quiesced) await Bun.sleep(50)
  }
  const settledRetained = await retainedDraft()
  const reopened = await openPeek()
  await waitFor(webview, `(${descViewText}).length > 0`, 5000)
  const first = { text: await webview.evaluate(descViewText), bars: await webview.evaluate(pendingBar) }
  await Bun.sleep(400)
  const after400 = await webview.evaluate(descViewText)
  const bars = await webview.evaluate(pendingBar)
  await assertStillOwner('case A end')
  await shot(webview, `d18-attempt-${STAMP}-a-reopened.png`)
  record(
    'D18',
    'case A: a delayed failed save keeps the draft through close/reopen',
    aHeld !== null && aRel.released >= 1 && aNotSaved >= 0 && retryShown && keptInEditor === DRAFT && closedIn >= 0 && aRetryHeld !== null && aRetryRel.released >= 1 && reopened >= 0 && first.text.includes(DRAFT) && after400.includes(DRAFT) && bars === 1,
    `peekIn=${peekIn}ms held=${JSON.stringify(aHeld?.notes)} released=${aRel.released} notSavedIn=${aNotSaved}ms retryShown=${retryShown} keptInEditor="${keptInEditor}" closedIn=${closedIn}ms closeRetryHeld=${aRetryHeld !== null} quiesced=${quiesced} [observation: retainedDraft@settled=${JSON.stringify(settledRetained)} moduleUrls=${JSON.stringify(await moduleUrls())}] reopened=${reopened}ms firstFrame="${first.text}" firstBars=${first.bars} after400ms="${after400}" bars=${bars}`,
  )

  // ------------------------------------------------ case B (the open failure) + trace
  await post('/gate', { [PATCH]: false })
  await webview.evaluate(`(() => { document.querySelectorAll('.lui-toast [aria-label="Dismiss"]').forEach(b => b.click()); return true })()`)
  await waitFor(webview, `document.querySelectorAll('.lui-toast').length === 0`, 4000)
  await ctl('/reset')
  const instr = await webview.evaluate(traceScript(STAMP))
  const bClick = await webview.evaluate(`(() => { const b = [...document.querySelectorAll('.ld-desc-pending .ld-tab')].find(x => x.innerText.trim() === 'Retry'); if (!b) return 'no-button'; b.click(); return 'clicked' })()`)
  const bPatch = await waitLog((e) => e.method === 'PATCH' && e.path === PATCH && e.done, 6000)
  const bCleared = await waitFor(webview, `!document.querySelector('[aria-label="Issue preview"] .ld-desc-pending')`, 4000)
  const bStored = (await mockState()).issues.find((i: any) => i.id === 'iss-draft').description
  const bPatches = (await reqlog()).filter((e) => e.method === 'PATCH' && e.path === PATCH)
  const bRetained = await retainedDraft()
  const trace = await webview.evaluate(`window.__trace ?? []`)
  await assertStillOwner('case B end')
  await shot(webview, `d18-attempt-${STAMP}-b-retry.png`)
  record(
    'D18',
    'case B: Retry after recovery saves the retained draft exactly once',
    instr === 'instrumented' && bClick === 'clicked' && bPatch !== null && bCleared >= 0 && bStored === DRAFT && bPatches.length === 1,
    `instr=${instr} click=${bClick} patchCount=${bPatches.length} patch=${bPatch?.note} pendingClearedIn=${bCleared}ms stored="${bStored}" retainedAfterRetry=${JSON.stringify(bRetained)}`,
  )
  record('D18-TRACE', 'event trace for the Retry click', true, JSON.stringify(trace))
  await closePeek()
  webview.close()
  webview = null

  // ------------------------------------------------ case C
  await ctl('/reset-state', { method: 'POST' })
  await ctl('/reset')
  await post('/gate', { [PATCH]: 'fail500' })
  await assertStillOwner('case C start')
  // Fresh page: case B saved the draft, so the client store no longer holds the pre-write text and
  // a mock reset alone would not put the page back into the state this case needs.
  await freshPage('case C')
  await waitFor(webview, `!!document.querySelector('[data-issue-row="OMO-104"]')`, 10000)
  const cOpen = await openPeek()
  if (cOpen < 0) setupFail('case C: peek opens', `cOpen=${cOpen}`)
  await webview.evaluate(`(() => { const el = document.querySelector('[aria-label="Issue preview"] .ld-desc-view'); if (!el) return false; el.click(); return true })()`)
  await waitFor(webview, `!!document.querySelector('textarea.ld-desc-textarea')`, 6000)
  await webview.evaluate(setInput('textarea.ld-desc-textarea', DRAFT))
  await webview.evaluate(key({ key: 'Escape', code: 'Escape' }, `document.querySelector('textarea.ld-desc-textarea')`))
  const cHeld = await waitPending(PATCH, 8000)
  const cRel1 = await release(PATCH)
  await waitFor(webview, `document.querySelector('.ld-desc-status')?.innerText.includes('Not saved') ?? false`, 8000)
  const cClosed = await closePeek()
  const cRetryHeld = await waitPending(PATCH, 6000)
  const cReopen = await openPeek()
  await waitFor(webview, `(${descViewText}).length > 0`, 5000)
  const cFirst = { text: await webview.evaluate(descViewText), retained: await retainedDraft() }
  const cRel2 = await release(PATCH)
  let cQuiesced = false
  for (let i = 0; i < 60 && !cQuiesced; i++) {
    cQuiesced = (await pending()).length === 0
    if (!cQuiesced) await Bun.sleep(50)
  }
  await Bun.sleep(600)
  const cSettled = { text: await webview.evaluate(descViewText), retained: await retainedDraft(), bars: await webview.evaluate(pendingBar) }
  await assertStillOwner('case C end')
  await shot(webview, `d18-attempt-${STAMP}-c-reopen-inflight.png`)
  record(
    'D18',
    'case C: reopening while the retry PATCH is in flight keeps the draft',
    cHeld !== null && cRel1.released >= 1 && cClosed >= 0 && cRetryHeld !== null && cReopen >= 0 && cRel2.released >= 1 && cFirst.text.includes(DRAFT) && cSettled.text.includes(DRAFT) && cSettled.bars === 1,
    `held=${JSON.stringify(cHeld?.notes)} released=${cRel1.released} closedIn=${cClosed}ms retryHeld=${JSON.stringify(cRetryHeld?.notes)} reopened=${cReopen}ms released2=${cRel2.released} quiesced=${cQuiesced} firstFrame="${cFirst.text}" [observation: retainedDraft@first=${JSON.stringify(cFirst.retained)}] | afterSettle="${cSettled.text}" bars=${cSettled.bars} [observation: retainedDraft@settle=${JSON.stringify(cSettled.retained)}]`,
  )
  await post('/gate', { [PATCH]: false })
  await closePeek()

  // The verdicts are only attributable to one tree revision if nothing moved while we ran.
  const treeEnd = await treeFingerprint()
  const moved = Object.keys(treeStart).filter((k) => treeStart[k] !== treeEnd[k])
  if (moved.length > 0) {
    setupFail('tree unchanged during the run', `changed=${JSON.stringify(moved)} start=${JSON.stringify(treeStart)} end=${JSON.stringify(treeEnd)}`)
  }
  record('SETUP', 'tree unchanged during the run', true, `fingerprints=${JSON.stringify(treeEnd)}`, 'setup-ok')
} catch (err) {
  if (!results.some((r) => r.phase === 'setup')) record('HARNESS', 'run completed without an exception', false, String(err), 'harness')
  console.error(err)
} finally {
  try {
    webview?.close()
  } catch {}
  const file = `${OUT}/results-d18-attempt-${STAMP}.json`
  await Bun.write(
    file,
    JSON.stringify({ token: TOKEN, stamp: STAMP, tree: treeStart, setupPassed: !results.some((r) => r.phase === 'setup'), results }, null, 1),
  )
  const setup = results.filter((r) => r.phase === 'setup')
  const fails = results.filter((r) => r.phase !== 'setup' && r.phase !== 'setup-ok' && !r.ok).length
  console.log(`\nwrote ${file}`)
  console.log(setup.length ? `SETUP FAILED - no verdict: ${setup.map((s) => s.check).join('; ')}` : `D18 attempt: ${fails} fail`)
  clearTimeout(hard)
  process.exit(setup.length ? 4 : fails === 0 ? 0 : 1)
}
