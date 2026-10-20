// Bounded capture of the D28 reopened dialog (visual evidence alongside the DOM read).
const BASE = 'http://127.0.0.1:5199'
const CTL = 'http://127.0.0.1:9121/__ctl'
const OUT = '/tmp/omoverify'
const hard = setTimeout(() => { console.error('HARD TIMEOUT'); process.exit(3) }, 90_000)
hard.unref?.()

const ctl = async (p: string, i?: RequestInit) => (await fetch(`${CTL}${p}`, i)).json()
async function waitFor(v: any, expr: string, ms: number) {
  const t0 = Date.now()
  while (Date.now() - t0 < ms) { if (await v.evaluate(expr)) return Date.now() - t0; await Bun.sleep(25) }
  return -1
}
const setInput = (sel: string, value: string) => `(() => { const el = document.querySelector(${JSON.stringify(sel)}); if (!el) return false; Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(el, ${JSON.stringify(value)}); el.dispatchEvent(new Event('input',{bubbles:true})); return true })()`
const dates = `JSON.stringify([...document.querySelectorAll('[role="dialog"] input[type="date"]')].map(i => i.value))`

await ctl('/reset-state', { method: 'POST' })
const view = new Bun.WebView({ width: 1440, height: 900 })
try {
  await view.navigate(`${BASE}/?cap=1#/tracker/team/OMO/cycles`)
  await waitFor(view, `!!document.querySelector('button')`, 20000)
  const openNew = `(() => { const b = [...document.querySelectorAll('button')].find(x => x.innerText.trim() === 'New cycle'); if (!b) return false; b.click(); return true })()`
  let ready = -1
  for (let i = 0; i < 4 && ready < 0; i++) {
    await view.evaluate(openNew)
    ready = await waitFor(view, `document.querySelectorAll('[role="dialog"] input[type="date"]').length === 2`, 4000)
  }
  const first = await view.evaluate(dates)
  await view.evaluate(setInput('[role="dialog"] input[placeholder="Cycle name (optional)"]', 'Cycle one'))
  await view.evaluate(`(() => { const d = document.querySelector('[role="dialog"]'); const b = [...d.querySelectorAll('button')].find(x => x.innerText.trim() === 'Create cycle'); if (!b || b.disabled) return false; b.click(); return true })()`)
  let created: any = null
  for (let i = 0; i < 160 && !created; i++) { created = (await ctl('/state')).cycles.at(-1); if (!created) await Bun.sleep(25) }
  await waitFor(view, `document.querySelectorAll('[role="dialog"]').length === 0`, 8000)
  let reopened = -1
  for (let i = 0; i < 4 && reopened < 0; i++) {
    await view.evaluate(openNew)
    reopened = await waitFor(view, `document.querySelectorAll('[role="dialog"] input[type="date"]').length === 2`, 4000)
  }
  const before = await view.evaluate(dates)
  // let the portal paint before the snapshot is taken
  await view.evaluate(`new Promise(r => requestAnimationFrame(() => requestAnimationFrame(() => r(true))))`)
  await Bun.sleep(300)
  const shot = await view.screenshot()
  const after = await view.evaluate(dates)
  await Bun.write(`${OUT}/d28-dialog.png`, shot)
  console.log(JSON.stringify({ firstOpen: first, created: created && [created.starts_at, created.ends_at], reopened: before, afterScreenshot: after, shotBytes: shot.size }))
} finally {
  await view.close()
  process.exit(0)
}
