import { AtSign, Bell, Bot, CheckCircle2, CircleAlert, CircleDot, MessageSquare, TrendingUp, UserPlus, XCircle } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Issue, Notification, Project, User } from '../types'

export type InboxFilter = 'all' | 'unread' | 'mentions' | 'snoozed'

export const INBOX_FILTERS: { id: InboxFilter; label: string }[] = [
  { id: 'all', label: 'All' },
  { id: 'unread', label: 'Unread' },
  { id: 'mentions', label: 'Mentions' },
  { id: 'snoozed', label: 'Snoozed' },
]

export function isSnoozed(n: Notification, now = Date.now()): boolean {
  return n.snoozed_until !== null && new Date(n.snoozed_until).getTime() > now
}

export function filterNotifications(items: Notification[], filter: InboxFilter, now = Date.now()): Notification[] {
  return items
    .filter((n) => !n.archived_at)
    .filter((n) => {
      switch (filter) {
        case 'snoozed':
          return isSnoozed(n, now)
        case 'unread':
          return !isSnoozed(n, now) && !n.read_at
        case 'mentions':
          return !isSnoozed(n, now) && (n.kind === 'mentioned' || n.kind === 'comment')
        case 'all':
          return !isSnoozed(n, now)
      }
    })
    .sort((a, b) => b.created_at.localeCompare(a.created_at))
}

export function filterCount(items: Notification[], filter: InboxFilter): number {
  const list = filterNotifications(items, filter)
  return filter === 'snoozed' ? list.length : list.filter((n) => !n.read_at).length
}

export function kindIcon(kind: string): ReactNode {
  const size = 10
  switch (kind) {
    case 'assigned':
      return <UserPlus size={size} />
    case 'mentioned':
      return <AtSign size={size} />
    case 'comment':
      return <MessageSquare size={size} />
    case 'status_change':
      return <CircleDot size={size} />
    case 'lane_blocked':
      return <CircleAlert size={size} />
    case 'lane_done_claimed':
    case 'lane_verified':
      return <CheckCircle2 size={size} />
    case 'lane_failed':
      return <XCircle size={size} />
    case 'project_update':
      return <TrendingUp size={size} />
    case 'subscribed_update':
      return <Bell size={size} />
    default:
      return <Bot size={size} />
  }
}

export function describe(n: Notification, actor: User | null): string {
  if (n.body.trim()) return n.body.trim().split('\n')[0]
  const who = actor ? actor.display_name || actor.name : 'Someone'
  switch (n.kind) {
    case 'assigned':
      return `${who} assigned the issue to you`
    case 'mentioned':
      return `${who} mentioned you`
    case 'comment':
      return `${who} commented`
    case 'status_change':
      return `${who} changed the status`
    case 'lane_blocked':
      return 'Agent is blocked and needs input'
    case 'lane_done_claimed':
      return 'Agent claims the work is done'
    case 'lane_verified':
      return 'Agent work was verified'
    case 'lane_failed':
      return 'Agent lane failed'
    case 'project_update':
      return `${who} posted a project update`
    default:
      return `${who} updated the issue`
  }
}

export function notificationTitle(n: Notification, issue: Issue | undefined, project: Project | undefined): { id: string | null; title: string } {
  if (issue) return { id: issue.identifier, title: issue.title }
  if (project) return { id: null, title: project.name }
  return { id: null, title: 'Notification' }
}

export function snoozeOptions(now = new Date()): { id: string; label: string; until: string }[] {
  const at = (days: number, hour = 9) => {
    const d = new Date(now)
    d.setDate(d.getDate() + days)
    d.setHours(hour, 0, 0, 0)
    return d.toISOString()
  }
  const daysToMonday = ((8 - now.getDay()) % 7) || 7
  return [
    { id: '1h', label: 'In one hour', until: new Date(now.getTime() + 3_600_000).toISOString() },
    { id: 'tomorrow', label: 'Tomorrow', until: at(1) },
    { id: 'next-week', label: 'Next week', until: at(daysToMonday) },
    { id: 'month', label: 'In one month', until: at(30) },
  ]
}
