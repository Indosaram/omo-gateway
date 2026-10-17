import { Plus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { cycleLabel, cycleProgressRatio, cycleStatus, shortDate } from '../format'
import { CreateCycleDialog, cycleStats } from '../projects/cycles'
import { ProgressBar, percent } from '../projects/shared'
import { hrefFor } from '../router'
import { PageHeader } from '../shell/Header'
import { useTracker } from '../store'
import type { Cycle } from '../types'
import { Button, CycleIcon } from '../ui'

const SECTIONS: { id: ReturnType<typeof cycleStatus>; label: string }[] = [
  { id: 'current', label: 'Current' },
  { id: 'upcoming', label: 'Upcoming' },
  { id: 'past', label: 'Past' },
]

export function CyclesPage({ teamKey }: { teamKey: string }) {
  const t = useTracker()
  const team = t.teamsByKey.get(teamKey)
  const [creating, setCreating] = useState(false)

  const cycles = useMemo(() => (team ? t.cycles.filter((c) => c.team_id === team.id) : []), [t.cycles, team])
  const grouped = useMemo(() => {
    const out: Record<string, Cycle[]> = { current: [], upcoming: [], past: [] }
    for (const c of cycles) out[cycleStatus(c)].push(c)
    out.current.sort((a, b) => a.starts_at.localeCompare(b.starts_at))
    out.upcoming.sort((a, b) => a.starts_at.localeCompare(b.starts_at))
    out.past.sort((a, b) => b.starts_at.localeCompare(a.starts_at))
    return out
  }, [cycles])

  if (!team) {
    return (
      <div className="flex h-full flex-col">
        <PageHeader title="Cycles" />
        <div className="flex flex-1 items-center justify-center text-[13px] t-3">{t.ready ? `Team ${teamKey} not found` : 'Loading...'}</div>
      </div>
    )
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader
        title={
          <span className="flex items-center gap-1.5">
            <span className="h-3.5 w-3.5 rounded-[4px]" style={{ background: team.color }} />
            <span className="t-1">{team.name}</span>
            <span className="t-4">›</span>
            <span className="t-1">Cycles</span>
          </span>
        }
        actions={
          <Button variant="secondary" size="sm" icon={<Plus size={14} />} onClick={() => setCreating(true)}>
            New cycle
          </Button>
        }
      />
      <div className="min-h-0 flex-1 overflow-auto">
        {cycles.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-3 p-10 text-center">
            <div className="text-[15px] w-510 t-1">Cycles</div>
            <p className="max-w-[380px] text-[13px] t-3">
              Cycles are time-boxed periods of work, similar to sprints. Plan upcoming work and track the team's progress.
            </p>
            <Button variant="primary" size="sm" onClick={() => setCreating(true)}>
              Create cycle
            </Button>
          </div>
        ) : (
          SECTIONS.filter((s) => grouped[s.id].length > 0).map((s) => (
            <section key={s.id}>
              <div className="flex h-[34px] items-center gap-2 border-b px-4 text-[13px]" style={{ background: 'var(--bg-group)' }}>
                <span className="w-510 t-1">{s.label}</span>
                <span className="t-3 tabular">{grouped[s.id].length}</span>
              </div>
              {grouped[s.id].map((c) => (
                <CycleRow key={c.id} cycle={c} />
              ))}
            </section>
          ))
        )}
      </div>
      <CreateCycleDialog team={team} cycles={cycles} open={creating} onOpenChange={setCreating} />
    </div>
  )
}

function CycleRow({ cycle }: { cycle: Cycle }) {
  const t = useTracker()
  const stats = cycleStats(cycle.id, t.issues, t.statesById)
  const status = cycleStatus(cycle)
  const timeRatio = cycleProgressRatio(cycle)
  return (
    <a
      href={hrefFor(`/cycle/${cycle.id}`)}
      className="lin-hover grid min-h-[52px] grid-cols-[minmax(200px,1fr)_160px_200px_120px] items-center gap-4 border-b px-4 py-2 text-[13px] max-md:grid-cols-[1fr_auto]"
    >
      <span className="flex min-w-0 items-center gap-2.5">
        <CycleIcon progress={status === 'past' ? 1 : status === 'current' ? timeRatio : 0} />
        <span className="flex min-w-0 flex-col">
          <span className="truncate w-510 t-1">{cycleLabel(cycle)}</span>
          <span className="text-[12px] t-3 tabular">
            {shortDate(cycle.starts_at)} → {shortDate(cycle.ends_at)}
          </span>
        </span>
      </span>
      <span className="text-[12px] t-3 max-md:hidden">
        {status === 'current' ? `${Math.round(timeRatio * 100)}% of time elapsed` : status === 'upcoming' ? 'Upcoming' : cycle.completed_at ? 'Completed' : 'Ended'}
      </span>
      <span className="flex items-center gap-2 max-md:hidden">
        <span className="flex-1">
          <ProgressBar ratio={stats.ratio} />
        </span>
        <span className="w-9 text-right text-[12px] t-3 tabular">{percent(stats.ratio)}</span>
      </span>
      <span className="flex justify-end gap-3 text-[12px] t-3 tabular">
        <span title="Scope">{stats.scope} scope</span>
        <span title="Completed" style={{ color: 'var(--accent-hi)' }}>
          {stats.completed} done
        </span>
      </span>
    </a>
  )
}

export default CyclesPage
