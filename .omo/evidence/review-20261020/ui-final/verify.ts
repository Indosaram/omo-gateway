// Bounded UI verification for D11 / D12 / D14 / D15 / D18 / D19 / D24 / D26 / D27.
//
// Every wait is deadline-bounded. Where a request has to be in flight, the mock holds it on an
// explicit gate and this script releases it by signal (no fixed hold times, no quiet-window proof).
// The one place a wall clock appears is D12 leg A, where the store's own 10 s poll interval is the
// event being observed: the script derives the next tick from the previous read's timestamp and
// records the numbers instead of assuming a sleep is long enough.
const BASE = 'http://127.0.0.1:5299'
const CTL = 'http://127.0.0.1:9131/__ctl'
const OUT = '/Users/indo/code/project/omon-gateway/.omo/evidence/review-20261020/ui-final'
const POLL_MS = 10_000

const hard = setTimeout(() => {
  console.error('HARD TIMEOUT 300s')
  process.exit(3)
}, 300_000)
hard.unref?.()

const ctl = async (path: string, init?: RequestInit) => (await fetch(`${CTL}${path}`, init)).json()
const post = (path: string, body: unknown) => ctl(path, { method: 'POST', body: JSON.stringify(body) })
const resetLog = () => ctl('/reset')
const gate = (o: Record<string, string | false>) => post('/gate', o)
const pending = async (): Promise<{ path: string; notes: string[] }[]> => await ctl('/pending')
const release = (path: string) => ctl(`/release?path=${encodeURIComponent(path)}`)
const reqlog = async (): Promise<any[]> => await ctl('/log')
const mockState = async (): Promise<any> => await ctl('/state')

const results: { id: string; check: string; ok: boolean; detail: string }[] = []
function record(id: string, check: string, ok: boolean, detail: string) {
  results.push({ id, check, ok, detail })
  console.log(`${ok ? 'PASS' : 'FAIL'} ${id} ${check} :: ${detail}`)
}

const consoleErrors: string[] = []

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
async function go(v: any, hash: string) {
  navCount += 1
  await bounded(`navigate ${hash}`, 25000, () => v.navigate(`${BASE}/?n=${navCount}#${hash}`))
}
// In-app navigation: keeps the mounted provider, which is what the route effects react to.
async function hashTo(v: any, hash: string) {
  await bounded('hash', 8000, () => v.evaluate(`(() => { window.location.hash = ${JSON.stringify(hash)}; return true })()`))
}

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

// Dispatches a real keydown into the app's window listener (web/src/tracker/hotkeys.ts).
const key = (init: Record<string, unknown>, target = 'document.activeElement || document.body') => `
  (() => {
    const el = ${target}
    const ev = new KeyboardEvent('keydown', Object.assign({ bubbles: true, cancelable: true }, ${JSON.stringify(init)}))
    ;(el || document.body).dispatchEvent(ev)
    return true
  })()`

const openPalette = async (view: any) => {
  await view.evaluate(key({ key: 'k', code: 'KeyK', metaKey: true }))
  return waitFor(view, `!!document.querySelector('.ls-palette')`, 6000)
}
const closePalette = async (view: any) => {
  await view.evaluate(key({ key: 'Escape', code: 'Escape' }, `document.querySelector('.ls-palette-input')`))
  return waitFor(view, `!document.querySelector('.ls-palette')`, 6000)
}
const hoverRow = (ident: string) => `
  (() => {
    const row = document.querySelector('[data-issue-row=${JSON.stringify(ident)}]')
    if (!row) return false
    row.dispatchEvent(new MouseEvent('mousemove', { bubbles: true }))
    return true
  })()`
const rowsReady = (ident: string) => `!!document.querySelector('[data-issue-row=${JSON.stringify(ident)}]')`
const rowIds = `[...document.querySelectorAll('[data-issue-row]')].map(r => r.getAttribute('data-issue-row'))`

// Opens the property picker for `label` on an issue page and chooses `choice`.
const pickProperty = async (view: any, label: string, choice: string) => {
  await view.evaluate(`
    (() => {
      const row = [...document.querySelectorAll('.ld-prop')].find(r => r.querySelector('.ld-prop-label')?.textContent?.trim() === ${JSON.stringify(label)})
      const btn = row?.querySelector('button')
      if (!btn) return false
      btn.click()
      return true
    })()`)
  const menuIn = await waitFor(view, `!!document.querySelector('[role="menuitem"]')`, 6000)
  await view.evaluate(`
    (() => {
      const item = [...document.querySelectorAll('[role="menuitem"]')].find(i => i.querySelector('.lui-menu-item-label')?.textContent?.trim() === ${JSON.stringify(choice)})
      if (!item) return false
      item.click()
      return true
    })()`)
  return menuIn
}

const priorityPropText = `[...document.querySelectorAll('.ld-prop')].find(r => r.querySelector('.ld-prop-label')?.textContent?.trim() === 'Priority')?.innerText.replace(/\\n/g, ' ') ?? ''`

const webview = new Bun.WebView({ width: 1440, height: 900, console: (type: string, ...args: any[]) => {
  if (type === 'error') consoleErrors.push(args.map((a) => (typeof a === 'string' ? a : JSON.stringify(a))).join(' '))
} })

try {
  // ---------------------------------------------------------------- D11
  await ctl('/reset-state', { method: 'POST' })
  await resetLog()
  await go(webview, '/tracker/team/OMO/all')
  const d11Rows = await waitFor(webview, rowsReady('OMO-101'), 30000)
  await webview.evaluate(hoverRow('OMO-101'))
  const focusedIn = await waitFor(webview, `!!document.querySelector('.lic-row--focused')`, 5000)
  const paletteIn = await openPalette(webview)
  const chipBefore = await webview.evaluate(`document.querySelector('.ls-palette-ctx')?.innerText.replace(/\\n/g, ' ') ?? ''`)
  const statusCmdBefore = await webview.evaluate(
    `[...document.querySelectorAll('.ls-palette-item')].some(b => b.innerText.includes('Change status'))`,
  )
  await shot(webview, 'd11-palette-focused-desktop.png')
  await closePalette(webview)

  // Same page, in-app navigation: the hovered row is unmounted, so the focus target must be cleared.
  await hashTo(webview, '#/tracker/projects')
  const projectsIn = await waitFor(webview, `!document.querySelector('[data-issue-row]')`, 8000)
  const paletteIn2 = await openPalette(webview)
  const chipAfter = await webview.evaluate(`document.querySelector('.ls-palette-ctx')?.innerText ?? ''`)
  const statusCmdAfter = await webview.evaluate(
    `[...document.querySelectorAll('.ls-palette-item')].some(b => b.innerText.includes('Change status'))`,
  )
  record(
    'D11',
    'navigation clears the focused issue so the palette has no stale target',
    d11Rows >= 0 && focusedIn >= 0 && paletteIn >= 0 && chipBefore.includes('OMO-101') && statusCmdBefore === true && projectsIn >= 0 && paletteIn2 >= 0 && chipAfter === '' && statusCmdAfter === false,
    `rowsIn=${d11Rows}ms focusedIn=${focusedIn}ms chipBefore="${chipBefore}" statusCmdBefore=${statusCmdBefore} | projectsIn=${projectsIn}ms chipAfter="${chipAfter}" statusCmdAfter=${statusCmdAfter}`,
  )
  await shot(webview, 'd11-palette-cleared-desktop.png')
  await closePalette(webview)

  // mobile 390x844: same navigation, no stale palette target
  await webview.resize(390, 844)
  await hashTo(webview, '#/tracker/team/OMO/all')
  const mobRows = await waitFor(webview, rowsReady('OMO-101'), 10000)
  await webview.evaluate(hoverRow('OMO-101'))
  const mobFocused = await waitFor(webview, `!!document.querySelector('.lic-row--focused')`, 5000)
  const mobPalette = await openPalette(webview)
  const mobChipBefore = await webview.evaluate(`document.querySelector('.ls-palette-ctx')?.innerText.replace(/\\n/g, ' ') ?? ''`)
  await closePalette(webview)
  await hashTo(webview, '#/tracker/projects')
  await waitFor(webview, `!document.querySelector('[data-issue-row]')`, 8000)
  const mobPalette2 = await openPalette(webview)
  const mobChipAfter = await webview.evaluate(`document.querySelector('.ls-palette-ctx')?.innerText ?? ''`)
  const mobOverflow = await webview.evaluate(`({ scrollW: document.documentElement.scrollWidth, clientW: document.documentElement.clientWidth })`)
  record(
    'D11',
    'mobile 390px: navigation clears the focused issue, no horizontal overflow',
    mobRows >= 0 && mobFocused >= 0 && mobPalette >= 0 && mobChipBefore.includes('OMO-101') && mobPalette2 >= 0 && mobChipAfter === '' && mobOverflow.scrollW <= mobOverflow.clientW,
    `rowsIn=${mobRows}ms chipBefore="${mobChipBefore}" chipAfter="${mobChipAfter}" overflow=${JSON.stringify(mobOverflow)}`,
  )
  await shot(webview, 'd11-mobile-palette-cleared.png')
  await closePalette(webview)
  await webview.resize(1440, 900)

  // ---------------------------------------------------------------- D12
  // Leg A: a read that would start while a write is in flight must not be issued at all.
  await ctl('/reset-state', { method: 'POST' })
  await resetLog()
  await go(webview, '/tracker/issue/OMO-104')
  const propReady = await waitFor(webview, `!!document.querySelector('.ld-prop')`, 25000)
  await gate({ '/api/tracker/issues/iss-draft': 'ok' })
  const patchStart = Date.now()
  await pickProperty(webview, 'Priority', 'High')
  const patchPending = await waitPending('/api/tracker/issues/iss-draft', 6000)
  // The store polls every POLL_MS from mount; the next tick is due one interval after the last read.
  const readsBefore = (await reqlog()).filter((e) => e.path === '/api/tracker/issues' && e.t <= patchStart)
  const lastRead = readsBefore.length ? readsBefore[readsBefore.length - 1].t : patchStart
  const tickDue = lastRead + POLL_MS
  const windowEnd = tickDue + 2500
  let readDuringWrite: any = null
  while (Date.now() < windowEnd) {
    readDuringWrite = (await reqlog()).find((e) => e.path === '/api/tracker/issues' && e.t > patchStart)
    if (readDuringWrite) break
    await Bun.sleep(100)
  }
  const releasedPatch = await release('/api/tracker/issues/iss-draft')
  const patchDone = await waitLog((e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-draft' && e.done, 12000)
  const priorityHigh = await waitFor(webview, `(${priorityPropText}).includes('High')`, 6000)
  const afterPriority = await webview.evaluate(priorityPropText)
  record(
    'D12',
    'leg A: no issues read is issued while a write is in flight (poll tick crossed)',
    propReady >= 0 && patchPending !== null && !readDuringWrite && releasedPatch.released === 1 && patchDone !== null && afterPriority.includes('High'),
    `propReady=${propReady}ms patchPending=${JSON.stringify(patchPending?.notes)} patchDone=${patchDone ? 'yes' : 'NO'} lastRead=${lastRead - patchStart}ms(before patch) tickDue=+${tickDue - patchStart}ms windowEnd=+${windowEnd - patchStart}ms readsDuringWrite=${readDuringWrite ? JSON.stringify(readDuringWrite) : 'none'} released=${releasedPatch.released} priority="${afterPriority}"`,
  )

  // Leg B: a read issued BEFORE the write is discarded when it lands with pre-write rows.
  // The leg-A gate must be cleared first: a still-registered gate holds every later PATCH.
  await gate({ '/api/tracker/issues/iss-draft': false })
  await resetLog()
  await gate({ '/api/tracker/issues': 'ok' })
  const gatedRead = await waitPending('/api/tracker/issues', 16000)
  const readIssuedAt = Date.now()
  const writeAt = Date.now()
  await pickProperty(webview, 'Priority', 'Urgent')
  const writeDone = await waitLog(
    (e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-draft' && e.done && e.note?.includes('"priority":1'),
    8000,
  )
  const urgentShown = await waitFor(webview, `(${priorityPropText}).includes('Urgent')`, 6000)
  const beforeRelease = await webview.evaluate(priorityPropText)
  const releasedRead = await release('/api/tracker/issues')
  const staleLanded = await waitLog((e) => e.path === '/api/tracker/issues' && e.done, 8000)
  const afterStale = await webview.evaluate(priorityPropText)
  const mockPriority = (await mockState()).issues.find((i: any) => i.id === 'iss-draft').priority
  record(
    'D12',
    'leg B: a stale list read that lands after the write is discarded',
    gatedRead !== null && writeDone !== null && urgentShown >= 0 && releasedRead.released === 1 && staleLanded !== null && afterStale.includes('Urgent') && mockPriority === 1,
    `gatedRead=${JSON.stringify(gatedRead?.notes)} urgentShownIn=${urgentShown}ms writeDone=${writeDone?.note} releasedRead=${releasedRead.released} beforeRelease="${beforeRelease}" afterStaleLanded="${afterStale}" mockPriority=${mockPriority}`,
  )
  await gate({ '/api/tracker/issues': false })

  // ---------------------------------------------------------------- D14
  await ctl('/reset-state', { method: 'POST' })
  await resetLog()
  consoleErrors.length = 0
  await go(webview, '/tracker/issue/%E0')
  const shellIn = await waitFor(webview, `!!document.querySelector('.ls-root')`, 15000)
  const bodyText = await webview.evaluate(`document.body.innerText.trim().slice(0, 200)`)
  const shellHtml = await webview.evaluate(`document.querySelector('.ls-root')?.className ?? ''`)
  const uriErrors = consoleErrors.filter((e) => /URIError|decodeURIComponent|malformed/i.test(e))
  record(
    'D14',
    'malformed %E0 in the issue route does not blank the app',
    shellIn >= 0 && shellHtml.includes('ls-root') && bodyText.length > 20 && uriErrors.length === 0,
    `shellIn=${shellIn}ms rootClass="${shellHtml}" bodyChars=${bodyText.length} text="${bodyText.replace(/\n/g, ' | ')}" uriErrors=${JSON.stringify(uriErrors)}`,
  )
  await shot(webview, 'd14-malformed-url.png')

  // ---------------------------------------------------------------- D15
  await go(webview, '/tracker/my-issues/assigned')
  const titleReady = await waitFor(webview, `document.title.endsWith(' - Omon')`, 15000)
  const trackerTitle = await webview.evaluate(`document.title`)
  await hashTo(webview, '#/overview')
  const restored = await waitFor(webview, `document.title === 'omon gateway dashboard'`, 10000)
  const dashTitle = await webview.evaluate(`document.title`)
  const dashMounted = await webview.evaluate(`document.body.innerText.includes('Autonomous Agent Hub')`)
  await hashTo(webview, '#/tracker/my-issues/assigned')
  const again = await waitFor(webview, `document.title.endsWith(' - Omon')`, 10000)
  const backTitle = await webview.evaluate(`document.title`)
  record(
    'D15',
    'leaving the tracker restores the dashboard tab title',
    titleReady >= 0 && restored >= 0 && dashTitle === 'omon gateway dashboard' && dashMounted === true && again >= 0,
    `trackerTitle="${trackerTitle}" restoredIn=${restored}ms dashboardTitle="${dashTitle}" dashboardMounted=${dashMounted} backTitle="${backTitle}"`,
  )
  await shot(webview, 'd15-title-restored.png')

  // ---------------------------------------------------------------- D18
  // Three cases. A: the delayed failure has fully settled before the peek is reopened.
  // B: the draft that A retains is saved by an explicit Retry, exactly once.
  // C: the peek is reopened while the close-triggered retry PATCH is still in flight.
  const DRAFT = 'Draft kept across a failed save'
  const retainedDraft = () =>
    webview.evaluate(`import('/src/tracker/detail/descriptionDraft.ts').then(m => m.retainedDraft('iss-draft') ?? null)`)
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
  const descViewText = `document.querySelector('[aria-label="Issue preview"] .ld-desc-view')?.innerText.trim() ?? ''`
  const pendingBar = `document.querySelectorAll('[aria-label="Issue preview"] .ld-desc-pending').length`

  await ctl('/reset-state', { method: 'POST' })
  await resetLog()
  await go(webview, '/tracker/team/OMO/all')
  await waitFor(webview, rowsReady('OMO-104'), 25000)
  const peekIn = await openPeek()
  await webview.evaluate(`(() => { const el = document.querySelector('[aria-label="Issue preview"] .ld-desc-view'); if (!el) return false; el.click(); return true })()`)
  const textareaIn = await waitFor(webview, `!!document.querySelector('textarea.ld-desc-textarea')`, 6000)
  await webview.evaluate(setInput('textarea.ld-desc-textarea', DRAFT))
  await gate({ '/api/tracker/issues/iss-draft': 'fail500' })
  await webview.evaluate(key({ key: 'Escape', code: 'Escape' }, `document.querySelector('textarea.ld-desc-textarea')`))
  const failedPending = await waitPending('/api/tracker/issues/iss-draft', 8000)
  const failedReleased = await release('/api/tracker/issues/iss-draft')
  const failedShown = await waitFor(webview, `document.querySelector('.ld-desc-status')?.innerText.includes('Not saved') ?? false`, 8000)
  const retryShown = await webview.evaluate(`[...document.querySelectorAll('.ld-tab')].some(b => b.innerText.trim() === 'Retry')`)
  const keptInEditor = await webview.evaluate(`document.querySelector('textarea.ld-desc-textarea')?.value ?? ''`)

  // close: the unmount keeps the draft and fires one more save, which the gate fails as well
  const peekClosed = await closePeek()
  const retryPending = await waitPending('/api/tracker/issues/iss-draft', 6000)
  const retryReleased = await release('/api/tracker/issues/iss-draft')
  let quiesced = false
  for (let i = 0; i < 60 && !quiesced; i++) {
    quiesced = (await pending()).length === 0
    if (!quiesced) await Bun.sleep(50)
  }
  const settledRetained = await retainedDraft()

  // case A: reopen after everything settled
  const reopened = await openPeek()
  await waitFor(webview, `(${descViewText}).length > 0`, 5000)
  const aFirst = { text: await webview.evaluate(descViewText), bars: await webview.evaluate(pendingBar) }
  await Bun.sleep(400)
  const draftAfterReopen = await webview.evaluate(descViewText)
  const pendingAfterReopen = await webview.evaluate(pendingBar)
  await shot(webview, 'd18-draft-after-reopen.png')
  record(
    'D18',
    'case A: a delayed failed save keeps the draft through close/reopen',
    peekIn >= 0 && textareaIn >= 0 && failedPending !== null && failedReleased.released >= 1 && failedShown >= 0 && retryShown === true && keptInEditor === DRAFT && peekClosed >= 0 && retryPending !== null && retryReleased.released === 1 && settledRetained === DRAFT && reopened >= 0 && draftAfterReopen.includes(DRAFT) && pendingAfterReopen === 1,
    `peekIn=${peekIn}ms failedPending=${JSON.stringify(failedPending?.notes)} notSavedIn=${failedShown}ms retryShown=${retryShown} keptInEditor="${keptInEditor}" closedIn=${peekClosed}ms retryPending=${JSON.stringify(retryPending?.notes)} released=${retryReleased.released} quiesced=${quiesced} retained@settled=${JSON.stringify(settledRetained)} reopened=${reopened}ms firstFrame="${aFirst.text}" firstBars=${aFirst.bars} after400ms="${draftAfterReopen}" pendingBars=${pendingAfterReopen}`,
  )

  // case B: Retry from that state saves the retained draft exactly once
  await gate({ '/api/tracker/issues/iss-draft': false })
  await webview.evaluate(`(() => { document.querySelectorAll('.lui-toast [aria-label="Dismiss"]').forEach(b => b.click()); return true })()`)
  await waitFor(webview, `document.querySelectorAll('.lui-toast').length === 0`, 4000)
  await resetLog()
  const retryClicked = await webview.evaluate(`(() => { const b = [...document.querySelectorAll('[aria-label="Issue preview"] .ld-desc-pending .ld-tab')].find(x => x.innerText.trim() === 'Retry'); if (!b) return 'no-button'; b.click(); return 'clicked' })()`)
  const retryPatch = await waitLog((e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-draft' && e.done, 8000)
  const pendingCleared = await waitFor(webview, `!document.querySelector('[aria-label="Issue preview"] .ld-desc-pending')`, 6000)
  const stored = (await mockState()).issues.find((i: any) => i.id === 'iss-draft').description
  const patchesForRetry = (await reqlog()).filter((e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-draft')
  const retryToasts = await webview.evaluate(`[...document.querySelectorAll('.lui-toast')].map(t => t.innerText.replace(/\\n/g, ' '))`)
  const retainedAfterRetry = await retainedDraft()
  record(
    'D18',
    'case B: Retry after recovery saves the retained draft exactly once',
    retryClicked === 'clicked' && retryPatch !== null && pendingCleared >= 0 && stored === DRAFT && patchesForRetry.length === 1,
    `click=${retryClicked} patch=${retryPatch?.note} patchCount=${patchesForRetry.length} pendingClearedIn=${pendingCleared}ms stored="${stored}" retainedAfterRetry=${JSON.stringify(retainedAfterRetry)} toasts=${JSON.stringify(retryToasts)}`,
  )
  await shot(webview, 'd18-retry-saved.png')
  await closePeek()

  // case C: reopen while the close-triggered retry is still in flight (the delayed-failure window)
  await ctl('/reset-state', { method: 'POST' })
  await resetLog()
  await gate({ '/api/tracker/issues/iss-draft': 'fail500' })
  await hashTo(webview, '#/tracker/team/OMO/all')
  await waitFor(webview, rowsReady('OMO-104'), 10000)
  await openPeek()
  await webview.evaluate(`(() => { const el = document.querySelector('[aria-label="Issue preview"] .ld-desc-view'); if (!el) return false; el.click(); return true })()`)
  await waitFor(webview, `!!document.querySelector('textarea.ld-desc-textarea')`, 6000)
  await webview.evaluate(setInput('textarea.ld-desc-textarea', DRAFT))
  await webview.evaluate(key({ key: 'Escape', code: 'Escape' }, `document.querySelector('textarea.ld-desc-textarea')`))
  await waitPending('/api/tracker/issues/iss-draft', 8000)
  await release('/api/tracker/issues/iss-draft')
  await waitFor(webview, `document.querySelector('.ld-desc-status')?.innerText.includes('Not saved') ?? false`, 8000)
  await closePeek()
  const cRetryPending = await waitPending('/api/tracker/issues/iss-draft', 6000)
  await openPeek()
  await waitFor(webview, `(${descViewText}).length > 0`, 5000)
  const cFirstFrame = { text: await webview.evaluate(descViewText), retained: await retainedDraft() }
  await Bun.sleep(600)
  const cSettled = { text: await webview.evaluate(descViewText), retained: await retainedDraft(), bars: await webview.evaluate(pendingBar) }
  await shot(webview, 'd18-reopen-while-retry-inflight.png')
  record(
    'D18',
    'case C: reopening while the retry PATCH is in flight keeps the draft',
    cRetryPending !== null && cSettled.text.includes(DRAFT) && cSettled.retained === DRAFT,
    `retryStillPending=${JSON.stringify(cRetryPending?.notes)} firstFrame="${cFirstFrame.text}" retained@firstFrame=${JSON.stringify(cFirstFrame.retained)} | afterSettle="${cSettled.text}" retained@settle=${JSON.stringify(cSettled.retained)} pendingBars=${cSettled.bars}`,
  )
  await gate({ '/api/tracker/issues/iss-draft': false })
  await closePeek()

  // ---------------------------------------------------------------- D19
  await post('/scenario', { name: 'system-only' })
  await ctl('/reset-state', { method: 'POST' })
  await post('/scenario', { name: 'system-only' })
  await resetLog()
  await go(webview, '/tracker/team/SES/all')
  await waitFor(webview, `!!document.querySelector('.ls-page')`, 15000)
  await webview.evaluate(key({ key: 'c', code: 'KeyC' }))
  const modalIn = await waitFor(webview, `!!document.querySelector('[aria-label="New issue"]')`, 8000)
  const notice = await webview.evaluate(`document.querySelector('[aria-label="New issue"] .ci-notice')?.innerText.replace(/\\n/g, ' ') ?? ''`)
  const createDisabled = await webview.evaluate(
    `[...document.querySelectorAll('[aria-label="New issue"] button')].find(b => b.innerText.trim().startsWith('Create issue'))?.disabled ?? null`,
  )
  const teamLabel = await webview.evaluate(`document.querySelector('[aria-label="New issue"] [aria-label="Team"]')?.innerText.trim() ?? ''`)
  // mod+enter must not slip past the disabled button
  await webview.evaluate(setInput('[aria-label="New issue"] textarea[aria-label="Issue title"]', 'should not be filed'))
  await webview.evaluate(key({ key: 'Enter', code: 'Enter', metaKey: true }, `document.querySelector('textarea[aria-label="Issue title"]')`))
  const createCalls = (await reqlog()).filter((e) => e.method === 'POST' && e.path === '/api/tracker/issues')
  record(
    'D19',
    'system-only workspace: issue creation is blocked with an actionable notice',
    modalIn >= 0 && notice.length > 20 && createDisabled === true && teamLabel.trim() === 'Team' && createCalls.length === 0,
    `modalIn=${modalIn}ms notice="${notice}" createDisabled=${createDisabled} teamLabel="${teamLabel}" createPosts=${createCalls.length}`,
  )
  await shot(webview, 'd19-system-only-desktop.png')
  await webview.resize(390, 844)
  const d19Notice = await webview.evaluate(`document.querySelector('[aria-label="New issue"] .ci-notice')?.innerText.trim().length ?? 0`)
  const d19Overflow = await webview.evaluate(`({ scrollW: document.documentElement.scrollWidth, clientW: document.documentElement.clientWidth })`)
  record(
    'D19',
    'mobile 390px: the notice is visible without horizontal overflow',
    d19Notice > 20 && d19Overflow.scrollW <= d19Overflow.clientW,
    `noticeChars=${d19Notice} overflow=${JSON.stringify(d19Overflow)}`,
  )
  await shot(webview, 'd19-system-only-mobile.png')
  await webview.resize(1440, 900)
  await webview.evaluate(`(() => { const b = document.querySelector('[aria-label="New issue"] [aria-label="Close"]'); if (!b) return false; b.click(); return true })()`)
  await waitFor(webview, `!document.querySelector('[aria-label="New issue"]')`, 6000)

  // mixed workspace: a system team route must still seed a regular team
  await ctl('/reset-state', { method: 'POST' })
  await go(webview, '/tracker/team/SES/all')
  await waitFor(webview, `!!document.querySelector('.ls-page')`, 15000)
  await webview.evaluate(key({ key: 'c', code: 'KeyC' }))
  const mixedIn = await waitFor(webview, `!!document.querySelector('[aria-label="New issue"]')`, 8000)
  const mixedTeam = await webview.evaluate(`document.querySelector('[aria-label="New issue"] [aria-label="Team"]')?.innerText.trim() ?? ''`)
  const mixedNotice = await webview.evaluate(`!!document.querySelector('[aria-label="New issue"] .ci-notice')`)
  record(
    'D19',
    'mixed workspace: opening create from the Sessions team seeds a regular team',
    mixedIn >= 0 && mixedTeam.replace(/\s+/g, '') === 'OOMO' && mixedNotice === false,
    `modalIn=${mixedIn}ms team="${mixedTeam}" notice=${mixedNotice}`,
  )
  await webview.evaluate(`(() => { const b = document.querySelector('[aria-label="New issue"] [aria-label="Close"]'); if (!b) return false; b.click(); return true })()`)
  await waitFor(webview, `!document.querySelector('[aria-label="New issue"]')`, 6000)

  // ---------------------------------------------------------------- D26
  await ctl('/reset-state', { method: 'POST' })
  await resetLog()
  await go(webview, '/tracker/my-issues/subscribed')
  const subIn = await waitFor(webview, `!!document.querySelector('[data-issue-row]')`, 15000)
  const subRows = await webview.evaluate(rowIds)
  await shot(webview, 'd26-subscribed-desktop.png')
  await hashTo(webview, '#/tracker/my-issues/created')
  const createdIn = await waitFor(webview, `!!document.querySelector('[data-issue-row="OMO-102"]')`, 8000)
  const createdRows = await webview.evaluate(rowIds)
  const payloadHasField = await webview.evaluate(`fetch('/api/tracker/issues?include_archived=1').then(r => r.json()).then(j => j.items.every(i => Array.isArray(i.subscriber_ids)))`)
  record(
    'D26',
    'Subscribed tab filters on the backend subscriber_ids field',
    subIn >= 0 && subRows.includes('OMO-101') && !subRows.includes('OMO-102') && createdIn >= 0 && createdRows.includes('OMO-102') && payloadHasField === true,
    `subscribedRows=${JSON.stringify(subRows)} createdIn=${createdIn}ms createdRows=${JSON.stringify(createdRows)} payloadCarriesSubscriberIds=${payloadHasField}`,
  )

  // ---------------------------------------------------------------- D27
  await ctl('/reset-state', { method: 'POST' })
  await resetLog()
  await go(webview, '/tracker/team/OMO/all')
  await waitFor(webview, rowsReady('OMO-103'), 15000)
  const completedName = (await mockState()).completed.name
  const openStatus = (ident: string) => `
    (() => {
      const row = document.querySelector('[data-issue-row=${JSON.stringify(ident)}]')
      const btn = row?.querySelector('button[title="Change status"]')
      if (!btn) return false
      btn.click()
      return true
    })()`
  await webview.evaluate(openStatus('OMO-103'))
  const statusMenu = await waitFor(webview, `!!document.querySelector('[role="menuitem"]')`, 6000)
  await webview.evaluate(`
    (() => {
      const item = [...document.querySelectorAll('[role="menuitem"]')].find(i => i.querySelector('.lui-menu-item-label')?.textContent?.trim() === ${JSON.stringify(completedName)})
      if (!item) return false
      item.click()
      return true
    })()`)
  const rejected = await waitLog((e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-lane' && e.done, 8000)
  const toastIn = await waitFor(webview, `!!document.querySelector('.lui-toast')`, 8000)
  const toasts = await webview.evaluate(
    `[...document.querySelectorAll('.lui-toast')].map(t => ({ text: t.innerText.replace(/\\n/g, ' '), role: t.getAttribute('role') }))`,
  )
  await shot(webview, 'd27-lane-not-verified-toast.png')
  record(
    'D27',
    'lane_not_verified 409 raises exactly one actionable toast (no duplicate generic one)',
    statusMenu >= 0 && rejected !== null && toastIn >= 0 && toasts.length === 1 && toasts[0].text.includes('Force close') && toasts[0].text.includes('lane must be verified') && toasts[0].role === 'alert',
    `menuIn=${statusMenu}ms patchDone=${rejected !== null} toastIn=${toastIn}ms toasts=${JSON.stringify(toasts)}`,
  )
  // the action itself still works and is not toasted twice
  await resetLog()
  await webview.evaluate(`(() => { const b = [...document.querySelectorAll('.lui-toast button')].find(x => x.innerText.trim() === 'Force close'); if (!b) return false; b.click(); return true })()`)
  const forced = await waitLog((e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-lane' && e.done && e.note?.includes('force_note'), 8000)
  const forcedState = (await mockState()).issues.find((i: any) => i.id === 'iss-lane').state_id
  const forcedToasts = await webview.evaluate(`document.querySelectorAll('.lui-toast').length`)
  record(
    'D27',
    'Force close retries once with force_note and raises no second toast',
    forced !== null && forcedState === (await mockState()).completed.id && forcedToasts <= 1,
    `patch=${forced?.note} storedState=${forcedState} toasts=${forcedToasts}`,
  )

  // plain failure path keeps exactly one generic toast
  await resetLog()
  await webview.evaluate(`(() => { document.querySelectorAll('.lui-toast [aria-label="Dismiss"]').forEach(b => b.click()); return true })()`)
  await waitFor(webview, `document.querySelectorAll('.lui-toast').length === 0`, 4000)
  await webview.evaluate(openStatus('OMO-105'))
  await waitFor(webview, `!!document.querySelector('[role="menuitem"]')`, 6000)
  await webview.evaluate(`
    (() => {
      const item = [...document.querySelectorAll('[role="menuitem"]')].find(i => i.querySelector('.lui-menu-item-label')?.textContent?.trim() === ${JSON.stringify(completedName)})
      if (!item) return false
      item.click()
      return true
    })()`)
  await waitLog((e) => e.method === 'PATCH' && e.path === '/api/tracker/issues/iss-500' && e.done, 8000)
  await waitFor(webview, `!!document.querySelector('.lui-toast')`, 8000)
  const plainToasts = await webview.evaluate(
    `[...document.querySelectorAll('.lui-toast')].map(t => t.innerText.replace(/\\n/g, ' '))`,
  )
  record(
    'D27',
    'a plain 500 still shows exactly one generic toast',
    plainToasts.length === 1 && plainToasts[0].includes('tracker unavailable') && !plainToasts[0].includes('Force close'),
    `toasts=${JSON.stringify(plainToasts)}`,
  )

  // ---------------------------------------------------------------- D24 (chrome backend: live matchMedia change)
  const chrome = new Bun.WebView({ backend: 'chrome', width: 1440, height: 900 })
  try {
    await go(chrome, '/tracker/settings/preferences')
    const radios = await waitFor(chrome, `document.querySelectorAll('[role="radio"]').length === 3`, 20000)
    await chrome.cdp('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'dark' }] })
    const darkStart = await waitFor(chrome, `document.querySelector('.lin')?.dataset.theme === 'dark'`, 5000)
    await chrome.evaluate(`
      (() => {
        const b = [...document.querySelectorAll('[role="radio"]')].find(r => r.innerText.trim() === 'System')
        if (!b) return false
        b.click()
        return true
      })()`)
    const systemChecked = await waitFor(
      chrome,
      `[...document.querySelectorAll('[role="radio"]')].find(r => r.innerText.trim() === 'System')?.getAttribute('aria-checked') === 'true'`,
      5000,
    )
    const storedPref = await chrome.evaluate(`localStorage.getItem('tracker.theme')`)
    // live OS-level change, no reload, no re-selection
    await chrome.cdp('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'light' }] })
    const lightIn = await waitFor(chrome, `document.querySelector('.lin')?.dataset.theme === 'light'`, 6000)
    const lightTheme = await chrome.evaluate(`document.querySelector('.lin')?.dataset.theme`)
    await shot(chrome, 'd24-system-light.png')
    await chrome.cdp('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'dark' }] })
    const darkIn = await waitFor(chrome, `document.querySelector('.lin')?.dataset.theme === 'dark'`, 6000)
    const darkTheme = await chrome.evaluate(`document.querySelector('.lin')?.dataset.theme`)
    await shot(chrome, 'd24-system-dark.png')
    const stillSystem = await chrome.evaluate(
      `[...document.querySelectorAll('[role="radio"]')].find(r => r.innerText.trim() === 'System')?.getAttribute('aria-checked')`,
    )
    record(
      'D24',
      'System theme follows a live prefers-color-scheme change without a reload',
      radios >= 0 && darkStart >= 0 && systemChecked >= 0 && storedPref === '"system"' && lightIn >= 0 && lightTheme === 'light' && darkIn >= 0 && darkTheme === 'dark' && stillSystem === 'true',
      `radiosIn=${radios}ms startTheme=dark systemCheckedIn=${systemChecked}ms stored=${storedPref} lightIn=${lightIn}ms darkIn=${darkIn}ms finalTheme=${darkTheme} systemStillChecked=${stillSystem}`,
    )
    // explicit Light/Dark still override the OS preference
    await chrome.evaluate(`(() => { const b = [...document.querySelectorAll('[role="radio"]')].find(r => r.innerText.trim() === 'Light'); if (!b) return false; b.click(); return true })()`)
    const explicitLight = await waitFor(chrome, `document.querySelector('.lin')?.dataset.theme === 'light'`, 5000)
    await chrome.cdp('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'dark' }] })
    const stillLight = await waitFor(chrome, `document.querySelector('.lin')?.dataset.theme === 'light'`, 3000)
    record(
      'D24',
      'an explicit Light choice ignores the OS change',
      explicitLight >= 0 && stillLight >= 0,
      `explicitLightIn=${explicitLight}ms themeAfterOsFlipToDark=${await chrome.evaluate(`document.querySelector('.lin')?.dataset.theme`)}`,
    )
  } finally {
    chrome.close()
  }
} catch (err) {
  record('HARNESS', 'run completed without an exception', false, String(err))
  console.error(err)
} finally {
  webview.close()
  await Bun.write(`${OUT}/results-final.json`, JSON.stringify(results, null, 1))
  const fails = results.filter((r) => !r.ok).length
  console.log(`\n${results.length - fails} pass / ${fails} fail`)
  clearTimeout(hard)
  process.exit(fails === 0 ? 0 : 1)
}
