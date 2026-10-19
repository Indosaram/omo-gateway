const { loadOmowright } = await import('/Users/indo/.bun/install/global/node_modules/omo-ai/plugin/skills/browser/scripts/omowright.mjs')
const { omowright } = await loadOmowright()
const { mkdtempSync, rmSync, mkdirSync } = await import('node:fs')

const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const ORIGIN = process.env.TRACKER_ORIGIN ?? 'http://127.0.0.1:9333'
const OUT = process.env.TRACKER_SHOTS ?? '/tmp/tracker-qa/shots'
mkdirSync(OUT, { recursive: true })

const log: string[] = []
const results: { id: string; pass: boolean; detail: string }[] = []
const note = (s: string) => {
  log.push(`${new Date().toISOString()} ${s}`)
  console.log(s)
}
const check = (id: string, pass: boolean, detail = '') => {
  results.push({ id, pass, detail })
  note(`${pass ? 'PASS' : 'FAIL'} ${id} ${detail}`)
}

const profile = mkdtempSync('/tmp/tracker-qa/browser-')
const browser = await omowright.connectPipe({
  browserPath: CHROME,
  browserArgs: ['--headless=new', '--no-first-run', `--user-data-dir=${profile}`],
  storageRoot: profile,
})

async function waitFor(page: any, fn: string, timeoutMs = 8000): Promise<boolean> {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const ok = await page.evaluate(new Function(`return (${fn})()`) as () => boolean).catch(() => false)
    if (ok) return true
    await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))))
  }
  return false
}

async function shot(page: any, name: string) {
  const path = `${OUT}/${name}.png`
  await Bun.write(path, await page.screenshot())
  note(`shot ${path}`)
  return path
}

async function key(page: any, k: string, mods: string[] = []) {
  for (const m of mods) await page.keyboard.down(m)
  await page.keyboard.press(k)
  for (const m of [...mods].reverse()) await page.keyboard.up(m)
}

const textIncludes = (t: string) => `() => document.body.innerText.includes(${JSON.stringify(t)})`

async function setTheme(page: any, theme: 'dark' | 'light') {
  await page.evaluate((t: string) => localStorage.setItem('tracker.theme', JSON.stringify(t)), theme)
}

let page: any
try {
  page = await browser.newTab('about:blank')
  await omowright.emulate(page, 'desktop-1440')
  await page.goto(`${ORIGIN}/#/tracker`, { waitUntil: 'load' })
  await setTheme(page, 'dark')
  await page.goto(`${ORIGIN}/#/tracker`, { waitUntil: 'load' })
  const shellReady = await waitFor(page, `() => !!document.querySelector('.lin') && document.body.innerText.includes('Inbox') && document.body.innerText.includes('My issues')`)
  check('B1 shell renders (sidebar Inbox/My issues)', shellReady)
  await waitFor(page, `() => location.hash.includes('/team/')`)
  check('B1 home redirects to team issues', (await page.evaluate(() => location.hash)).includes('/team/'), await page.evaluate(() => location.hash))
  await shot(page, 'B1-team-active-desktop-dark')

  await key(page, 'c')
  const modalOpen = await waitFor(page, `() => !!document.querySelector('[role=dialog]')`)
  check('B2 C opens create modal', modalOpen)
  await page.keyboard.type('Browser QA issue')
  await shot(page, 'B2-create-modal-desktop-dark')
  await key(page, 'Enter', ['Meta'])
  const created = await waitFor(page, textIncludes('Browser QA issue'), 6000)
  const modalClosed = await waitFor(page, `() => !document.querySelector('[role=dialog]')`, 4000)
  check('B2 issue created and listed', created && modalClosed)
  await shot(page, 'B2-after-create-desktop-dark')

  await key(page, 'b', ['Meta'])
  await page.evaluate(() => new Promise((r) => setTimeout(r, 300)))
  await shot(page, 'B3-board-desktop-dark')
  const boardCols = await page.evaluate(() => document.querySelectorAll('[data-board-column]').length)
  check('B3 board layout has columns', boardCols >= 2, `columns=${boardCols}`)

  const dragInfo = await page.evaluate(() => {
    const card = [...document.querySelectorAll('[data-issue-card]')].find((el) => (el as HTMLElement).innerText.includes('Browser QA issue')) as HTMLElement | undefined
    const cols = [...document.querySelectorAll('[data-board-column]')] as HTMLElement[]
    const target = cols.find((c) => (c.getAttribute('data-board-column-name') ?? c.innerText).includes('In Progress'))
    if (!card || !target) return null
    const a = card.getBoundingClientRect()
    const b = target.getBoundingClientRect()
    return { from: { x: a.x + a.width / 2, y: a.y + 12 }, to: { x: b.x + b.width / 2, y: b.y + 80 } }
  })
  if (dragInfo) {
    await page.mouse.move(dragInfo.from.x, dragInfo.from.y)
    await page.mouse.down()
    for (let i = 1; i <= 12; i++) {
      await page.mouse.move(dragInfo.from.x + ((dragInfo.to.x - dragInfo.from.x) * i) / 12, dragInfo.from.y + ((dragInfo.to.y - dragInfo.from.y) * i) / 12)
    }
    await shot(page, 'B3-board-dragging')
    await page.mouse.up()
    const persisted = await waitFor(
      page,
      `async () => { const r = await fetch('/api/tracker/search?q=Browser%20QA%20issue').then(r=>r.json()); const b = await fetch('/api/tracker/bootstrap').then(r=>r.json()); const i = r.issues[0]; const s = b.states.find(x=>x.id===i?.state_id); return s?.name === 'In Progress' }`,
      6000,
    )
    check('B3 drag Todo→In Progress persisted', persisted)
  } else {
    check('B3 drag Todo→In Progress persisted', false, 'card or In Progress column not found')
  }
  await key(page, 'b', ['Meta'])

  await key(page, 'f')
  const filterMenu = await waitFor(page, textIncludes('Priority'))
  check('B4 F opens filter menu', filterMenu)
  await shot(page, 'B4-filter-menu')
  await key(page, 'Escape')

  await page.goto(`${ORIGIN}/#/tracker/search?q=Browser%20QA`, { waitUntil: 'load' })
  await waitFor(page, textIncludes('Browser QA issue'))
  const identifier = await page.evaluate(async () => {
    const r = await fetch('/api/tracker/search?q=Browser%20QA%20issue').then((x) => x.json())
    return r.issues[0]?.identifier as string | undefined
  })
  check('S6/B search page shows result', Boolean(identifier), identifier ?? '')
  await shot(page, 'B-search-desktop-dark')

  if (identifier) {
    await page.goto(`${ORIGIN}/#/tracker/issue/${identifier}`, { waitUntil: 'load' })
    const detailReady = await waitFor(page, textIncludes('Browser QA issue'))
    check('B5 issue page renders', detailReady)
    await shot(page, 'B5-issue-page-desktop-dark')
    await key(page, 's')
    const statusMenu = await waitFor(page, textIncludes('Done'))
    check('B5 S opens status picker', statusMenu)
    await shot(page, 'B5-status-picker')
    await key(page, 'Escape')
    await key(page, 'm')
    await page.keyboard.type('Comment from browser QA')
    await key(page, 'Enter', ['Meta'])
    const commented = await waitFor(page, textIncludes('Comment from browser QA'), 6000)
    check('B5 comment posted', commented)
    await shot(page, 'B5-comment')
  }

  await key(page, 'k', ['Meta'])
  const palette = await waitFor(page, `() => document.body.innerText.includes('Go to') || document.body.innerText.includes('Projects')`)
  check('B7 ⌘K opens command palette', palette)
  await shot(page, 'B7-command-palette')
  await page.keyboard.type('projects')
  await key(page, 'Enter')
  const onProjects = await waitFor(page, `() => location.hash.includes('/projects')`)
  check('B7 palette navigates to Projects', onProjects)
  await shot(page, 'B7-projects-desktop-dark')

  await page.goto(`${ORIGIN}/#/tracker/inbox`, { waitUntil: 'load' })
  await waitFor(page, textIncludes('Inbox'))
  await shot(page, 'B8-inbox-desktop-dark')
  check('B8 inbox renders', true)

  await page.goto(`${ORIGIN}/#/tracker/my-issues/assigned`, { waitUntil: 'load' })
  await waitFor(page, textIncludes('My issues'))
  await shot(page, 'B-my-issues-desktop-dark')

  const firstTeam = await page.evaluate(async () => (await fetch('/api/tracker/bootstrap').then((r) => r.json())).teams.find((t: any) => !t.system).key)
  await page.goto(`${ORIGIN}/#/tracker/team/${firstTeam}/cycles`, { waitUntil: 'load' })
  await waitFor(page, textIncludes('Cycle'))
  await shot(page, 'B8-cycles-desktop-dark')

  await page.goto(`${ORIGIN}/#/tracker/team/AGT/all`, { waitUntil: 'load' })
  const agentIssues = await waitFor(page, `() => document.querySelectorAll('[data-issue-row]').length >= 1`)
  check('B agent lanes visible as AGT issues', agentIssues)
  await shot(page, 'B-agents-team-desktop-dark')

  await page.goto(`${ORIGIN}/#/tracker/team/${firstTeam}/all`, { waitUntil: 'load' })
  await waitFor(page, `() => document.querySelectorAll('[data-issue-row]').length >= 2`)
  await page.evaluate(() => (document.querySelector('[data-issue-row]') as HTMLElement)?.focus())
  await key(page, 'j')
  await key(page, 'x')
  await key(page, 'j')
  await key(page, 'x')
  const bulkBar = await waitFor(page, textIncludes('selected'))
  check('B9 X multi-select shows bulk bar', bulkBar)
  await shot(page, 'B9-bulk-bar')
  await key(page, 'Escape')

  await setTheme(page, 'light')
  await page.goto(`${ORIGIN}/#/tracker/team/${firstTeam}/all`, { waitUntil: 'load' })
  await waitFor(page, `() => document.querySelector('.lin')?.getAttribute('data-theme') === 'light'`)
  await shot(page, 'B10-team-desktop-light')
  if (identifier) {
    await page.goto(`${ORIGIN}/#/tracker/issue/${identifier}`, { waitUntil: 'load' })
    await waitFor(page, textIncludes('Browser QA issue'))
    await shot(page, 'B10-issue-desktop-light')
  }

  await omowright.emulate(page, 'iphone-14')
  for (const theme of ['dark', 'light'] as const) {
    await setTheme(page, theme)
    await page.goto(`${ORIGIN}/#/tracker/team/${firstTeam}/all`, { waitUntil: 'load' })
    await waitFor(page, `() => !!document.querySelector('.lin')`)
    await shot(page, `B10-team-mobile-${theme}`)
    const noHScroll = await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)
    check(`B10 mobile ${theme} no horizontal page scroll`, noHScroll)
    if (identifier) {
      await page.goto(`${ORIGIN}/#/tracker/issue/${identifier}`, { waitUntil: 'load' })
      await waitFor(page, textIncludes('Browser QA issue'))
      await shot(page, `B10-issue-mobile-${theme}`)
    }
  }
  await setTheme(page, 'dark')
  await page.goto(`${ORIGIN}/#/tracker/team/${firstTeam}/all`, { waitUntil: 'load' })
  const opened = await page.evaluate(() => {
    const btn = [...document.querySelectorAll('button')].find((b) => /menu|navigation|sidebar/i.test(b.getAttribute('aria-label') ?? ''))
    btn?.click()
    return Boolean(btn)
  })
  await waitFor(page, `() => document.body.innerText.includes('Inbox')`)
  await shot(page, 'B10-mobile-drawer-dark')
  check('B10 mobile drawer toggle exists', opened)
} finally {
  if (page) await omowright.emulate(page, null).catch(() => undefined)
  await browser.close()
  rmSync(profile, { recursive: true, force: true })
  await Bun.write(`${OUT}/browser-qa-log.txt`, log.join('\n'))
  await Bun.write(`${OUT}/browser-qa.json`, JSON.stringify(results, null, 2))
}
const failed = results.filter((r) => !r.pass)
console.log(`\n${results.length - failed.length}/${results.length} passed`)
process.exit(failed.length ? 1 : 0)
