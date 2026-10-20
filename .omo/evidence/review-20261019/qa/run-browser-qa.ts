import { mkdtempSync, rmSync, mkdirSync } from 'node:fs';

const { loadOmowright } = await import('/Users/indo/.bun/install/global/node_modules/omo-ai/plugin/skills/browser/scripts/omowright.mjs');
const { omowright } = await loadOmowright();

const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const PORT = Number(process.env.TRACKER_QA_PORT ?? 9680);
const ORIGIN = `http://127.0.0.1:${PORT}`;
const SHOTS_DIR = `${process.cwd()}/.omo/evidence/review-20261019/qa/shots`;
const QA_DIR = `${process.cwd()}/.omo/evidence/review-20261019/qa`;
mkdirSync(SHOTS_DIR, { recursive: true });

const tmpDir = mkdtempSync('/tmp/tracker-browser-qa-');
const dbPath = `${tmpDir}/qa.db`;

console.log(`[QA] Starting isolated QA server on port ${PORT} with DB ${dbPath}`);
const serverEnv = Object.assign({}, process.env, {
  DATABASE_URL: `sqlite://${dbPath}`,
  DASHBOARD_WEB_ROOT: `${process.cwd()}/web/dist`,
  RUST_LOG: 'info',
});

const serverProc = Bun.spawn([
  `${process.cwd()}/target/debug/omo-gateway`,
  'dashboard',
  '--port', `${PORT}`
], {
  env: serverEnv,
  stdout: 'pipe',
  stderr: 'pipe'
});

const serverReady = await new Promise<boolean>((resolve) => {
  const timeout = setTimeout(() => resolve(false), 30000);
  const consume = async (stream: ReadableStream<Uint8Array>) => {
    let text = '';
    for await (const chunk of stream) {
      text += new TextDecoder().decode(chunk);
      if (text.includes('omon dashboard listening')) {
        clearTimeout(timeout);
        resolve(true);
      }
      text = text.slice(-4096);
    }
  };
  void consume(serverProc.stdout);
  void consume(serverProc.stderr);
  void serverProc.exited.then(() => { clearTimeout(timeout); resolve(false); });
});

if (!serverReady) {
  console.error('[QA] Server failed to start');
  serverProc.kill();
  process.exit(1);
}
console.log('[QA] Server ready. Seeding fixture data...');

const boot = await fetch(`${ORIGIN}/api/tracker/bootstrap`).then(r => r.json());
const me = boot.me;
const omo = boot.teams.find((t: any) => t.key === 'OMO');
const createLabel = async (name: string, color: string) => {
  const response = await fetch(`${ORIGIN}/api/tracker/labels`, {
    method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ name, color, team_id: omo.id })
  });
  if (!response.ok) throw new Error(`Cannot create fixture label: ${response.status}`);
  return response.json();
};
const labelA = await createLabel('UI', '#6366f1');
const labelB = await createLabel('API', '#22c55e');

const projRes = await fetch(`${ORIGIN}/api/tracker/projects`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ name: 'Alpha Initiative', team_ids: [omo.id], color: '#6366f1' })
});
const project = await projRes.json();

const m1Res = await fetch(`${ORIGIN}/api/tracker/projects/${project.id}/milestones`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ name: 'Milestone 1.0', target_date: '2026-11-01' })
});
const m1 = await m1Res.json();

const m2Res = await fetch(`${ORIGIN}/api/tracker/projects/${project.id}/milestones`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ name: 'Milestone 2.0', target_date: '2026-12-01' })
});
const m2 = await m2Res.json();

const viewRes = await fetch(`${ORIGIN}/api/tracker/views`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({
    name: 'High Priority View',
    team_id: omo.id,
    filters: { priorities: [1, 2] },
    display: { layout: 'list', ordering: 'manual' }
  })
});
const savedView = await viewRes.json();

const issue1Res = await fetch(`${ORIGIN}/api/tracker/issues`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({
    team_id: omo.id,
    title: 'QA Issue for Autosave and Milestone',
    description: 'Initial description content',
    priority: 1,
    project_id: project.id,
    milestone_id: m1.id,
    label_ids: [labelA.id, labelB.id]
  })
});
const issue1 = await issue1Res.json();

const issue2Res = await fetch(`${ORIGIN}/api/tracker/issues`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({
    team_id: omo.id,
    title: 'Board Drag Target Issue',
    description: 'Target issue description',
    priority: 2
  })
});
const issue2 = await issue2Res.json();

const now = new Date().toISOString();
await Bun.$`sqlite3 ${dbPath} "INSERT INTO tracker_notifications (id, user_id, issue_id, kind, actor_id, body, created_at) VALUES ('n-1', '${me.id}', '${issue1.id}', 'mentioned', '${me.id}', 'Mentioned you in QA Issue', '${now}')"`;
await Bun.$`sqlite3 ${dbPath} "INSERT INTO tracker_notifications (id, user_id, issue_id, kind, actor_id, body, created_at) VALUES ('n-2', '${me.id}', '${issue2.id}', 'assigned', '${me.id}', 'Assigned you to Target Issue', '${now}')"`;
await Bun.$`sqlite3 ${dbPath} "INSERT INTO tracker_notifications (id, user_id, issue_id, kind, actor_id, body, created_at) VALUES ('n-3', '${me.id}', '${issue1.id}', 'comment', '${me.id}', 'Commented on QA Issue', '${now}')"`;

console.log('[QA] Fixture data seeded successfully.');

const results: { id: string; status: 'PASS' | 'FAIL' | 'UNTESTED'; detail: string; shot?: string }[] = [];
function record(id: string, pass: boolean, detail: string, shot?: string) {
  results.push({ id, status: pass ? 'PASS' : 'FAIL', detail, shot });
  console.log(`[${pass ? 'PASS' : 'FAIL'}] ${id}: ${detail}`);
}

const profile = mkdtempSync('/tmp/tracker-chrome-');
const browser = await omowright.connectPipe({
  browserPath: CHROME,
  browserArgs: ['--headless=new', '--no-first-run', `--user-data-dir=${profile}`],
  storageRoot: profile,
});

async function waitFor(page: any, fn: string, timeoutMs = 8000): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const ok = await page.evaluate(new Function(`return (${fn})()`) as () => boolean).catch(() => false);
    if (ok) return true;
    await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
  }
  return false;
}

async function takeShot(page: any, name: string) {
  const path = `${SHOTS_DIR}/${name}.png`;
  await Bun.write(path, await page.screenshot());
  return `${name}.png`;
}

async function key(page: any, k: string, mods: string[] = []) {
  for (const m of mods) await page.keyboard.down(m);
  await page.keyboard.press(k);
  for (const m of [...mods].reverse()) await page.keyboard.up(m);
}

try {
  const page = await browser.newTab('about:blank');
  await omowright.emulate(page, 'desktop-1440');

  await page.goto(`${ORIGIN}/#/tracker`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  await page.evaluate(() => localStorage.setItem('tracker.theme', JSON.stringify('dark')));
  await page.goto(`${ORIGIN}/#/tracker`, { waitUntil: 'domcontentloaded', timeout: 8000 });

  console.log('[QA] Testing G navigation sequences...');
  await waitFor(page, `() => !!document.querySelector('.lin')`);
  const initialShot = await takeShot(page, 'desktop-team-initial');

  await page.keyboard.press('g');
  await page.keyboard.press('i');
  const onInbox = await waitFor(page, `() => location.hash.includes('/inbox')`);
  const sInbox = await takeShot(page, 'seq-g-i-inbox');
  record('G-Sequence: g i -> Inbox', onInbox, `hash=${await page.evaluate(() => location.hash)}`, sInbox);

  await page.keyboard.press('g');
  await page.keyboard.press('m');
  const onMyIssues = await waitFor(page, `() => location.hash.includes('/my-issues')`);
  const sMyIssues = await takeShot(page, 'seq-g-m-my-issues');
  record('G-Sequence: g m -> My Issues', onMyIssues, `hash=${await page.evaluate(() => location.hash)}`, sMyIssues);

  await page.keyboard.press('g');
  await page.keyboard.press('p');
  const onProjects = await waitFor(page, `() => location.hash.includes('/projects')`);
  const sProjects = await takeShot(page, 'seq-g-p-projects');
  record('G-Sequence: g p -> Projects', onProjects, `hash=${await page.evaluate(() => location.hash)}`, sProjects);

  await page.keyboard.press('g');
  await page.keyboard.press('s');
  const onSettings = await waitFor(page, `() => location.hash.includes('/settings')`);
  const sSettings = await takeShot(page, 'seq-g-s-settings');
  record('G-Sequence: g s -> Settings', onSettings, `hash=${await page.evaluate(() => location.hash)}`, sSettings);

  console.log('[QA] Testing Inbox navigation, j/k/e/Escape, and snoozed badge...');
  await page.goto(`${ORIGIN}/#/tracker/inbox`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  await waitFor(page, `() => document.body.innerText.includes('Inbox')`);

  const initialUnread = await page.evaluate(() => {
    const badge = document.querySelector('a[href*="/inbox"] .ls-badge');
    return badge ? badge.textContent?.trim() : null;
  });
  console.log('[QA] Initial sidebar unread badge:', initialUnread);

  await page.keyboard.press('j');
  const firstSelected = await waitFor(page, `() => location.hash.includes('/inbox/')`);
  const sInboxFirst = await takeShot(page, 'inbox-j-first-selected');
  record('Inbox: j key navigates to first notification', firstSelected, `hash=${await page.evaluate(() => location.hash)}`, sInboxFirst);

  const firstHash = await page.evaluate(() => location.hash);
  const selectedNoteId = firstHash.replace('#/tracker/inbox/', '');

  await page.keyboard.press('j');
  const secondSelected = await waitFor(page, `() => location.hash !== ${JSON.stringify(firstHash)} && location.hash.includes('/inbox/')`);
  const sInboxSecond = await takeShot(page, 'inbox-j-second-selected');
  record('Inbox: j key steps to next notification', secondSelected, `hash=${await page.evaluate(() => location.hash)}`, sInboxSecond);

  await page.keyboard.press('k');
  const steppedBack = await waitFor(page, `() => location.hash === ${JSON.stringify(firstHash)}`);
  const sInboxBack = await takeShot(page, 'inbox-k-step-back');
  record('Inbox: k key steps back to previous notification', steppedBack, `hash=${await page.evaluate(() => location.hash)}`, sInboxBack);

  await page.keyboard.press('e');
  const readUpdated = await waitFor(page, `async () => {
    const notes = await fetch('/api/tracker/notifications').then(r => r.json());
    return notes.items.some(n => n.id === ${JSON.stringify(selectedNoteId)} && n.read_at !== null);
  }`);
  const sInboxRead = await takeShot(page, 'inbox-e-mark-read');
  record('Inbox: e key marks notification as read', readUpdated, `${selectedNoteId} marked read`, sInboxRead);

  await page.keyboard.press('Escape');
  const escaped = await waitFor(page, `() => location.hash === '#/tracker/inbox'`);
  const sInboxEscaped = await takeShot(page, 'inbox-escape-to-list');
  record('Inbox: Escape returns to notification list', escaped, `hash=${await page.evaluate(() => location.hash)}`, sInboxEscaped);

  const tomorrow = new Date(Date.now() + 86400000).toISOString();
  await fetch(`${ORIGIN}/api/tracker/notifications/n-2`, {
    method: 'PATCH',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ snoozed_until: tomorrow })
  });

  await page.goto(`${ORIGIN}/#/tracker/inbox`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  await waitFor(page, `() => !!document.querySelector('.lin')`);
  const afterSnoozeBadge = await page.evaluate(() => {
    const badge = document.querySelector('a[href*="/inbox"] .ls-badge');
    return badge ? Number(badge.textContent?.trim()) : 0;
  });
  const unreadExcludesSnoozed = afterSnoozeBadge === 1;
  const sSnoozeBadge = await takeShot(page, 'inbox-snoozed-unread-badge');
  record('Inbox: Snoozed notification excluded from unread badge', unreadExcludesSnoozed, `badgeCount=${afterSnoozeBadge} (expected 1)`, sSnoozeBadge);

  console.log('[QA] Testing Saved View save and dirty reset...');
  await page.goto(`${ORIGIN}/#/tracker/view/${savedView.id}`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  await waitFor(page, `() => document.body.innerText.includes('High Priority View')`);
  const sViewInitial = await takeShot(page, 'view-saved-initial');

  await key(page, 'b', ['Meta']);
  await waitFor(page, `() => document.body.innerText.includes('Reset') && document.body.innerText.includes('Save changes')`, 4000);

  const dirtyShown = await page.evaluate(() => document.body.innerText.includes('Reset') && document.body.innerText.includes('Save changes'));
  const sViewDirty = await takeShot(page, 'view-dirty-controls');
  record('Saved View: Dirty state shows Reset and Save changes', dirtyShown, 'Reset & Save changes buttons visible', sViewDirty);

  if (dirtyShown) {
    await page.evaluate(() => {
      const resetBtn = [...document.querySelectorAll('button')].find(b => b.innerText.trim() === 'Reset');
      resetBtn?.click();
    });
    const resetCleared = await waitFor(page, `() => !document.body.innerText.includes('Reset')`, 4000);
    const sViewReset = await takeShot(page, 'view-after-reset');
    record('Saved View: Reset clears dirty state', resetCleared, 'Reset restored view clean state', sViewReset);

    await key(page, 'b', ['Meta']);
    await waitFor(page, `() => document.body.innerText.includes('Save changes')`, 4000);
    await page.evaluate(() => {
      const saveBtn = [...document.querySelectorAll('button')].find(b => b.innerText.trim() === 'Save changes');
      saveBtn?.click();
    });
    const saved = await waitFor(page, `async () => {
      const boot = await fetch('/api/tracker/bootstrap').then(r => r.json());
      const v = boot.views.find(x => x.id === ${JSON.stringify(savedView.id)});
      return !document.body.innerText.includes('Save changes') && v?.display?.layout === 'board';
    }`, 5000);
    const sViewSaved = await takeShot(page, 'view-after-save');
    record('Saved View: Save changes persists display/filter update', saved, 'Save changes persisted to DB', sViewSaved);
  }

  await page.goto(`${ORIGIN}/#/tracker/view/${savedView.id}`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  if (!await waitFor(page, `() => document.body.innerText.includes('High Priority View')`)) throw new Error('Saved view not ready');
  await page.evaluate(() => {
    const button = [...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Add filter' || b.getAttribute('aria-label') === 'Filter');
    if (!button) throw new Error('Filter button missing');
    button.click();
  });
  if (!await waitFor(page, `() => [...document.querySelectorAll('.lui-menu-item')].some(el => el.textContent.includes('Assignee'))`)) throw new Error('Assignee filter field missing');
  await page.evaluate(() => {
    ([...document.querySelectorAll('.lui-menu-item')].find(el => el.textContent?.includes('Assignee')) as HTMLElement).click();
  });
  if (!await waitFor(page, `() => !!document.querySelector('[role="menuitemcheckbox"]')`)) throw new Error('Assignee options missing');
  await page.evaluate(() => (document.querySelector('[role="menuitemcheckbox"]') as HTMLElement).click());
  await page.keyboard.press('Escape');
  if (!await waitFor(page, `() => document.body.innerText.includes('Save changes')`)) throw new Error('Filter change did not become dirty');
  await page.evaluate(() => {
    ([...document.querySelectorAll('button')].find(b => b.innerText.trim() === 'Save changes') as HTMLButtonElement).click();
  });
  const filterSaved = await waitFor(page, `async () => {
    const boot = await fetch('/api/tracker/bootstrap').then(r => r.json());
    const view = boot.views.find(v => v.id === ${JSON.stringify(savedView.id)});
    return Array.isArray(view.filters.assignee_ids) && view.filters.assignee_ids.length > 0 && !document.body.innerText.includes('Save changes');
  }`);
  record('Saved View: priorities then assignee filters save cleanly', filterSaved, 'Both filters persisted and dirty controls removed', await takeShot(page, 'view-filter-key-order'));

  console.log('[QA] Testing Description unmount autosave...');
  await page.goto(`${ORIGIN}/#/tracker/issue/${issue1.identifier}`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  await waitFor(page, `() => document.body.innerText.includes(${JSON.stringify(issue1.title)})`);

  await page.evaluate(() => {
    const descArea = document.querySelector('.ld-desc');
    (descArea as HTMLElement)?.click();
  });
  await waitFor(page, `() => !!document.querySelector('.ld-desc-edit textarea')`);
  const addedText = ' - Autosaved Unmount Note ' + Date.now();
  await page.keyboard.type(addedText);

  await page.goto(`${ORIGIN}/#/tracker/team/OMO/all`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  await waitFor(page, `() => !!document.querySelector('.lin')`);

  const autosavedDesc = await waitFor(page, `async () => {
    const r = await fetch('/api/tracker/issues/${issue1.identifier}').then(r => r.json());
    return Boolean(r.issue?.description?.includes('Autosaved Unmount Note'));
  }`, 6000);
  const sAutosave = await takeShot(page, 'description-unmount-autosaved');
  record('Description: Unmount autosaves pending changes', autosavedDesc, 'Description persisted on unmount before blur', sAutosave);

  console.log('[QA] Testing Milestone retention...');
  await page.goto(`${ORIGIN}/#/tracker/issue/${issue1.identifier}`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  await waitFor(page, `() => document.body.innerText.includes(${JSON.stringify(issue1.title)})`);

  const initialMilestone = await page.evaluate(async (id: string) => {
    const r = await fetch(`/api/tracker/issues/${id}`).then(r => r.json());
    return r.issue?.milestone_id;
  }, issue1.identifier);

  await page.evaluate(() => {
    const projectRow = [...document.querySelectorAll('.ld-prop')].find(el => el.querySelector('.ld-prop-label')?.textContent === 'Project')?.querySelector('button');
    if (!projectRow) throw new Error('Project property button not found');
    projectRow.click();
  });
  if (!await waitFor(page, `() => !!document.querySelector('[role="menuitem"]')`)) throw new Error('Project picker not open');
  await page.evaluate(() => {
    const option = [...document.querySelectorAll('[role="menuitem"]')].find(el => el.textContent?.includes('Alpha Initiative')) as HTMLElement;
    if (!option) throw new Error('Current project option not found');
    option.click();
  });
  if (!await waitFor(page, `() => !document.querySelector('[role="menuitem"]')`)) throw new Error('Project picker did not close');

  const milestoneAfterPatch = await page.evaluate(async (id: string) => {
    const r = await fetch(`/api/tracker/issues/${id}`).then(r => r.json());
    return r.issue?.milestone_id;
  }, issue1.identifier);

  const milestoneRetained = initialMilestone === m1.id && milestoneAfterPatch === m1.id;
  const sMilestone = await takeShot(page, 'milestone-retention');
  record('Milestone: Retained after reselecting the same project', milestoneRetained, `milestone_id=${milestoneAfterPatch}`, sMilestone);

  console.log('[QA] Testing Board reorder and drag...');
  await page.goto(`${ORIGIN}/#/tracker/team/OMO/all`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  await waitFor(page, `() => !!document.querySelector('.lin')`);
  await key(page, 'b', ['Meta']);
  await waitFor(page, `() => document.querySelectorAll('[data-board-column]').length >= 2`);
  const sBoardCols = await takeShot(page, 'board-view-columns');

  const dragSuccess = await page.evaluate(async () => {
    const card = document.querySelector('[data-issue-card]') as HTMLElement | null;
    const cols = [...document.querySelectorAll('[data-board-column]')] as HTMLElement[];
    const inProgressCol = cols.find(c => (c.getAttribute('data-board-column-name') || '').includes('In Progress'));
    return { hasCard: !!card, hasCol: !!inProgressCol };
  });

  if (dragSuccess.hasCard && dragSuccess.hasCol) {
    const dragCoords = await page.evaluate(() => {
      const card = document.querySelector('[data-issue-card]') as HTMLElement;
      const cols = [...document.querySelectorAll('[data-board-column]')] as HTMLElement[];
      const inProgressCol = cols.find(c => (c.getAttribute('data-board-column-name') || '').includes('In Progress'))!;
      const a = card.getBoundingClientRect();
      const b = inProgressCol.getBoundingClientRect();
      return { from: { x: a.x + a.width / 2, y: a.y + 15 }, to: { x: b.x + b.width / 2, y: b.y + 80 } };
    });

    await page.mouse.move(dragCoords.from.x, dragCoords.from.y);
    await page.mouse.down();
    for (let step = 1; step <= 10; step++) {
      await page.mouse.move(
        dragCoords.from.x + ((dragCoords.to.x - dragCoords.from.x) * step) / 10,
        dragCoords.from.y + ((dragCoords.to.y - dragCoords.from.y) * step) / 10
      );
    }
    const sDragging = await takeShot(page, 'board-dragging');
    await page.mouse.up();

    const dragPersisted = await waitFor(page, `async () => {
      const issues = await fetch('/api/tracker/issues').then(r => r.json());
      const boot = await fetch('/api/tracker/bootstrap').then(r => r.json());
      const inProgState = boot.states.find(s => s.name === 'In Progress');
      return issues.items.some(i => i.state_id === inProgState?.id);
    }`, 6000);

    const sAfterDrag = await takeShot(page, 'board-drag-completed');
    record('Board: Drag issue card to In Progress column', dragPersisted, 'Issue moved to In Progress and persisted', sAfterDrag);
  } else {
    record('Board: Drag issue card to In Progress column', false, 'Card or target column not found');
  }

  const boardViewKey = await page.evaluate(() => document.querySelector('[data-view-key]')?.getAttribute('data-view-key'));
  const allStates = await fetch(`${ORIGIN}/api/tracker/bootstrap`).then(r => r.json());
  const todoState = allStates.states.find((s: any) => s.name === 'Todo');
  for (const [issue, order] of [[issue1, 100], [issue2, 200]] as const) {
    const res = await fetch(`${ORIGIN}/api/tracker/issues/${issue.id}`, {
      method: 'PATCH', headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ state_id: todoState.id, sort_order: order })
    });
    if (!res.ok) throw new Error('Cannot prepare manual ordering fixture');
  }
  await page.evaluate((viewKey: string) => {
    const displays = JSON.parse(localStorage.getItem('tracker.displays') || '{}');
    displays[viewKey] = { ...displays[viewKey], layout: 'board', ordering: 'manual', grouping: 'status' };
    localStorage.setItem('tracker.displays', JSON.stringify(displays));
  }, boardViewKey);
  await page.goto('about:blank', { waitUntil: 'domcontentloaded', timeout: 8000 });
  await page.goto(`${ORIGIN}/#/tracker/team/OMO/all`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  if (!await waitFor(page, `() => document.querySelectorAll('[data-issue-card]').length >= 2`)) throw new Error('Manual board not ready');
  const reorderCoords = await page.evaluate((ids: string[]) => {
    const rect = (id: string) => {
      const el = document.querySelector(`[data-issue-id="${id}"]`)!;
      const r = el.getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
    };
    return { from: rect(ids[0]), to: rect(ids[1]) };
  }, [issue1.id, issue2.id]);
  await page.mouse.move(reorderCoords.from.x, reorderCoords.from.y);
  await page.mouse.down();
  for (let step = 1; step <= 12; step++) {
    await page.mouse.move(reorderCoords.from.x + (reorderCoords.to.x - reorderCoords.from.x) * step / 12,
      reorderCoords.from.y + (reorderCoords.to.y - reorderCoords.from.y) * step / 12);
  }
  await page.mouse.up();
  const reordered = await waitFor(page, `async () => {
    const response = await fetch('/api/tracker/issues').then(r => r.json());
    const a = response.items.find(i => i.id === ${JSON.stringify(issue1.id)});
    const b = response.items.find(i => i.id === ${JSON.stringify(issue2.id)});
    return a.state_id === b.state_id && a.sort_order > b.sort_order;
  }`);
  record('Board: Same-column manual downward reorder persists', reordered, 'first card order must exceed second card order', await takeShot(page, 'board-manual-reorder'));

  await page.evaluate((viewKey: string) => {
    const displays = JSON.parse(localStorage.getItem('tracker.displays') || '{}');
    displays[viewKey] = { ...displays[viewKey], grouping: 'label' };
    localStorage.setItem('tracker.displays', JSON.stringify(displays));
  }, boardViewKey);
  await page.goto('about:blank', { waitUntil: 'domcontentloaded', timeout: 8000 });
  await page.goto(`${ORIGIN}/#/tracker/team/OMO/all`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  if (!await waitFor(page, `() => document.querySelectorAll('[data-issue-id="${issue1.id}"]').length === 2`)) throw new Error('Multi-label issue not present in both columns');
  const labelCoords = await page.evaluate((id: string) => {
    const columns = [...document.querySelectorAll('[data-board-column]')];
    const source = columns.find(c => c.getAttribute('data-board-column-name') === 'UI')!;
    const target = columns.find(c => c.getAttribute('data-board-column-name') === 'API')!;
    const card = source.querySelector(`[data-issue-id="${id}"]`)!;
    const a = card.getBoundingClientRect(), b = target.getBoundingClientRect();
    return { from: { x: a.x + a.width / 2, y: a.y + 15 }, to: { x: b.x + b.width / 2, y: b.bottom - 30 } };
  }, issue1.id);
  await page.mouse.move(labelCoords.from.x, labelCoords.from.y);
  await page.mouse.down();
  for (let step = 1; step <= 12; step++) {
    await page.mouse.move(labelCoords.from.x + (labelCoords.to.x - labelCoords.from.x) * step / 12,
      labelCoords.from.y + (labelCoords.to.y - labelCoords.from.y) * step / 12);
  }
  await page.mouse.up();
  const labelMoved = await waitFor(page, `async () => {
    const r = await fetch('/api/tracker/issues/${issue1.identifier}').then(r => r.json());
    return r.issue.label_ids.includes(${JSON.stringify(labelB.id)}) && !r.issue.label_ids.includes(${JSON.stringify(labelA.id)});
  }`);
  record('Board: Multi-label drag removes the actual source label', labelMoved, 'UI removed and API retained', await takeShot(page, 'board-multilabel-drag'));

  console.log('[QA] Testing Mobile emulation...');
  await omowright.emulate(page, 'iphone-14');
  await page.goto(`${ORIGIN}/#/tracker/team/OMO/all`, { waitUntil: 'domcontentloaded', timeout: 8000 });
  await waitFor(page, `() => !!document.querySelector('.lin')`);
  const sMobileDark = await takeShot(page, 'mobile-team-dark');

  const noHScroll = await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 2);
  record('Mobile: No horizontal page overflow on iPhone 14', noHScroll, `scrollWidth=${await page.evaluate(() => document.documentElement.scrollWidth)}`, sMobileDark);

  const drawerToggled = await page.evaluate(() => {
    const btn = [...document.querySelectorAll('button')].find(b => /menu|navigation|sidebar/i.test(b.getAttribute('aria-label') || ''));
    if (btn) {
      btn.click();
      return true;
    }
    return false;
  });
  const drawerOpen = await waitFor(page, `() => document.body.innerText.includes('Inbox') && document.body.innerText.includes('My issues')`);
  const sMobileDrawer = await takeShot(page, 'mobile-drawer-open');
  record('Mobile: Drawer navigation toggle opens sidebar menu', drawerToggled && drawerOpen, 'Sidebar drawer opened', sMobileDrawer);

  await omowright.emulate(page, null);
  await page.close();
} catch (err: any) {
  console.error('[QA] Error during browser test execution:', err);
  record('Execution Error', false, err.message);
} finally {
  await browser.close();
  rmSync(profile, { recursive: true, force: true });
  serverProc.kill();
  rmSync(tmpDir, { recursive: true, force: true });
  console.log('[QA] Cleaned up browser profile and temporary QA database.');
}

await Bun.write(`${QA_DIR}/results.json`, JSON.stringify(results, null, 2));
console.log(`\n[QA Completed] ${results.filter(r => r.status === 'PASS').length}/${results.length} PASSED.`);
