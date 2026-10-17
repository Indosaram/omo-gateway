import type { ReactNode } from 'react'
import { cycleLabel, cycleStatus, dueShortcuts, shortDate, stateProgress } from '../format'
import type { TrackerState } from '../store'
import type { ID, Priority } from '../types'
import { ESTIMATE_POINTS, PRIORITY_LABELS } from '../types'
import { Avatar, CycleIcon, EstimateIcon, LabelDot, PriorityIcon, ProjectIcon, StatusIcon } from '../ui'
import type { MenuItem } from '../ui'

export const NONE = '__none__'

export function statusItems(t: TrackerState, teamId: ID): MenuItem[] {
  const states = t.teamStates(teamId)
  return states.map((s) => ({
    id: s.id,
    label: s.name,
    icon: <StatusIcon type={s.type} color={s.color} progress={stateProgress(s, states)} size={14} />,
  }))
}

export const PRIORITY_ORDER: Priority[] = [0, 1, 2, 3, 4]

export function priorityItems(): MenuItem[] {
  return PRIORITY_ORDER.map((p) => ({
    id: String(p),
    label: PRIORITY_LABELS[p],
    icon: <PriorityIcon priority={p} size={14} />,
  }))
}

export function assigneeItems(t: TrackerState): MenuItem[] {
  const humans = t.users.filter((u) => u.kind === 'human')
  const agents = t.users.filter((u) => u.kind === 'agent')
  const meFirst = [...humans].sort((a, b) => Number(b.is_me) - Number(a.is_me) || a.name.localeCompare(b.name))
  return [
    { id: NONE, label: 'No assignee', icon: <Avatar user={null} size={16} /> },
    ...meFirst.map((u) => ({
      id: u.id,
      label: u.is_me ? `${u.name} (you)` : u.name,
      icon: <Avatar user={u} size={16} />,
      group: 'Members',
    })),
    ...agents.map((u) => ({
      id: u.id,
      label: u.name,
      hint: u.model ?? undefined,
      icon: <Avatar user={u} size={16} />,
      group: 'Agents',
    })),
  ]
}

export function teamLabels(t: TrackerState, teamId: ID) {
  return t.labels.filter((l) => l.team_id === null || l.team_id === teamId)
}

export function labelItems(t: TrackerState, teamId: ID): MenuItem[] {
  return teamLabels(t, teamId).map((l) => ({
    id: l.id,
    label: l.name,
    icon: <LabelDot color={l.color} />,
    color: l.color,
  }))
}

export function projectItems(t: TrackerState, teamId: ID): MenuItem[] {
  const projects = t.projects.filter(
    (p) => !p.archived_at && (p.team_ids.length === 0 || p.team_ids.includes(teamId)),
  )
  return [
    { id: NONE, label: 'No project', icon: <ProjectIcon icon="box" color="var(--text-4)" /> },
    ...projects.map((p) => ({ id: p.id, label: p.name, icon: <ProjectIcon icon={p.icon} color={p.color} /> })),
  ]
}

export function milestoneItems(t: TrackerState, projectId: ID | null): MenuItem[] {
  const list = projectId ? t.milestones.filter((m) => m.project_id === projectId) : []
  return [
    { id: NONE, label: 'No milestone' },
    ...list
      .slice()
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((m) => ({ id: m.id, label: m.name, hint: m.target_date ? shortDate(m.target_date) : undefined })),
  ]
}

export function cycleItems(t: TrackerState, teamId: ID): MenuItem[] {
  const cycles = t.cycles
    .filter((c) => c.team_id === teamId && cycleStatus(c) !== 'past')
    .sort((a, b) => a.number - b.number)
  return [
    { id: NONE, label: 'No cycle', icon: <CycleIcon progress={0} /> },
    ...cycles.map((c) => ({
      id: c.id,
      label: cycleLabel(c),
      hint: cycleStatus(c) === 'current' ? 'Current' : `${shortDate(c.starts_at)} – ${shortDate(c.ends_at)}`,
      icon: <CycleIcon progress={cycleStatus(c) === 'current' ? 0.5 : 0} />,
    })),
  ]
}

export function estimateItems(): MenuItem[] {
  return [
    { id: NONE, label: 'No estimate', icon: <EstimateIcon /> },
    ...ESTIMATE_POINTS.map((n) => ({
      id: String(n),
      label: `${n} ${n === 1 ? 'point' : 'points'}`,
      icon: <EstimateIcon />,
    })),
  ]
}

export function dueItems(current: string | null): MenuItem[] {
  const items: MenuItem[] = dueShortcuts().map((s) => ({ id: s.date, label: s.label, hint: shortDate(s.date) }))
  if (current) items.push({ id: NONE, label: 'Remove due date' })
  return items
}

export function parseNullable(id: string): string | null {
  return id === NONE ? null : id
}

export function iconFor(items: MenuItem[], id: string | null): ReactNode {
  return items.find((i) => i.id === (id ?? NONE))?.icon ?? null
}
