import { useMemo } from 'react'
import { parseDateOnly, shortDate, toDateOnly } from '../format'
import type { IssueGroup, TrackerState } from '../store'
import type { Issue } from '../types'
import { useCollection } from './model'
import { GroupIcon, IssueStatusIcon } from './shared'

const DAY = 86_400_000
const DAY_W = 28

interface Span {
  start: number
  end: number
  source: 'due' | 'cycle'
}

function startOfDay(ms: number): number {
  const d = new Date(ms)
  d.setHours(0, 0, 0, 0)
  return d.getTime()
}

function issueSpan(issue: Issue, t: TrackerState): Span | null {
  const cycle = issue.cycle_id ? t.cyclesById.get(issue.cycle_id) : undefined
  if (issue.due_date) {
    const end = parseDateOnly(issue.due_date).getTime()
    const startCandidate = startOfDay(new Date(issue.started_at ?? issue.created_at).getTime())
    const start = Math.min(startCandidate, end)
    return { start, end, source: 'due' }
  }
  if (cycle) {
    return { start: startOfDay(new Date(cycle.starts_at).getTime()), end: startOfDay(new Date(cycle.ends_at).getTime()), source: 'cycle' }
  }
  return null
}

export function TimelineView({ groups, t }: { groups: IssueGroup[]; t: TrackerState }) {
  const c = useCollection()
  const spans = useMemo(() => {
    const map = new Map<string, Span>()
    for (const group of groups) for (const issue of group.issues) {
      const span = issueSpan(issue, t)
      if (span) map.set(issue.id, span)
    }
    return map
  }, [groups, t])

  const today = startOfDay(Date.now())
  const values = [...spans.values()]
  const min = Math.min(today - 7 * DAY, ...values.map((s) => s.start))
  const max = Math.max(today + 28 * DAY, ...values.map((s) => s.end))
  const rangeStart = startOfDay(min - 3 * DAY)
  const days = Math.round((startOfDay(max + 7 * DAY) - rangeStart) / DAY)
  const width = days * DAY_W
  const x = (ms: number) => Math.round((ms - rangeStart) / DAY) * DAY_W

  const months: { label: string; left: number }[] = []
  const ticks: { label: string; left: number; today: boolean; weekend: boolean }[] = []
  for (let i = 0; i < days; i++) {
    // Step by calendar day, not by 24h: a DST fall-back would otherwise repeat the previous date.
    const date = new Date(rangeStart)
    date.setDate(date.getDate() + i)
    if (date.getDate() === 1 || i === 0) {
      months.push({ label: date.toLocaleDateString('en-US', { month: 'long', year: 'numeric' }), left: i * DAY_W })
    }
    ticks.push({
      label: String(date.getDate()),
      left: i * DAY_W,
      today: toDateOnly(date) === toDateOnly(new Date(today)),
      weekend: date.getDay() === 0 || date.getDay() === 6,
    })
  }

  return (
    <div className="lic-timeline">
      <div className="lic-tl-side">
        <div className="lic-tl-head lic-tl-head--side t-3">Issues</div>
        {groups.map((group) => (
          <div key={group.key}>
            {group.grouping !== 'none' && (
              <div className="lic-tl-group">
                <GroupIcon group={group} t={t} />
                <span className="w-510">{group.label}</span>
                <span className="t-3 tabular">{group.issues.length}</span>
              </div>
            )}
            {group.issues.map((issue) => (
              <div
                key={issue.id}
                className={`lic-tl-row${c.focusedId === issue.id ? ' lic-row--focused' : ''}${c.selection.has(issue.id) ? ' lic-row--selected' : ''}`}
                data-issue-row={issue.identifier}
                data-issue-id={issue.id}
                tabIndex={c.focusedId === issue.id ? 0 : -1}
                onClick={(event) => c.onRowClick(issue, event)}
                onMouseMove={() => c.onRowHover(issue)}
              >
                <span className="lic-row-id t-3 tabular">{issue.identifier}</span>
                <IssueStatusIcon issue={issue} t={t} />
                <span className="lic-row-title-text w-510">{issue.title}</span>
              </div>
            ))}
          </div>
        ))}
      </div>
      <div className="lic-tl-scroll">
        <div className="lic-tl-canvas" style={{ width }}>
          <div className="lic-tl-head">
            <div className="lic-tl-months">
              {months.map((m) => (
                <span key={m.left} className="lic-tl-month w-510" style={{ left: m.left }}>
                  {m.label}
                </span>
              ))}
            </div>
            <div className="lic-tl-days">
              {ticks.map((tick) => (
                <span
                  key={tick.left}
                  className={`lic-tl-day t-4 tabular${tick.today ? ' lic-tl-day--today' : ''}`}
                  style={{ left: tick.left, width: DAY_W }}
                >
                  {tick.label}
                </span>
              ))}
            </div>
          </div>
          <div className="lic-tl-grid">
            {ticks
              .filter((tick) => tick.weekend)
              .map((tick) => (
                <span key={tick.left} className="lic-tl-weekend" style={{ left: tick.left, width: DAY_W }} />
              ))}
            <span className="lic-tl-today" style={{ left: x(today) + DAY_W / 2 }} />
            {groups.map((group) => (
              <div key={group.key}>
                {group.grouping !== 'none' && <div className="lic-tl-group lic-tl-group--canvas" />}
                {group.issues.map((issue) => {
                  const span = spans.get(issue.id)
                  const state = t.statesById.get(issue.state_id)
                  return (
                    <div key={issue.id} className="lic-tl-lane">
                      {span && (
                        <button
                          type="button"
                          className={`lic-tl-bar lic-tl-bar--${span.source}`}
                          style={
                            {
                              left: x(span.start),
                              width: Math.max(DAY_W, x(span.end) - x(span.start) + DAY_W),
                              '--bar': state?.color ?? 'var(--accent)',
                            } as React.CSSProperties
                          }
                          title={`${issue.identifier} · ${shortDate(toDateOnly(new Date(span.start)))} – ${shortDate(toDateOnly(new Date(span.end)))}`}
                          onClick={(event) => c.onRowClick(issue, event)}
                        >
                          <span className="lic-tl-bar-label">{issue.title}</span>
                        </button>
                      )}
                    </div>
                  )
                })}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
