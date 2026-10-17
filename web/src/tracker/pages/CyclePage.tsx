import { Star } from 'lucide-react'
import { useMemo } from 'react'
import { cycleLabel, cycleProgressRatio, cycleStatus, shortDate } from '../format'
import { IssueCollection } from '../issues'
import { BurnUpChart, CycleLegend, cycleIssues, cycleStats } from '../projects/cycles'
import { hrefFor } from '../router'
import { PageHeader } from '../shell/Header'
import { useTracker } from '../store'
import { IconButton } from '../ui'

const STATUS_LABEL = { current: 'Current', upcoming: 'Upcoming', past: 'Completed' } as const

export function CyclePage({ cycleId }: { cycleId: string }) {
  const t = useTracker()
  const cycle = t.cyclesById.get(cycleId)
  const issues = useMemo(() => cycleIssues(cycleId, t.issues), [cycleId, t.issues])

  if (!cycle) {
    return (
      <div className="flex h-full flex-col">
        <PageHeader title="Cycle" />
        <div className="flex flex-1 items-center justify-center text-[13px] t-3">{t.ready ? 'Cycle not found' : 'Loading...'}</div>
      </div>
    )
  }

  const team = t.teamsById.get(cycle.team_id)
  const status = cycleStatus(cycle)
  const stats = cycleStats(cycle.id, t.issues, t.statesById)
  const favorited = t.isFavorite('cycle', cycle.id)
  const daysLeft = Math.max(0, Math.ceil((new Date(cycle.ends_at).getTime() - Date.now()) / 86_400_000))

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader
        title={
          <span className="flex min-w-0 items-center gap-1.5">
            {team && (
              <>
                <span className="h-3.5 w-3.5 rounded-[4px]" style={{ background: team.color }} />
                <a href={hrefFor(`/team/${team.key}/cycles`)} className="t-3 hover:text-[var(--text-1)]">
                  {team.name}
                </a>
                <span className="t-4">›</span>
              </>
            )}
            <span className="truncate t-1">{cycleLabel(cycle)}</span>
          </span>
        }
        actions={
          <IconButton
            label={favorited ? 'Remove from favorites' : 'Add to favorites'}
            icon={<Star size={14} fill={favorited ? 'var(--warn)' : 'none'} color={favorited ? 'var(--warn)' : undefined} />}
            active={favorited}
            onClick={() => void t.toggleFavorite('cycle', cycle.id).catch(t.reportError)}
          />
        }
      />
      <div className="min-h-0 flex-1 overflow-auto">
        <div className="flex gap-6 border-b px-6 py-5 max-lg:flex-col max-md:px-4">
          <div className="flex min-w-[220px] flex-col gap-3">
            <div className="flex flex-wrap items-center gap-1.5 text-[12px]">
              <span className="rounded-[4px] px-1.5 py-0.5 t-2" style={{ background: 'var(--bg-active)' }}>
                {STATUS_LABEL[status]}
              </span>
              <span className="rounded-[4px] px-1.5 py-0.5 t-2 tabular" style={{ background: 'var(--bg-active)' }}>
                {shortDate(cycle.starts_at)} → {shortDate(cycle.ends_at)}
              </span>
            </div>
            <h1 className="text-[20px] w-590 t-1">{cycleLabel(cycle)}</h1>
            {cycle.description && <p className="text-[13px] t-3">{cycle.description}</p>}
            <p className="text-[12px] t-3">
              {status === 'current'
                ? `${daysLeft} ${daysLeft === 1 ? 'day' : 'days'} left · ${Math.round(cycleProgressRatio(cycle) * 100)}% elapsed`
                : status === 'upcoming'
                  ? `Starts ${shortDate(cycle.starts_at)}`
                  : `Ended ${shortDate(cycle.completed_at ?? cycle.ends_at)}`}
            </p>
            <div className="mt-2">
              <div className="mb-3 text-[13px] w-510 t-2">Progress</div>
              <CycleLegend stats={stats} />
            </div>
          </div>
          <div className="min-w-0 flex-1">
            <BurnUpChart cycle={cycle} issues={t.issues} />
          </div>
        </div>
        <IssueCollection
          issues={issues}
          viewKey={`cycle:${cycle.id}`}
          defaultDisplay={{ grouping: 'status' }}
          emptyState={
            <div className="p-10 text-center text-[13px] t-3">
              No issues in this cycle. Add issues from the backlog with <span className="t-2">⇧C</span>.
            </div>
          }
        />
      </div>
    </div>
  )
}

export default CyclePage
