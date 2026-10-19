const BASE = process.env.TRACKER_BASE ?? 'http://127.0.0.1:9333/api/tracker'
const DB = process.env.TRACKER_DB ?? '/tmp/tracker-qa/omon.db'
const results: { id: string; pass: boolean; detail: string }[] = []

async function call(method: string, path: string, body?: unknown) {
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: body === undefined ? undefined : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  const text = await res.text()
  let json: any = null
  try {
    json = text ? JSON.parse(text) : null
  } catch {
    json = { raw: text.slice(0, 300) }
  }
  return { status: res.status, json }
}

function check(id: string, pass: boolean, detail: unknown) {
  results.push({ id, pass, detail: typeof detail === 'string' ? detail : JSON.stringify(detail).slice(0, 400) })
}

async function sql(query: string): Promise<string> {
  const proc = Bun.spawn(['sqlite3', '-cmd', '.timeout 3000', DB, query], { stdout: 'pipe', stderr: 'pipe' })
  const out = await new Response(proc.stdout).text()
  await proc.exited
  return out.trim()
}

const boot = await call('GET', '/bootstrap')
const teamKeys = (boot.json?.teams ?? []).map((t: any) => t.key)
check('S1 bootstrap teams OMO+AGT', boot.status === 200 && teamKeys.includes('OMO') && teamKeys.includes('AGT'), { status: boot.status, teamKeys })
check('S1 states >= 15', (boot.json?.states ?? []).length >= 15, { states: boot.json?.states?.length })
const omo = boot.json.teams.find((t: any) => t.key === 'OMO')
const agt = boot.json.teams.find((t: any) => t.key === 'AGT')
const omoStates = boot.json.states.filter((s: any) => s.team_id === omo.id)
const agtStates = boot.json.states.filter((s: any) => s.team_id === agt.id)
const inProgress = omoStates.find((s: any) => s.name === 'In Progress')

const created = await call('POST', '/issues', { team_id: omo.id, title: 'QA issue', priority: 2 })
check('S2 create 201 + OMO-n identifier', created.status === 201 && /^OMO-\d+$/.test(created.json?.identifier ?? ''), {
  status: created.status,
  identifier: created.json?.identifier,
})
const issue = created.json
const patched = await call('PATCH', `/issues/${issue.id}`, { state_id: inProgress.id })
check('S2 patch state sets started_at', patched.status === 200 && Boolean(patched.json?.started_at), {
  status: patched.status,
  started_at: patched.json?.started_at,
})
const detail = await call('GET', `/issues/${issue.identifier}`)
check(
  'S2 detail by identifier has state activity',
  detail.status === 200 && (detail.json?.activity ?? []).some((a: any) => a.kind === 'state'),
  { status: detail.status, kinds: (detail.json?.activity ?? []).map((a: any) => a.kind) },
)

const agtIssues = await call('GET', `/issues?team_id=${agt.id}`)
const laneCount = Number(await sql('select count(*) from lanes'))
const mirrored = (agtIssues.json?.items ?? []).filter((i: any) => i.lane_key)
check('S3 every lane mirrored as AGT issue', agtIssues.status === 200 && mirrored.length === laneCount, {
  laneCount,
  mirrored: mirrored.length,
})
const closedLane = mirrored.find((i: any) => i.lane?.status === 'closed')
const doneState = agtStates.find((s: any) => s.lane_status === 'closed')
check('S3 closed lane -> AGT Done state', Boolean(closedLane && closedLane.state_id === doneState?.id), {
  state_id: closedLane?.state_id,
  expected: doneState?.id,
  lane: closedLane?.lane,
})

if (closedLane) {
  const del = await call('DELETE', `/issues/${closedLane.id}`)
  check('S4 delete lane-linked issue -> 409', del.status === 409, { status: del.status, body: del.json })
  const reopen = await call('PATCH', `/issues/${closedLane.id}`, { state_id: agtStates.find((s: any) => s.lane_status === 'working')?.id })
  const replies = Number(await sql(`select count(*) from lane_owner_replies where lane_key='${closedLane.lane_key}'`))
  check('S4 closed lane -> Working maps to Reopen (accepted)', reopen.status === 200 && replies >= 1, { status: reopen.status, replies, body: reopen.json })
}

const workingKey = 'qa-working-lane'
await sql(
  `insert or ignore into lanes(key,title,lead_session_key,cwd,brief_path,session_key,status,done_signals,last_activity_at) values ('${workingKey}','QA working lane','discord:qa','/tmp','/tmp/none.md','lane:${workingKey}','working','file /tmp/never','${new Date().toISOString()}')`,
)
const afterInsert = await call('GET', `/issues?team_id=${agt.id}`)
const workingIssue = (afterInsert.json?.items ?? []).find((i: any) => i.lane_key === workingKey)
const workingState = agtStates.find((s: any) => s.lane_status === 'working')
check('S3 new working lane mirrored into Working', Boolean(workingIssue && workingIssue.state_id === workingState?.id), {
  found: Boolean(workingIssue),
  state_id: workingIssue?.state_id,
})
if (workingIssue) {
  const completedState = agtStates.find((s: any) => s.lane_status === 'closed')
  const notVerified = await call('PATCH', `/issues/${workingIssue.id}`, { state_id: completedState.id })
  check('S4 close unverified lane -> 409 lane_not_verified', notVerified.status === 409 && notVerified.json?.code === 'lane_not_verified', {
    status: notVerified.status,
    body: notVerified.json,
  })
  const queued = await call('PATCH', `/issues/${workingIssue.id}`, { state_id: agtStates.find((s: any) => s.lane_status === 'queued')?.id })
  check('S4 working lane -> Queued -> 409 illegal_lane_transition', queued.status === 409 && queued.json?.code === 'illegal_lane_transition', { status: queued.status, body: queued.json })
  const laneStill = await sql(`select status from lanes where key='${workingKey}'`)
  check('S4 rejected transition leaves lane unchanged', laneStill === 'working', laneStill)
  const prio = await call('PATCH', `/issues/${workingIssue.id}`, { priority: 1 })
  check('S4 lane-linked non-state field editable', prio.status === 200 && prio.json?.priority === 1, { status: prio.status })
}

const comment = await call('POST', `/issues/${issue.id}/comments`, { body: 'QA comment' })
check('S5 comment create', comment.status === 200 || comment.status === 201, { status: comment.status })
const reaction = await call('POST', `/comments/${comment.json?.id}/reactions`, { emoji: '👍' })
check('S5 reaction toggle', reaction.status === 200 && (reaction.json?.reactions ?? []).length === 1, { status: reaction.status, r: reaction.json?.reactions })
const other = await call('POST', '/issues', { team_id: omo.id, title: 'QA blocker' })
const rel = await call('POST', `/issues/${issue.id}/relations`, { type: 'blocks', related_issue_id: other.json?.id })
check('S5 relation create', rel.status === 200 || rel.status === 201, { status: rel.status })
const otherDetail = await call('GET', `/issues/${other.json?.id}`)
check('S5 relation direction blocked_by', (otherDetail.json?.relations ?? []).some((r: any) => r.direction === 'blocked_by'), {
  dirs: (otherDetail.json?.relations ?? []).map((r: any) => r.direction),
})
const link = await call('POST', `/issues/${issue.id}/links`, { url: 'https://example.com/spec', title: 'Spec' })
check('S5 link create', link.status === 200 || link.status === 201, { status: link.status })
const sub = await call('POST', `/issues/${issue.id}/subscribe`, { subscribed: true })
check('S5 subscribe', sub.status === 200 && sub.json?.subscribed === true, sub.json)
const project = await call('POST', '/projects', { name: 'QA project', status: 'started', color: '#5e6ad2', icon: 'box' })
check('S5 project create', project.status === 200 || project.status === 201, { status: project.status })
const upd = await call('POST', `/projects/${project.json?.id}/updates`, { health: 'on_track', body: 'All good' })
check('S5 project update', upd.status === 200 || upd.status === 201, { status: upd.status })
const ms = await call('POST', `/projects/${project.json?.id}/milestones`, { name: 'Beta', target_date: '2026-11-01' })
check('S5 milestone', ms.status === 200 || ms.status === 201, { status: ms.status })
const assignProject = await call('PATCH', `/issues/${issue.id}`, { project_id: project.json?.id })
check('S5 issue -> project', assignProject.json?.project_id === project.json?.id, { status: assignProject.status })
const projDetail = await call('GET', `/projects/${project.json?.id}`)
check('S5 project detail counts', projDetail.status === 200 && projDetail.json?.project?.issue_count >= 1, projDetail.json?.project)
const cycle = await call('POST', '/cycles', {
  team_id: omo.id,
  starts_at: new Date(Date.now() - 86_400_000).toISOString(),
  ends_at: new Date(Date.now() + 13 * 86_400_000).toISOString(),
})
check('S5 cycle create', cycle.status === 200 || cycle.status === 201, { status: cycle.status, n: cycle.json?.number })
const label = await call('POST', '/labels', { name: 'QA label', color: '#eb5757' })
check('S5 label create', label.status === 200 || label.status === 201, { status: label.status })
const batch = await call('POST', '/issues/batch', { ids: [issue.id, other.json?.id], patch: { add_label_ids: [label.json?.id], cycle_id: cycle.json?.id } })
check('S5 batch update', batch.status === 200 && batch.json?.items?.every((i: any) => i.label_ids.includes(label.json?.id) && i.cycle_id === cycle.json?.id), {
  status: batch.status,
})
const view = await call('POST', '/views', {
  name: 'QA high',
  filters: { priorities: [2] },
  display: { layout: 'board', grouping: 'status', sub_grouping: 'none', ordering: 'priority', show_completed: 'all', show_sub_issues: true, show_empty_groups: false, properties: ['id'] },
})
check('S5 view create + JSON filters', (view.status === 200 || view.status === 201) && view.json?.filters?.priorities?.[0] === 2, { status: view.status, f: view.json?.filters })
const fav = await call('POST', '/favorites', { kind: 'view', target_id: view.json?.id })
check('S5 favorite toggle', fav.status === 200 && fav.json?.favorited === true, fav.json)
const boot2 = await call('GET', '/bootstrap')
check('S5 bootstrap round-trip', ['projects', 'cycles', 'labels', 'views', 'favorites'].every((k) => (boot2.json?.[k] ?? []).length > 0), {
  counts: Object.fromEntries(['projects', 'cycles', 'labels', 'views', 'favorites'].map((k) => [k, boot2.json?.[k]?.length])),
})
const notes = await call('GET', '/notifications')
check('S5 notifications list', notes.status === 200 && Array.isArray(notes.json?.items), { status: notes.status, n: notes.json?.items?.length })
if (notes.json?.items?.length) {
  const n0 = notes.json.items[0]
  const read = await call('PATCH', `/notifications/${n0.id}`, { read: true })
  check('S5 notification mark read', read.status === 200 && Boolean(read.json?.read_at), { status: read.status })
}
const readAll = await call('POST', '/notifications/read-all')
check('S5 read-all', readAll.status === 200 && typeof readAll.json?.updated === 'number', readAll.json)
const search = await call('GET', '/search?q=QA')
check('S6 search finds QA issue', search.status === 200 && (search.json?.issues ?? []).some((i: any) => i.id === issue.id), {
  status: search.status,
  n: search.json?.issues?.length,
})
const missing = await call('GET', '/issues/does-not-exist')
check('S5 unknown issue -> 404', missing.status === 404, { status: missing.status })
const bad = await call('POST', '/issues', { team_id: omo.id, title: '' })
check('S5 empty title -> 400', bad.status === 400, { status: bad.status })

const omoIssueForDelegate = await call('POST', '/issues', { team_id: omo.id, title: 'QA delegate', description: 'delegate me' })
const delegated = await call('POST', `/issues/${omoIssueForDelegate.json?.id}/delegate`, { cwd: '/tmp/tracker-qa', done: process.env.TRACKER_DONE_SIGNAL ?? 'file /tmp/tracker-qa/never-done', model: 'small' })
const delegatedKey = delegated.json?.issue?.lane_key ?? delegated.json?.lane_key
const laneRow = delegatedKey ? await sql(`select key||'|'||status from lanes where key='${delegatedKey}'`) : ''
check('S3 delegate creates lanes row + links issue', delegated.status === 200 && Boolean(laneRow), { status: delegated.status, delegatedKey, laneRow, body: delegated.status === 200 ? undefined : delegated.json })
const delegatedAct = await call('GET', `/issues/${omoIssueForDelegate.json?.id}`)
check('S3 delegated activity row', (delegatedAct.json?.activity ?? []).some((a: any) => a.kind === 'delegated'), { kinds: (delegatedAct.json?.activity ?? []).map((a: any) => a.kind) })

await sql(`delete from tracker_issues where lane_key='${workingKey}'`).catch(() => '')
await sql(`delete from lanes where key='${workingKey}'`)

const failed = results.filter((r) => !r.pass)
for (const r of results) console.log(`${r.pass ? 'PASS' : 'FAIL'} ${r.id} ${r.pass ? '' : r.detail}`)
console.log(`\n${results.length - failed.length}/${results.length} passed`)
await Bun.write('/tmp/tracker-qa/api-qa.json', JSON.stringify(results, null, 2))
process.exit(failed.length ? 1 : 0)
