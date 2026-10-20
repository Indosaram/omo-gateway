import { useState } from 'react'
import { trackerApi } from '../api'
import { toDateOnly } from '../format'
import { useTracker } from '../store'
import type { Cycle, ID, Issue, Team, WorkflowState } from '../types'
import { Button, Dialog } from '../ui'

const DAY = 86_400_000

export interface CycleStats {
  scope: number
  started: number
  completed: number
  ratio: number
}

export function cycleIssues(cycleId: ID, issues: Issue[]): Issue[] {
  return issues.filter((i) => i.cycle_id === cycleId && !i.archived_at)
}

export function cycleStats(cycleId: ID, issues: Issue[], statesById: Map<ID, WorkflowState>): CycleStats {
  let scope = 0
  let started = 0
  let completed = 0
  for (const issue of cycleIssues(cycleId, issues)) {
    const type = statesById.get(issue.state_id)?.type
    if (type === 'canceled') continue
    scope += 1
    if (type === 'completed') completed += 1
    else if (type === 'started') started += 1
  }
  return { scope, started, completed, ratio: scope ? completed / scope : 0 }
}

export interface BurnPoint {
  t: number
  scope: number
  started: number
  completed: number
}

export function burnUpSeries(cycle: Cycle, issues: Issue[], statesById: Map<ID, WorkflowState>, now = Date.now()): BurnPoint[] {
  const start = new Date(cycle.starts_at).getTime()
  const end = new Date(cycle.ends_at).getTime()
  const last = Math.min(end, now)
  const own = cycleIssues(cycle.id, issues).filter((i) => statesById.get(i.state_id)?.type !== 'canceled')
  const at = (value: string | null) => (value ? new Date(value).getTime() : Infinity)
  const points: BurnPoint[] = []
  if (last >= start) {
    for (let t = start; ; t += DAY) {
      const tick = Math.min(t, last)
      points.push({
        t: tick,
        scope: own.filter((i) => Math.max(at(i.created_at), start) <= tick).length,
        started: own.filter((i) => Math.min(at(i.started_at), at(i.completed_at)) <= tick).length,
        completed: own.filter((i) => at(i.completed_at) <= tick).length,
      })
      if (tick === last) break
    }
  }
  if (points.length === 0) points.push({ t: start, scope: own.length, started: 0, completed: 0 })
  return points
}

export function BurnUpChart({ cycle, issues }: { cycle: Cycle; issues: Issue[] }) {
  const t = useTracker()
  const series = burnUpSeries(cycle, issues, t.statesById)
  const W = 560
  const H = 200
  const P = { l: 8, r: 8, t: 12, b: 24 }
  const start = new Date(cycle.starts_at).getTime()
  const end = Math.max(new Date(cycle.ends_at).getTime(), start + DAY)
  const maxY = Math.max(1, ...series.map((p) => p.scope))
  const x = (v: number) => P.l + ((v - start) / (end - start)) * (W - P.l - P.r)
  const y = (v: number) => H - P.b - (v / maxY) * (H - P.t - P.b)
  const line = (key: keyof Omit<BurnPoint, 't'>) =>
    series.map((p, i) => `${i === 0 ? 'M' : 'L'}${x(p.t).toFixed(1)},${y(p[key]).toFixed(1)}`).join(' ')
  const lastPoint = series[series.length - 1]
  const startedArea = `${line('started')} L${x(lastPoint.t).toFixed(1)},${y(0)} L${x(series[0].t).toFixed(1)},${y(0)} Z`
  const ticks: number[] = []
  const days = Math.round((end - start) / DAY)
  const step = Math.max(1, Math.ceil(days / 5))
  for (let d = 0; d <= days; d += step) ticks.push(start + d * DAY)

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="h-auto w-full" role="img" aria-label="Cycle burn-up chart">
      <defs>
        <linearGradient id={`burn-${cycle.id}`} x1="0" x2="0" y1="0" y2="1">
          <stop offset="0" stopColor="var(--warn)" stopOpacity="0.14" />
          <stop offset="1" stopColor="var(--warn)" stopOpacity="0" />
        </linearGradient>
      </defs>
      <rect x={P.l} y={P.t} width={W - P.l - P.r} height={H - P.t - P.b} fill="var(--bg-subtle)" />
      <line x1={x(start)} x2={x(end)} y1={y(maxY)} y2={y(maxY)} stroke="var(--border-strong)" strokeDasharray="3 3" />
      <line
        x1={x(start)}
        y1={y(0)}
        x2={x(end)}
        y2={y(series[series.length - 1].scope)}
        stroke="var(--accent)"
        strokeOpacity="0.6"
        strokeDasharray="3 3"
      />
      <path d={startedArea} fill={`url(#burn-${cycle.id})`} />
      <path d={line('scope')} fill="none" stroke="var(--text-3)" strokeWidth="1.5" />
      <path d={line('started')} fill="none" stroke="var(--warn)" strokeWidth="1.5" />
      <path d={line('completed')} fill="none" stroke="var(--accent)" strokeWidth="1.5" />
      <circle cx={x(lastPoint.t)} cy={y(lastPoint.started)} r="3" fill="var(--warn)" />
      <circle cx={x(lastPoint.t)} cy={y(lastPoint.completed)} r="3" fill="var(--accent)" />
      <line x1={P.l} x2={W - P.r} y1={H - P.b} y2={H - P.b} stroke="var(--border-strong)" />
      {ticks.map((tick) => (
        <g key={tick}>
          <line x1={x(tick)} x2={x(tick)} y1={H - P.b} y2={H - P.b + 4} stroke="var(--text-4)" />
          <text x={x(tick)} y={H - 6} textAnchor="middle" fontSize="11" fill="var(--text-3)">
            {new Date(tick).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}
          </text>
        </g>
      ))}
    </svg>
  )
}

export function CycleLegend({ stats }: { stats: CycleStats }) {
  const pct = (n: number) => (stats.scope ? Math.round((n / stats.scope) * 100) : 0)
  const item = (label: string, color: string, value: number, showPct: boolean) => (
    <div className="flex flex-col gap-0.5">
      <span className="flex items-center gap-1.5 text-[12px] t-3">
        <span className="h-2 w-2 rounded-[2px]" style={{ background: color }} />
        {label}
      </span>
      <span className="pl-3.5 text-[13px] t-1 tabular">
        {value}
        {showPct && <span className="t-3"> · {pct(value)}%</span>}
      </span>
    </div>
  )
  return (
    <div className="flex gap-8">
      {item('Scope', 'var(--text-3)', stats.scope, false)}
      {item('Started', 'var(--warn)', stats.started, true)}
      {item('Completed', 'var(--accent)', stats.completed, true)}
    </div>
  )
}

export function CreateCycleDialog({
  team,
  cycles,
  open,
  onOpenChange,
}: {
  team: Team
  cycles: Cycle[]
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const t = useTracker()
  const latestEnd = cycles.reduce((max, c) => Math.max(max, new Date(c.ends_at).getTime()), 0)
  const defaultStart = latestEnd > Date.now() ? new Date(latestEnd) : new Date()
  const [name, setName] = useState('')
  const [startsAt, setStartsAt] = useState(() => toDateOnly(defaultStart))
  const [endsAt, setEndsAt] = useState(() => toDateOnly(new Date(defaultStart.getTime() + (team.cycle_weeks || 2) * 7 * DAY)))
  const [saving, setSaving] = useState(false)

  // The dialog stays mounted while closed, so re-seed the default range each time it opens.
  const [wasOpen, setWasOpen] = useState(open)
  if (open !== wasOpen) {
    setWasOpen(open)
    if (open) {
      setStartsAt(toDateOnly(defaultStart))
      setEndsAt(toDateOnly(new Date(defaultStart.getTime() + (team.cycle_weeks || 2) * 7 * DAY)))
    }
  }

  const valid = startsAt && endsAt && endsAt > startsAt

  const submit = async () => {
    if (!valid || saving) return
    setSaving(true)
    try {
      const cycle = await trackerApi.createCycle({
        team_id: team.id,
        starts_at: new Date(`${startsAt}T00:00:00`).toISOString(),
        ends_at: new Date(`${endsAt}T00:00:00`).toISOString(),
        name: name.trim() || null,
      })
      t.upsertCycle(cycle)
      onOpenChange(false)
      setName('')
    } catch (err) {
      t.reportError(err)
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={`New cycle · ${team.name}`}
      width={460}
      footer={
        <div className="flex justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" disabled={!valid || saving} onClick={() => void submit()}>
            Create cycle
          </Button>
        </div>
      }
    >
      <div className="flex flex-col gap-3 text-[13px]">
        <input
          autoFocus
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Cycle name (optional)"
          className="h-9 rounded-[6px] border px-2"
          style={{ background: 'var(--input)' }}
        />
        <div className="flex flex-wrap gap-3">
          <label className="flex items-center gap-2 t-3">
            Start
            <input
              type="date"
              value={startsAt}
              onChange={(e) => setStartsAt(e.target.value)}
              className="h-8 rounded-[6px] border px-2 t-2"
              style={{ background: 'var(--input)' }}
            />
          </label>
          <label className="flex items-center gap-2 t-3">
            End
            <input
              type="date"
              value={endsAt}
              onChange={(e) => setEndsAt(e.target.value)}
              className="h-8 rounded-[6px] border px-2 t-2"
              style={{ background: 'var(--input)' }}
            />
          </label>
        </div>
        {!valid && <p className="text-[12px]" style={{ color: 'var(--danger)' }}>End date must be after the start date.</p>}
      </div>
    </Dialog>
  )
}
