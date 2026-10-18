import type { ReactNode } from 'react'
import type { ID, Issue, Project, ProjectHealth, ProjectStatus, WorkflowState } from '../types'
import { HEALTH_LABELS, PROJECT_STATUS_LABELS } from '../types'

export const PROJECT_STATUSES: ProjectStatus[] = ['backlog', 'planned', 'started', 'paused', 'completed', 'canceled']
export const HEALTHS: ProjectHealth[] = ['on_track', 'at_risk', 'off_track']

export const HEALTH_COLORS: Record<ProjectHealth, string> = {
  on_track: 'var(--ok)',
  at_risk: 'var(--warn)',
  off_track: 'var(--danger)',
}

export const PROJECT_STATUS_COLORS: Record<ProjectStatus, string> = {
  backlog: '#bec2c8',
  planned: '#e2e2e2',
  started: '#f2c94c',
  paused: '#95a2b3',
  completed: '#5e6ad2',
  canceled: '#95a2b3',
}

export const PROJECT_COLORS = ['#5e6ad2', '#4ea7fc', '#4cb782', '#f2c94c', '#fc7840', '#eb5757', '#bb87fc', '#95a2b3']

/** Completed / total over the project's non-archived, non-canceled issues. */
export function projectIssueStats(
  projectId: ID,
  issues: Issue[],
  statesById: Map<ID, WorkflowState>,
): { total: number; completed: number; started: number; ratio: number } {
  let total = 0
  let completed = 0
  let started = 0
  for (const issue of issues) {
    if (issue.project_id !== projectId || issue.archived_at) continue
    const type = statesById.get(issue.state_id)?.type
    if (type === 'canceled') continue
    total += 1
    if (type === 'completed') completed += 1
    else if (type === 'started') started += 1
  }
  return { total, completed, started, ratio: total === 0 ? 0 : completed / total }
}

export function projectProgress(project: Project, issues: Issue[], statesById: Map<ID, WorkflowState>): number {
  const stats = projectIssueStats(project.id, issues, statesById)
  if (stats.total > 0) return stats.ratio
  return project.issue_count > 0 ? project.completed_issue_count / project.issue_count : 0
}

export function percent(ratio: number): string {
  return `${Math.round(ratio * 100)}%`
}

export function ProjectStatusIcon({ status, size = 14 }: { status: ProjectStatus; size?: number }) {
  const color = PROJECT_STATUS_COLORS[status]
  const r = 5
  const c = 2 * Math.PI * r
  const fill = status === 'started' ? 0.5 : status === 'completed' ? 1 : 0
  return (
    <svg width={size} height={size} viewBox="0 0 14 14" aria-hidden="true" style={{ flexShrink: 0 }}>
      <circle
        cx="7"
        cy="7"
        r={r}
        fill="none"
        stroke={color}
        strokeWidth="1.5"
        strokeDasharray={status === 'backlog' ? '1.6 1.6' : undefined}
      />
      {status === 'paused' && (
        <>
          <rect x="5" y="4.5" width="1.3" height="5" rx="0.5" fill={color} />
          <rect x="7.7" y="4.5" width="1.3" height="5" rx="0.5" fill={color} />
        </>
      )}
      {status === 'canceled' && <path d="M5 5l4 4M9 5l-4 4" stroke={color} strokeWidth="1.4" strokeLinecap="round" />}
      {fill > 0 && fill < 1 && (
        <circle
          cx="7"
          cy="7"
          r={r / 2}
          fill="none"
          stroke={color}
          strokeWidth={r}
          strokeDasharray={`${(c / 2) * fill} ${c}`}
          transform="rotate(-90 7 7)"
        />
      )}
      {fill === 1 && (
        <>
          <circle cx="7" cy="7" r={r} fill={color} />
          <path d="M4.8 7.1l1.5 1.5 2.9-3" fill="none" stroke="var(--panel)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
        </>
      )}
    </svg>
  )
}

export function ProjectStatusLabel({ status }: { status: ProjectStatus }) {
  return (
    <span className="inline-flex items-center gap-1.5 t-2">
      <ProjectStatusIcon status={status} />
      {PROJECT_STATUS_LABELS[status]}
    </span>
  )
}

export function HealthIcon({ health, size = 14 }: { health: ProjectHealth | null; size?: number }) {
  const color = health ? HEALTH_COLORS[health] : 'var(--text-4)'
  return (
    <svg width={size} height={size} viewBox="0 0 14 14" aria-hidden="true" style={{ flexShrink: 0 }}>
      <circle cx="7" cy="7" r="6" fill="none" stroke={color} strokeWidth="1.2" strokeDasharray={health ? undefined : '1.5 1.5'} />
      <path d="M3.5 7.5h1.8l1.2-2.5 1.6 4 1.1-2h1.3" fill="none" stroke={color} strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

export function HealthLabel({ health }: { health: ProjectHealth | null }) {
  return (
    <span className="inline-flex items-center gap-1.5" style={{ color: health ? HEALTH_COLORS[health] : 'var(--text-3)' }}>
      <HealthIcon health={health} />
      {health ? HEALTH_LABELS[health] : 'No updates'}
    </span>
  )
}

export function ProgressRing({ ratio, size = 14 }: { ratio: number; size?: number }) {
  const r = 5
  const c = 2 * Math.PI * r
  return (
    <svg width={size} height={size} viewBox="0 0 14 14" aria-hidden="true" style={{ flexShrink: 0 }}>
      <circle cx="7" cy="7" r={r} fill="none" stroke="var(--border-strong)" strokeWidth="2" />
      <circle
        cx="7"
        cy="7"
        r={r}
        fill="none"
        stroke="var(--accent)"
        strokeWidth="2"
        strokeDasharray={`${c * Math.max(0, Math.min(1, ratio))} ${c}`}
        strokeLinecap="round"
        transform="rotate(-90 7 7)"
      />
    </svg>
  )
}

export function ProgressBar({ ratio, color = 'var(--accent)' }: { ratio: number; color?: string }) {
  return (
    <div className="h-1 w-full overflow-hidden rounded-full" style={{ background: 'var(--bg-active)' }}>
      <div
        className="h-full rounded-full"
        style={{ width: '100%', background: color, transform: `scaleX(${Math.max(0, Math.min(1, ratio))})`, transformOrigin: 'left', transition: 'transform 180ms var(--ease)' }}
      />
    </div>
  )
}

export function ChoiceChip({
  active,
  onClick,
  children,
}: {
  active: boolean
  onClick: () => void
  children: ReactNode
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="inline-flex h-7 items-center gap-1.5 rounded-[6px] border px-2 text-[12px]"
      style={{
        borderColor: active ? 'var(--border-strong)' : 'var(--border)',
        background: active ? 'var(--bg-active)' : 'transparent',
        color: active ? 'var(--text-1)' : 'var(--text-2)',
      }}
    >
      {children}
    </button>
  )
}
