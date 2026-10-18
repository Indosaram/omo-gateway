import { Archive, Calendar, CircleUser, Copy, Hexagon, Link, Signal, Tag, Trash2, User } from 'lucide-react'
import type { TrackerState } from '../store'
import type { Issue } from '../types'
import type { MenuItem } from '../ui'

export function issueContextItems(issue: Issue, t: TrackerState): MenuItem[] {
  const isMine = issue.assignee_id !== null && issue.assignee_id === t.me?.id
  return [
    { id: 'status', label: 'Status', icon: <Hexagon size={14} />, kbd: 'S', group: 'Properties' },
    { id: 'priority', label: 'Priority', icon: <Signal size={14} />, kbd: 'P', group: 'Properties' },
    { id: 'assignee', label: 'Assignee', icon: <CircleUser size={14} />, kbd: 'A', group: 'Properties' },
    { id: 'assign_me', label: isMine ? 'Unassign from me' : 'Assign to me', icon: <User size={14} />, kbd: 'I', group: 'Properties' },
    { id: 'labels', label: 'Labels', icon: <Tag size={14} />, kbd: 'L', group: 'Properties' },
    { id: 'due', label: 'Due date', icon: <Calendar size={14} />, kbd: '⇧D', group: 'Properties' },
    { id: 'copy_id', label: 'Copy ID', icon: <Copy size={14} />, kbd: '⌘.', group: 'Copy' },
    { id: 'copy_link', label: 'Copy link', icon: <Link size={14} />, group: 'Copy' },
    { id: 'archive', label: issue.archived_at ? 'Unarchive' : 'Archive', icon: <Archive size={14} />, kbd: '⌘⇧A', group: 'Danger' },
    { id: 'delete', label: 'Delete', icon: <Trash2 size={14} />, kbd: '⌘⌫', group: 'Danger', disabled: issue.lane_key !== null },
  ]
}
