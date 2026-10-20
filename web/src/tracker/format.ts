import type { Cycle, Issue, Priority, StateType, User, WorkflowState } from './types'
import { PRIORITY_LABELS } from './types'

const DAY = 86_400_000

function startOfDay(date: Date): Date {
  const copy = new Date(date)
  copy.setHours(0, 0, 0, 0)
  return copy
}

export function parseDateOnly(value: string): Date {
  const [y, m, d] = value.split('-').map(Number)
  return new Date(y, (m ?? 1) - 1, d ?? 1)
}

export function toDateOnly(date: Date): string {
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}

export function shortDate(value: string | null | undefined): string {
  if (!value) return ''
  const date = value.length === 10 ? parseDateOnly(value) : new Date(value)
  const now = new Date()
  const opts: Intl.DateTimeFormatOptions =
    date.getFullYear() === now.getFullYear() ? { month: 'short', day: 'numeric' } : { month: 'short', day: 'numeric', year: 'numeric' }
  return date.toLocaleDateString('en-US', opts)
}

export function fullDate(value: string | null | undefined): string {
  if (!value) return ''
  const date = new Date(value)
  return date.toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  })
}

export function relativeTime(value: string | null | undefined, now = Date.now()): string {
  if (!value) return ''
  const diff = now - new Date(value).getTime()
  const abs = Math.abs(diff)
  const suffix = diff >= 0 ? 'ago' : 'from now'
  if (abs < 60_000) return 'just now'
  if (abs < 3_600_000) return `${Math.round(abs / 60_000)} min ${suffix}`
  if (abs < DAY) {
    const h = Math.round(abs / 3_600_000)
    return `${h} ${h === 1 ? 'hour' : 'hours'} ${suffix}`
  }
  if (abs < 30 * DAY) {
    const d = Math.round(abs / DAY)
    return `${d} ${d === 1 ? 'day' : 'days'} ${suffix}`
  }
  if (abs < 365 * DAY) {
    const mo = Math.round(abs / (30 * DAY))
    return `${mo} ${mo === 1 ? 'month' : 'months'} ${suffix}`
  }
  const y = Math.round(abs / (365 * DAY))
  return `${y} ${y === 1 ? 'year' : 'years'} ${suffix}`
}

export function compactRelative(value: string | null | undefined, now = Date.now()): string {
  if (!value) return ''
  const abs = Math.abs(now - new Date(value).getTime())
  if (abs < 3_600_000) return `${Math.max(1, Math.round(abs / 60_000))}m`
  if (abs < DAY) return `${Math.round(abs / 3_600_000)}h`
  if (abs < 30 * DAY) return `${Math.round(abs / DAY)}d`
  if (abs < 365 * DAY) return `${Math.round(abs / (30 * DAY))}mo`
  return `${Math.round(abs / (365 * DAY))}y`
}

export type DueState = 'overdue' | 'soon' | 'later' | null

export function dueState(due: string | null, completed: boolean): DueState {
  if (!due || completed) return null
  const today = startOfDay(new Date()).getTime()
  const target = parseDateOnly(due).getTime()
  if (target < today) return 'overdue'
  if (target - today <= 2 * DAY) return 'soon'
  return 'later'
}

export function dueShortcuts(now = new Date()): { id: string; label: string; date: string }[] {
  const today = startOfDay(now)
  // Calendar math, not `today.getTime() + days * DAY`: adding fixed 24h steps lands on 23:00 of
  // the previous day whenever the range crosses a DST change, which shifts the date back a day.
  const add = (days: number) => {
    const target = new Date(today)
    target.setDate(target.getDate() + days)
    return toDateOnly(target)
  }
  const endOfWeek = 5 - today.getDay()
  return [
    { id: 'today', label: 'Today', date: add(0) },
    { id: 'tomorrow', label: 'Tomorrow', date: add(1) },
    { id: 'eow', label: 'End of this week', date: add(endOfWeek >= 0 ? endOfWeek : endOfWeek + 7) },
    { id: 'week', label: 'In one week', date: add(7) },
    { id: 'two-weeks', label: 'In two weeks', date: add(14) },
    { id: 'month', label: 'In one month', date: add(30) },
  ]
}

export function priorityLabel(priority: Priority): string {
  return PRIORITY_LABELS[priority]
}

export function stateProgress(state: WorkflowState, teamStates: WorkflowState[]): number {
  if (state.type === 'completed') return 1
  if (state.type !== 'started') return 0
  const started = teamStates.filter((s) => s.type === 'started').sort((a, b) => a.position - b.position)
  const index = started.findIndex((s) => s.id === state.id)
  return started.length === 0 ? 0.5 : (index + 1) / (started.length + 1)
}

export function isClosedType(type: StateType): boolean {
  return type === 'completed' || type === 'canceled'
}

export function branchName(issue: Issue, me: User | null): string {
  const slug = issue.title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48)
  const owner = (me?.display_name || me?.name || 'me').toLowerCase().replace(/[^a-z0-9]+/g, '')
  return `${owner}/${issue.identifier.toLowerCase()}-${slug}`
}

export function cycleLabel(cycle: Cycle): string {
  return cycle.name?.trim() ? cycle.name : `Cycle ${cycle.number}`
}

export function cycleStatus(cycle: Cycle, now = Date.now()): 'past' | 'current' | 'upcoming' {
  if (cycle.completed_at || new Date(cycle.ends_at).getTime() < now) return 'past'
  if (new Date(cycle.starts_at).getTime() <= now) return 'current'
  return 'upcoming'
}

export function cycleProgressRatio(cycle: Cycle, now = Date.now()): number {
  const start = new Date(cycle.starts_at).getTime()
  const end = new Date(cycle.ends_at).getTime()
  if (end <= start) return 0
  return Math.min(1, Math.max(0, (now - start) / (end - start)))
}

export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean)
  if (parts.length === 0) return '?'
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase()
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase()
}

export function pluralize(count: number, singular: string, plural = `${singular}s`): string {
  return `${count} ${count === 1 ? singular : plural}`
}

export function isMac(): boolean {
  return typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent)
}

export function modKey(): string {
  return isMac() ? '⌘' : 'Ctrl'
}
