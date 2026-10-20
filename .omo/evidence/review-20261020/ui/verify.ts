// Bounded UI verification for D23 / D25 / D28 / D29 against the isolated mock API.
// Every wait is deadline-bounded and event-driven: gated mock requests are released by an explicit
// signal, so no assertion depends on a fixed sleep or on timing luck.
const BASE = 'http://127.0.0.1:5199'
const CTL = 'http://127.0.0.1:9121/__ctl'
const OUT = '/tmp/omoverify'

const hard = setTimeout(() => {
  console.error('HARD TIMEOUT 150s')
  process.exit(3)
}, 150_000)
hard.unref?.()

const ctl = async (path: string, init?: RequestInit) => (await fetch(`${CTL}${path}`, init)).json()
const reset = () => ctl('/reset')
const gate = (o: Record<string, boolean>) => ctl('/gate', { method: 'POST', body: JSON.stringify(o) })
const pending = async (): Promise<{ path: string; notes: string[] }[]> => await ctl('/pending')
const release = (path: string) => ctl(`/release?path=${encodeURIComponent(path)}`)
const reqlog = async (): Promise<any[]> => await ctl('/log')
const mockState = async (): Promise<any> => await ctl('/state')

const results: { id: string; check: string; ok: boolean; detail: string }[] = []
function record(id: string, check: string, ok: boolean, detail: string) {
  results.push({ id, check, ok, detail })
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
async function go(v: any, hash: string) {
  navCount += 1
  await bounded(`navigate ${hash}`, 25000, () => v.navigate(`${BASE}/?n=${navCount}#${hash}`))
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
    const p = await pending()
    const hit = p.find((x) => x.path === path)
    if (hit) return hit
    await Bun.sleep(25)
  }
  return null
}

const setInput = (selector: string, value: string) => `
  (() => {
    const el = document.querySelector(${JSON.stringify(selector)})
    if (!el) return false
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set
    setter.call(el, ${JSON.stringify(value)})
    el.dispatchEvent(new Event('input', { bubbles: true }))
    return true
  })()`

const enterOn = (selector: string) => `
  (() => {
    const el = document.querySelector(${JSON.stringify(selector)})
    if (!el) return false
    el.focus()
    el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    return true
  })()`

const text = (selector: string) =>
  `(document.querySelector(${JSON.stringify(selector)})?.innerText ?? '').replace(/\\n/g, ' | ')`

const view = new Bun.WebView({ width: 1440, height: 900 })

try {
  // ------------------------------------------------------------------ D23
  await gate({ '/api/tracker/search': true })
  await ctl('/reset-state', { method: 'POST' })
  await reset()
  await go(view, '/tracker/search')
  await waitFor(view, `!!document.querySelector('input[aria-label="Search"]')`, 20000)
  await view.evaluate(`(() => { const el = document.querySelector('input[aria-label="Search"]'); if (!el) return false; el.focus(); return true })()`)
  await view.evaluate(setInput('input[aria-label="Search"]', 'atlas'))
  const d23Pending = await waitPending('/api/tracker/search', 5000)
  const barInFlight = await view.evaluate(text('.ls-search-bar'))
  record(
    'D23',
    'label shown while search request is in flight',
    d23Pending !== null && barInFlight.includes('Searching'),
    `pending=${JSON.stringify(d23Pending)} bar="${barInFlight}"`,
  )

  const clearAt = Date.now()
  await view.evaluate(setInput('input[aria-label="Search"]', ''))
  const goneMs = await waitFor(view, `!document.querySelector('.ls-search-bar').innerText.includes('Searching')`, 5000)
  const stillPending = (await pending()).some((x) => x.path === '/api/tracker/search')
  const barAfterClear = await view.evaluate(text('.ls-search-bar'))
  record(
    'D23',
    'label cleared while request still unreleased (pre-fix it stayed)',
    goneMs >= 0 && stillPending,
    `cleared ${goneMs}ms after clearing input; request still pending=${stillPending}; bar="${barAfterClear}"`,
  )

  const released = (await release('/api/tracker/search')).released
  const settled = await waitFor(
    view,
    `document.querySelector('.ls-empty')?.innerText.includes('Search your workspace') ?? false`,
    5000,
  )
  const barFinal = await view.evaluate(text('.ls-search-bar'))
  const log23 = (await reqlog()).filter((e) => e.path === '/api/tracker/search')
  record(
    'D23',
    'after release the empty-query state holds with no stale label',
    released === 1 && settled >= 0 && !barFinal.includes('Searching'),
    `released=${released} emptyStateAfter=${settled}ms bar="${barFinal}" requestMs=${log23[0]?.ms}`,
  )
  await Bun.write(`${OUT}/d23-cleared.png`, await view.screenshot())

  // ------------------------------------------------------------------ D25
  await ctl('/reset-state', { method: 'POST' })
  await reset()
  await go(view, '/tracker/project/proj-1/overview')
  const nameReady = await waitFor(view, `!!document.querySelector('input[placeholder="Project name"]')`, 15000)
  const nameSel = 'input[placeholder="Project name"]'
  const before = await view.evaluate(`document.querySelector('${nameSel}').value`)
  await view.evaluate(`(() => { const el = document.querySelector('${nameSel}'); if (!el) return false; el.focus(); return true })()`)
  await view.evaluate(setInput(nameSel, '   '))
  const emptied = await view.evaluate(`document.querySelector('${nameSel}').value`)
  await view.evaluate(enterOn(nameSel))
  const revertedMs = await waitFor(
    view,
    `document.querySelector('${nameSel}').value === ${JSON.stringify(before)}`,
    5000,
  )
  const after = await view.evaluate(`document.querySelector('${nameSel}').value`)
  const patches = (await reqlog()).filter((e) => e.method === 'PATCH')
  record(
    'D25',
    'empty project name reverts to the stored name on Enter commit',
    nameReady >= 0 && revertedMs >= 0 && after === before && patches.length === 0,
    `before="${before}" emptied="${emptied}" after="${after}" revertedIn=${revertedMs}ms PATCHes=${patches.length}`,
  )
  await Bun.write(`${OUT}/d25-reverted.png`, await view.screenshot())

  // also confirm a real rename still saves (the guard must not block valid input)
  await reset()
  await view.evaluate(`(() => { const el = document.querySelector('${nameSel}'); if (!el) return false; el.focus(); return true })()`)
  await view.evaluate(setInput(nameSel, 'Atlas Prime'))
  await view.evaluate(enterOn(nameSel))
  const saved = await waitFor(view, `(${JSON.stringify('x')} && true)`, 1)
  let patchSeen: any = null
  for (let i = 0; i < 100 && !patchSeen; i++) {
    patchSeen = (await reqlog()).find((e) => e.method === 'PATCH' && e.done)
    if (!patchSeen) await Bun.sleep(25)
  }
  const stored = (await mockState()).projects[0].name
  record(
    'D25',
    'a non-empty rename still commits',
    Boolean(patchSeen) && stored === 'Atlas Prime',
    `PATCH=${patchSeen?.note} stored="${stored}" probeMs=${saved}`,
  )

  // ------------------------------------------------------------------ D28
  await ctl('/reset-state', { method: 'POST' })
  await reset()
  await go(view, '/tracker/team/OMO/cycles')
  await waitFor(view, `!!document.querySelector('button')`, 15000)
  const openNew = `(() => { const b = [...document.querySelectorAll('button')].find(x => x.innerText.trim() === 'New cycle'); if (!b) return false; b.click(); return true })()`
  const dialogDatesReady = `document.querySelectorAll('[role="dialog"] input[type="date"]').length === 2 && [...document.querySelectorAll('[role="dialog"] input[type="date"]')].every(i => i.value)`
  // The header button can be clicked before the page finishes its first render; retry the open.
  let dialogReady = -1
  for (let attempt = 0; attempt < 4 && dialogReady < 0; attempt++) {
    await view.evaluate(openNew)
    dialogReady = await waitFor(view, dialogDatesReady, 4000)
  }
  const readDates = `[...document.querySelectorAll('[role="dialog"] input[type="date"]')].map(i => i.value).join('..')`
  const firstDates = await view.evaluate(readDates)
  await view.evaluate(setInput('[role="dialog"] input[placeholder="Cycle name (optional)"]', 'Cycle one'))
  const submitCycle = `(() => {
    const dialog = document.querySelector('[role="dialog"]')
    if (!dialog) return 'no-dialog'
    const b = [...dialog.querySelectorAll('button')].find(x => x.innerText.trim() === 'Create cycle')
    if (!b) return 'no-button:' + [...dialog.querySelectorAll('button')].map(x => x.innerText.trim()).join('/')
    if (b.disabled) return 'disabled'
    b.click()
    return 'clicked'
  })()`
  const submitted = await view.evaluate(submitCycle)
  let created: any = null
  for (let i = 0; i < 160 && !created; i++) {
    created = (await mockState()).cycles.at(-1)
    if (!created) await Bun.sleep(25)
  }
  const dialogClosed = await waitFor(view, `document.querySelectorAll('[role="dialog"]').length === 0`, 8000)
  let reopened = -1
  for (let attempt = 0; attempt < 4 && reopened < 0; attempt++) {
    await view.evaluate(openNew)
    reopened = await waitFor(view, dialogDatesReady, 4000)
  }
  const secondDates = await view.evaluate(readDates)
  // starts_at/ends_at are stored in UTC; the dialog's date inputs are local (UTC+9 here), so the
  // expectation has to be formatted in the same local zone the browser shows.
  const localDay = (value: string) => {
    const d = new Date(value)
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  }
  const createdEndDay = created ? localDay(created.ends_at) : ''
  const secondStart = secondDates.split('..')[0]
  const firstStart = firstDates.split('..')[0]
  record(
    'D28',
    'reopened dialog seeds start from the cycle just created, not the first-open default',
    Boolean(created) && reopened >= 0 && secondStart === createdEndDay && secondDates !== firstDates,
    `submit=${submitted} dialogReady=${dialogReady}ms created=${created ? localDay(created.starts_at) + '->' + localDay(created.ends_at) : 'none'} firstOpen="${firstDates}" reopen="${secondDates}" expectedStart=${createdEndDay} firstStartDiffers=${firstStart !== secondStart} closedIn=${dialogClosed}ms`,
  )
  const dialogText = await view.evaluate(
    `(document.querySelector('[role="dialog"]')?.innerText ?? '').replace(/\\n/g, ' | ')`,
  )
  const dialogInputs = await view.evaluate(
    `JSON.stringify([...document.querySelectorAll('[role="dialog"] input')].map(i => ({ type: i.type, value: i.value })))`,
  )
  record(
    'D28',
    'reopened dialog DOM shows the seeded dates',
    dialogInputs.includes(createdEndDay),
    `dialogInputs=${dialogInputs} dialogText="${dialogText.slice(0, 120)}"`,
  )
  await Bun.write(`${OUT}/d28-reopened.png`, await view.screenshot())

  // ------------------------------------------------------------------ D29
  await gate({ '/api/tracker/milestone': true })
  await ctl('/reset-state', { method: 'POST' })
  await reset()
  await go(view, '/tracker/project/proj-1/overview')
  await waitFor(view, `!!document.querySelector('input[placeholder="Project name"]')`, 15000)
  const openAdd = `(() => { const b = [...document.querySelectorAll('button')].find(x => x.innerText.trim() === 'Add milestone'); if (!b) return false; b.click(); return true })()`
  await view.evaluate(openAdd)
  const addReady = await waitFor(view, `!!document.querySelector('input[placeholder="Milestone name"]')`, 8000)
  await view.evaluate(setInput('input[placeholder="Milestone name"]', 'M1'))
  const mSel = 'input[placeholder="Milestone name"]'
  const addButtonState = `(() => {
    const input = document.querySelector('input[placeholder="Milestone name"]')
    const row = input ? input.closest('div') : null
    const b = row ? [...row.querySelectorAll('button')].find(x => x.innerText.trim() === 'Add') : null
    if (!b) return 'missing:' + [...document.querySelectorAll('button')].map(x => x.innerText.trim()).filter(Boolean).join('/')
    return b.disabled ? 'disabled' : 'enabled'
  })()`
  await view.evaluate(enterOn(mSel))
  const firstPending = await waitPending('/api/tracker/milestone', 5000)
  const btnState = await view.evaluate(addButtonState)
  await view.evaluate(enterOn(mSel))
  // Prove the absence of a second request: watch the mock's pending queue for a bounded window.
  let pendingCount = 0
  const settleUntil = Date.now() + 1500
  while (Date.now() < settleUntil) {
    const p = (await pending()).find((x) => x.path === '/api/tracker/milestone')
    pendingCount = Math.max(pendingCount, p ? p.notes.length : 0)
    await Bun.sleep(50)
  }
  await Bun.write(`${OUT}/d29-inflight.png`, await view.screenshot())
  record(
    'D29',
    'a second Enter while the first add is in flight does not queue a second request',
    firstPending !== null && addReady >= 0 && pendingCount === 1 && btnState === 'disabled',
    `firstPending=${JSON.stringify(firstPending)} pendingAfterSecondEnter=${pendingCount} addButton=${btnState}`,
  )
  const relMs = (await release('/api/tracker/milestone')).released
  const rows = await waitFor(view, `[...document.querySelectorAll('li')].some(li => li.innerText.includes('M1'))`, 8000)
  const createdMs = (await mockState()).milestones
  record(
    'D29',
    'the single released request creates exactly one milestone',
    relMs === 1 && rows >= 0 && createdMs.length === 1,
    `released=${relMs} rowShownIn=${rows}ms milestones=${createdMs.length} names=${createdMs.map((m: any) => m.name).join(',')}`,
  )

  // ------------------------------------------------------------------ mobile
  const mobile = new Bun.WebView({ width: 390, height: 844 })
  try {
    await go(mobile, '/tracker/search')
    await waitFor(mobile, `!!document.querySelector('input[aria-label="Search"]')`, 20000)
    await gate({ '/api/tracker/search': false })
    await mobile.evaluate(`(() => { const el = document.querySelector('input[aria-label="Search"]'); if (!el) return false; el.focus(); return true })()`)
    await mobile.evaluate(setInput('input[aria-label="Search"]', 'atlas'))
    const mobileResults = await waitFor(mobile, `!!document.querySelector('.ls-result')`, 8000)
    const overflow = await mobile.evaluate(
      `(() => { const d = document.documentElement; return { scrollW: d.scrollWidth, clientW: d.clientWidth } })()`,
    )
    record(
      'D23',
      'mobile 390px: search renders results without horizontal overflow',
      mobileResults >= 0 && overflow.scrollW <= overflow.clientW + 1,
      `resultsIn=${mobileResults}ms ${JSON.stringify(overflow)}`,
    )
    await Bun.write(`${OUT}/mobile-search.png`, await mobile.screenshot())

    await go(mobile, '/tracker/project/proj-1/overview')
    await waitFor(mobile, `!!document.querySelector('input[placeholder="Project name"]')`, 15000)
    await mobile.evaluate(`(() => { const b = [...document.querySelectorAll('button')].find(x => x.innerText.trim() === 'Add milestone'); if (b) b.click(); return !!b })()`)
    const mobileAdd = await waitFor(mobile, `!!document.querySelector('input[placeholder="Milestone name"]')`, 8000)
    const mobileOverflow = await mobile.evaluate(
      `(() => { const d = document.documentElement; return { scrollW: d.scrollWidth, clientW: d.clientWidth } })()`,
    )
    record(
      'D29',
      'mobile 390px: milestone add form is reachable without horizontal overflow',
      mobileAdd >= 0 && mobileOverflow.scrollW <= mobileOverflow.clientW + 1,
      `addFormIn=${mobileAdd}ms ${JSON.stringify(mobileOverflow)}`,
    )
    await Bun.write(`${OUT}/mobile-milestone.png`, await mobile.screenshot())
  } finally {
    await mobile.close()
  }
} catch (error) {
  console.error('ERROR', error instanceof Error ? `${error.message}\n${error.stack}` : String(error))
  results.push({ id: 'harness', check: 'script completed', ok: false, detail: String(error) })
} finally {
  await view.close()
  const failed = results.filter((r) => !r.ok)
  console.log(`\nSUMMARY ${results.length - failed.length}/${results.length} checks passed`)
  await Bun.write(`${OUT}/results.json`, JSON.stringify(results, null, 1))
  process.exit(failed.length === 0 ? 0 : 1)
}
