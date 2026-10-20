// D18 case B with a TRUSTED click (Bun.WebView.click(selector) dispatches native input, so the page
// sees event.isTrusted === true). The earlier B evidence used element.click(), which is synthetic.
// Same harness, same guards: claim the mock, assert setup, fingerprint the tree, refuse on collision.
const BASE = 'http://127.0.0.1:5299'
const CTL = 'http://127.0.0.1:9131/__ctl'
const OUT = '/Users/indo/code/project/omon-gateway/.omo/evidence/review-20261020/ui-final'
const STAMP = new Date().toISOString().replace(/[:.]/g, '-')
const TOKEN = `st_01a1582e-${process.pid}-${STAMP}`
const TASK_PREFIX = 'st_01a1582e-'
const PATCH = '/api/tracker/issues/iss-draft'
const DRAFT = 'Draft kept across a failed save'

const hard = setTimeout(() => {
  console.error('HARD TIMEOUT 150s')
  process.exit(3)
}, 150_000)
hard.unref?.()

const ctl = async (path: string, init?: RequestInit) => (await fetch(`${CTL}${path}`, init)).json()
const post = (path: string, body: unknown) => ctl(path, { method: 'POST', body: JSON.stringify(body) })
const pending = async (): Promise<{ path: string; notes: string[] }[]> => await ctl('/pending')
const reqlog = async (): Promise<any[]> => await ctl('/log')
const mockState = async (): Promise<any> => await ctl('/state')

const results: any[] = []
const record = (id: string, check: string, ok: boolean, detail: string, phase = 'attempt') => {
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
async function waitFor(view: any, expr: string, ms: number) {
  const t0 = Date.now()
  while (Date.now() - t0 < ms) {
    if (await bounded('eval', 8000, () => view.evaluate(expr))) return Date.now() - t0
    await Bun.sleep(25)
  }
  return -1
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

const webview = new Bun.WebView({ width: 1440, height: 900 })
let treeStart: Record<string, string> = {}

try {
  // ---- prerequisites -------------------------------------------------------
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
  const st = await mockState()
  const ids = (st.issues ?? []).map((i: any) => i.id)
  if (!['iss-sub', 'iss-draft', 'iss-lane', 'iss-500', 'iss-created'].every((n) => ids.includes(n)) || st.scenario !== 'mixed')
    setupFail('mock holds this harness seed', `scenario=${st.scenario}`)
  const proxied = await bounded('proxy', 10000, () => fetch(`${BASE}/api/tracker/issues?include_archived=1`).then((r) => r.json()))
  if (!(proxied.items ?? []).some((i: any) => i.id === 'iss-draft')) setupFail('vite 5299 proxies to this mock', 'iss-draft missing via proxy')
  if ((await ctl('/owner')).owner.token !== TOKEN) setupFail('ownership retained', 'owner mismatch')
  treeStart = await treeFingerprint()

  // ---- set up: fail one save, close, reopen (the state where Retry was dead) ----
  await ctl('/reset-state', { method: 'POST' })
  await ctl('/reset', { method: 'POST' })
  await bounded('nav', 25000, () => webview.navigate(`${BASE}/?n=1#/tracker/team/OMO/all`))
  if ((await waitFor(webview, `!!document.querySelector('[data-issue-row="OMO-104"]')`, 25000)) < 0) setupFail('rows render', 'no OMO-104 row')
  await webview.evaluate(hover('OMO-104'))
  await waitFor(webview, `!!document.querySelector('.lic-row--focused')`, 5000)
  await webview.evaluate(key({ key: ' ', code: 'Space' }))
  if ((await waitFor(webview, `!!document.querySelector('[aria-label="Issue preview"] .ld-desc-view')`, 8000)) < 0) setupFail('peek opens', 'no desc view')
  await webview.evaluate(`(() => { document.querySelector('[aria-label="Issue preview"] .ld-desc-view').click(); return true })()`)
  if ((await waitFor(webview, `!!document.querySelector('textarea.ld-desc-textarea')`, 6000)) < 0) setupFail('editor opens', 'no textarea')
  await webview.evaluate(setInput('textarea.ld-desc-textarea', DRAFT))
  await post('/gate', { [PATCH]: 'fail500' })
  await webview.evaluate(key({ key: 'Escape', code: 'Escape' }, `document.querySelector('textarea.ld-desc-textarea')`))
  const held = await waitPending(PATCH, 8000)
  await ctl(`/release?path=${encodeURIComponent(PATCH)}`)
  await waitFor(webview, `document.querySelector('.ld-desc-status')?.innerText.includes('Not saved') ?? false`, 8000)
  await webview.evaluate(`(() => { document.querySelector('[aria-label="Issue preview"] [aria-label="Close"]').click(); return true })()`)
  await waitFor(webview, `!document.querySelector('[aria-label="Issue preview"]')`, 6000)
  await waitPending(PATCH, 6000)
  await ctl(`/release?path=${encodeURIComponent(PATCH)}`)
  for (let i = 0; i < 60; i++) {
    if ((await pending()).length === 0) break
    await Bun.sleep(50)
  }
  await webview.evaluate(hover('OMO-104'))
  await waitFor(webview, `!!document.querySelector('.lic-row--focused')`, 5000)
  await webview.evaluate(key({ key: ' ', code: 'Space' }))
  if ((await waitFor(webview, `!!document.querySelector('[aria-label="Issue preview"] .ld-desc-view')`, 8000)) < 0) setupFail('reopen shows peek', 'no desc view after reopen')
  const barsBefore = await webview.evaluate(`document.querySelectorAll('.ld-desc-pending').length`)
  const viewBefore = await webview.evaluate(`document.querySelector('[aria-label="Issue preview"] .ld-desc-view')?.innerText.trim() ?? ''`)
  if (barsBefore !== 1) setupFail('reopened peek shows the Unsaved changes bar', `bars=${barsBefore}`)
  if (!viewBefore.includes(DRAFT)) setupFail('reopened peek shows the draft', `view="${viewBefore}"`)

  // ---- the measurement: a TRUSTED click on Retry -------------------------------
  await post('/gate', { [PATCH]: false })
  await webview.evaluate(`(() => { document.querySelectorAll('.lui-toast [aria-label="Dismiss"]').forEach(b => b.click()); return true })()`)
  await waitFor(webview, `document.querySelectorAll('.lui-toast').length === 0`, 4000)
  await ctl('/reset')
  // capture isTrusted on the button's own click listener
  await webview.evaluate(`
    (() => {
      window.__t = []
      const btn = [...document.querySelectorAll('.ld-desc-pending .ld-tab')].find(b => b.innerText.trim() === 'Retry')
      btn.addEventListener('click', (e) => window.__t.push({ k: 'click', trusted: e.isTrusted }), true)
      const of = window.fetch
      window.fetch = function (...a) { const u = String(a[0]); window.__t.push({ k: 'fetch', m: (a[1] && a[1].method) || 'GET', u }); return of.apply(this, a) }
      return true
    })()`)
  // Bun.WebView.click(selector) waits for actionability and dispatches native input.
  await bounded('trusted click', 15000, () => webview.click('[aria-label="Issue preview"] .ld-desc-pending .ld-tab'))
  const patch = await waitLog((e) => e.method === 'PATCH' && e.path === PATCH && e.done, 8000)
  const cleared = await waitFor(webview, `!document.querySelector('[aria-label="Issue preview"] .ld-desc-pending')`, 6000)
  const stored = (await mockState()).issues.find((i: any) => i.id === 'iss-draft').description
  const patches = (await reqlog()).filter((e) => e.method === 'PATCH' && e.path === PATCH)
  const trace = await webview.evaluate(`window.__t ?? []`)
  const trustedSeen = trace.some((t: any) => t.k === 'click' && t.trusted === true)
  await Bun.write(`${OUT}/d18-attempt-${STAMP}-b-trusted-click.png`, await webview.screenshot())

  const treeEnd = await treeFingerprint()
  const moved = Object.keys(treeStart).filter((k) => treeStart[k] !== treeEnd[k])
  if (moved.length > 0) setupFail('tree unchanged during the run', `changed=${JSON.stringify(moved)}`)

  record(
    'D18',
    'case B with a trusted click: Retry saves the retained draft exactly once',
    held !== null && trustedSeen && patch !== null && cleared >= 0 && stored === DRAFT && patches.length === 1,
    `trustedClick=${trustedSeen} patchCount=${patches.length} patch=${patch?.note} barClearedIn=${cleared}ms stored="${stored}" trace=${JSON.stringify(trace)}`,
  )
  record('SETUP', 'tree unchanged during the run', true, JSON.stringify(treeEnd), 'setup-ok')
} catch (err) {
  if (!results.some((r) => r.phase === 'setup')) record('HARNESS', 'run completed without an exception', false, String(err), 'harness')
  console.error(err)
} finally {
  webview.close()
  const file = `${OUT}/results-d18-b-trusted-${STAMP}.json`
  await Bun.write(file, JSON.stringify({ token: TOKEN, stamp: STAMP, tree: treeStart, results }, null, 1))
  const setup = results.filter((r) => r.phase === 'setup')
  const fails = results.filter((r) => r.phase !== 'setup' && r.phase !== 'setup-ok' && !r.ok).length
  console.log(`\nwrote ${file}`)
  console.log(setup.length ? `SETUP FAILED - no verdict: ${setup.map((s) => s.check).join('; ')}` : `D18-B trusted: ${fails} fail`)
  clearTimeout(hard)
  process.exit(setup.length ? 4 : fails === 0 ? 0 : 1)
}
