// D18-only re-verification after the web-detail patch. Same harness, same ports, same mock.
// Baseline failures are preserved in results-final.json; this writes results-d18-postfix.json.
const BASE = 'http://127.0.0.1:5299'
const CTL = 'http://127.0.0.1:9131/__ctl'
const OUT = '/Users/indo/code/project/omon-gateway/.omo/evidence/review-20261020/ui-final'

const hard = setTimeout(() => {
  console.error('HARD TIMEOUT 180s')
  process.exit(3)
}, 180_000)
hard.unref?.()

const ctl = async (path: string, init?: RequestInit) => (await fetch(`${CTL}${path}`, init)).json()
const post = (path: string, body: unknown) => ctl(path, { method: 'POST', body: JSON.stringify(body) })
const resetLog = () => ctl('/reset')
const gate = (o: Record<string, string | false>) => post('/gate', o)
const pending = async (): Promise<{ path: string; notes: string[] }[]> => await ctl('/pending')
const release = (path: string) => ctl(`/release?path=${encodeURIComponent(path)}`)
const reqlog = async (): Promise<any[]> => await ctl('/log')
const mockState = async (): Promise<any> => await ctl('/state')

const results: { id: string; check: string; ok: boolean; detail: string; phase: string }[] = []
function record(id: string, check: string, ok: boolean, detail: string) {
  results.push({ id, check, ok, detail, phase: 'postfix' })
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

const webview = new Bun.WebView({ width: 1440, height: 900 })
const PATCH = '/api/tracker/issues/iss-draft'
const DRAFT = 'Draft kept across a failed save'
const retainedDraft = () => webview.evaluate(`import('/src/tracker/detail/descriptionDraft.ts').then(m => m.retainedDraft('iss-draft') ?? null)`)
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
// Types the draft, blurs (Esc), and fails the resulting PATCH; returns the counters for the record.
async function failOneSave(tag: string) {
  await webview.evaluate(`(() => { const el = document.querySelector('[aria-label="Issue preview"] .ld-desc-view'); if (!el) return false; el.click(); return true })()`)
  await waitFor(webview, `!!document.querySelector('textarea.ld-desc-textarea')`, 6000)
  await webview.evaluate(setInput('textarea.ld-desc-textarea', DRAFT))
  await webview.evaluate(key({ key: 'Escape', code: 'Escape' }, `document.querySelector('textarea.ld-desc-textarea')`))
  const held = await waitPending(PATCH, 8000)
  const rel = await release(PATCH)
  const notSaved = await waitFor(webview, `document.querySelector('.ld-desc-status')?.innerText.includes('Not saved') ?? false`, 8000)
  const patches = (await reqlog()).filter((e) => e.method === 'PATCH' && e.path === PATCH)
  return { tag, held, rel, notSaved, patchCount: patches.length }
}

try {
  await ctl('/reset-state', { method: 'POST' })
  await resetLog()

  // ------------------------------------------------ case A (baseline: PASS)
  await go(webview, '/tracker/team/OMO/all')
  await waitFor(webview, `!!document.querySelector('[data-issue-row="OMO-104"]')`, 25000)
  const peekIn = await openPeek()
  await gate({ [PATCH]: 'fail500' })
  const a = await failOneSave('A')
  const keptInEditor = await webview.evaluate(`document.querySelector('textarea.ld-desc-textarea')?.value ?? ''`)
  const retryShown = await webview.evaluate(`[...document.querySelectorAll('.ld-tab')].some(b => b.innerText.trim() === 'Retry')`)
  const peekClosed = await closePeek()
  const retryHeld = await waitPending(PATCH, 6000)
  const retryRel = await release(PATCH)
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
  await shot(webview, 'd18-postfix-a-reopened.png')
  record(
    'D18',
    'case A: a delayed failed save keeps the draft through close/reopen',
    peekIn >= 0 && a.held !== null && a.rel.released >= 1 && a.notSaved >= 0 && retryShown && keptInEditor === DRAFT && peekClosed >= 0 && retryHeld !== null && retryRel.released >= 1 && settledRetained === DRAFT && reopened >= 0 && first.text.includes(DRAFT) && after400.includes(DRAFT) && bars === 1,
    `peekIn=${peekIn}ms patchHeld=${JSON.stringify(a.held?.notes)} released=${a.rel.released} notSavedIn=${a.notSaved}ms retryShown=${retryShown} keptInEditor="${keptInEditor}" closedIn=${peekClosed}ms closeRetryHeld=${retryHeld !== null} closeRetryReleased=${retryRel.released} quiesced=${quiesced} retained@settled=${JSON.stringify(settledRetained)} reopened=${reopened}ms firstFrame="${first.text}" firstBars=${first.bars} after400ms="${after400}" bars=${bars}`,
  )

  // ------------------------------------------------ case B (baseline: FAIL - Retry was a silent no-op)
  await gate({ [PATCH]: false })
  await webview.evaluate(`(() => { document.querySelectorAll('.lui-toast [aria-label="Dismiss"]').forEach(b => b.click()); return true })()`)
  await waitFor(webview, `document.querySelectorAll('.lui-toast').length === 0`, 4000)
  await resetLog()
  const bClick = await webview.evaluate(`(() => { const b = [...document.querySelectorAll('[aria-label="Issue preview"] .ld-desc-pending .ld-tab')].find(x => x.innerText.trim() === 'Retry'); if (!b) return 'no-button'; b.click(); return 'clicked' })()`)
  const bPatch = await waitLog((e) => e.method === 'PATCH' && e.path === PATCH && e.done, 8000)
  const bCleared = await waitFor(webview, `!document.querySelector('[aria-label="Issue preview"] .ld-desc-pending')`, 6000)
  const bStored = (await mockState()).issues.find((i: any) => i.id === 'iss-draft').description
  const bPatches = (await reqlog()).filter((e) => e.method === 'PATCH' && e.path === PATCH)
  const bRetained = await retainedDraft()
  const bToasts = await webview.evaluate(`[...document.querySelectorAll('.lui-toast')].map(t => t.innerText.replace(/\\n/g, ' '))`)
  await shot(webview, 'd18-postfix-b-retry-saved.png')
  record(
    'D18',
    'case B: Retry after recovery saves the retained draft exactly once',
    bClick === 'clicked' && bPatch !== null && bCleared >= 0 && bStored === DRAFT && bPatches.length === 1,
    `click=${bClick} patchCount=${bPatches.length} patch=${bPatch?.note} pendingClearedIn=${bCleared}ms stored="${bStored}" retainedAfterRetry=${JSON.stringify(bRetained)} toasts=${JSON.stringify(bToasts)}`,
  )
  await closePeek()

  // ------------------------------------------------ case C (baseline: FAIL - draft wiped on settle)
  await ctl('/reset-state', { method: 'POST' })
  await resetLog()
  await gate({ [PATCH]: 'fail500' })
  await hashTo(webview, '#/tracker/team/OMO/all')
  await waitFor(webview, `!!document.querySelector('[data-issue-row="OMO-104"]')`, 10000)
  await openPeek()
  const c1 = await failOneSave('C')
  const cClosed = await closePeek()
  const cRetryHeld = await waitPending(PATCH, 6000)
  await openPeek()
  await waitFor(webview, `(${descViewText}).length > 0`, 5000)
  const cFirst = { text: await webview.evaluate(descViewText), retained: await retainedDraft() }
  const cRel = await release(PATCH)
  let cQuiesced = false
  for (let i = 0; i < 60 && !cQuiesced; i++) {
    cQuiesced = (await pending()).length === 0
    if (!cQuiesced) await Bun.sleep(50)
  }
  await Bun.sleep(600)
  const cSettled = { text: await webview.evaluate(descViewText), retained: await retainedDraft(), bars: await webview.evaluate(pendingBar) }
  const cStored = (await mockState()).issues.find((i: any) => i.id === 'iss-draft').description
  await shot(webview, 'd18-postfix-c-reopen-inflight.png')
  record(
    'D18',
    'case C: reopening while the retry PATCH is in flight keeps the draft',
    c1.held !== null && cClosed >= 0 && cRetryHeld !== null && cRel.released >= 1 && cFirst.text.includes(DRAFT) && cSettled.text.includes(DRAFT) && cSettled.retained === DRAFT,
    `firstSavePatch=${c1.patchCount} closedIn=${cClosed}ms retryStillPending=${JSON.stringify(cRetryHeld?.notes)} released=${cRel.released} quiesced=${cQuiesced} firstFrame="${cFirst.text}" retained@first=${JSON.stringify(cFirst.retained)} | afterSettle="${cSettled.text}" retained@settle=${JSON.stringify(cSettled.retained)} bars=${cSettled.bars} stored="${cStored}"`,
  )
  await gate({ [PATCH]: false })
  await closePeek()
} catch (err) {
  record('HARNESS', 'D18 rerun completed without an exception', false, String(err))
  console.error(err)
} finally {
  webview.close()
  await Bun.write(`${OUT}/results-d18-postfix.json`, JSON.stringify(results, null, 1))
  const fails = results.filter((r) => !r.ok).length
  console.log(`\nD18 postfix: ${results.length - fails} pass / ${fails} fail`)
  clearTimeout(hard)
  process.exit(fails === 0 ? 0 : 1)
}
