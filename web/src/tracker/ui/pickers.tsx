import React, { useMemo, useState } from 'react'
import { CalendarDays, ChevronLeft, ChevronRight, CircleDashed, Plus, UserRound, X } from 'lucide-react'
import { TrackerApiError, trackerApi } from '../api'
import { cycleLabel, cycleStatus, dueShortcuts, isClosedType, parseDateOnly, shortDate, stateProgress, toDateOnly } from '../format'
import { useTracker } from '../store'
import type { Issue, IssuePatch, Priority } from '../types'
import { ESTIMATE_POINTS, PRIORITY_LABELS } from '../types'
import { Avatar } from './Avatar'
import { IconButton } from './Button'
import { CycleIcon, EstimateIcon, LabelDot, PriorityIcon, ProjectIcon, StatusIcon } from './icons'
import { MenuList, Popover, type Anchor, type MenuItem } from './Menu'
import { toast } from './Toast'

export interface PickerProps {
  issueIds: string[]
  anchor: Anchor
  open: boolean
  onOpenChange: (open: boolean) => void
}

const LABEL_COLORS = ['#eb5757', '#f2994a', '#f2c94c', '#4cb782', '#26b5ce', '#4ea7fc', '#5e6ad2', '#bb87fc', '#f7c8c1', '#95a2b3']

function useIssues(ids: string[]): Issue[] {
  const { issuesById } = useTracker()
  return useMemo(() => ids.map((id) => issuesById.get(id)).filter((i): i is Issue => Boolean(i)), [ids, issuesById])
}

function common<T>(issues: Issue[], pick: (issue: Issue) => T): T | undefined {
  if (issues.length === 0) return undefined
  const first = pick(issues[0])
  return issues.every((issue) => pick(issue) === first) ? first : undefined
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function useApply(issueIds: string[]) {
  const { updateIssues } = useTracker()
  // updateIssues already reports failures through the store's lastError toast, so only the
  // lane_not_verified case adds its own toast (for the Force close action).
  return (patch: IssuePatch) => {
    updateIssues(issueIds, patch).catch((error: unknown) => {
      if (error instanceof TrackerApiError && error.code === 'lane_not_verified' && patch.state_id) {
        toast(error.message, {
          kind: 'error',
          action: {
            label: 'Force close',
            onClick: () => {
              updateIssues(issueIds, { ...patch, force_note: 'closed from tracker' }).catch(() => undefined)
            },
          },
        })
      }
    })
  }
}

interface PickerShellProps extends Omit<PickerProps, 'issueIds'> {
  items: MenuItem[]
  onSelect: (item: MenuItem) => void
  placeholder: string
  selectedIds?: string[]
  multi?: boolean
  width?: number
  emptyText?: string
  query?: string
  onQueryChange?: (q: string) => void
  footer?: React.ReactNode
}

function PickerShell({ anchor, open, onOpenChange, items, onSelect, placeholder, selectedIds, multi, width = 240, emptyText, query, onQueryChange, footer }: PickerShellProps) {
  return (
    <Popover open={open} onOpenChange={onOpenChange} anchor={anchor} width={width}>
      <MenuList
        searchable
        items={items}
        placeholder={placeholder}
        selectedIds={selectedIds}
        multi={multi}
        emptyText={emptyText}
        query={query}
        onQueryChange={onQueryChange}
        footer={footer}
        onSelect={(item) => {
          onSelect(item)
          if (!multi) onOpenChange(false)
        }}
      />
    </Popover>
  )
}

export function StatusPicker({ issueIds, anchor, open, onOpenChange }: PickerProps) {
  const t = useTracker()
  const issues = useIssues(issueIds)
  const apply = useApply(issueIds)
  const teamId = issues[0]?.team_id
  const items: MenuItem[] = useMemo(() => {
    if (!teamId) return []
    const states = t.teamStates(teamId)
    return states.map((state, index) => ({
      id: state.id,
      label: state.name,
      icon: <StatusIcon type={state.type} color={state.color} progress={stateProgress(state, states)} />,
      kbd: index < 9 ? String(index + 1) : undefined,
    }))
  }, [t, teamId])
  const current = common(issues, (i) => i.state_id)
  return (
    <PickerShell
      anchor={anchor}
      open={open}
      onOpenChange={onOpenChange}
      items={items}
      placeholder="Change status…"
      selectedIds={current ? [current] : []}
      onSelect={(item) => apply({ state_id: item.id })}
    />
  )
}

export const PRIORITY_ORDER: Priority[] = [0, 1, 2, 3, 4]

export function priorityMenuItems(): MenuItem[] {
  return PRIORITY_ORDER.map((p) => ({
    id: String(p),
    label: PRIORITY_LABELS[p],
    icon: <PriorityIcon priority={p} />,
    kbd: String(p),
  }))
}

export function PriorityPicker({ issueIds, anchor, open, onOpenChange }: PickerProps) {
  const issues = useIssues(issueIds)
  const apply = useApply(issueIds)
  const current = common(issues, (i) => i.priority)
  const items = useMemo(priorityMenuItems, [])
  return (
    <PickerShell
      anchor={anchor}
      open={open}
      onOpenChange={onOpenChange}
      items={items}
      placeholder="Set priority to…"
      width={220}
      selectedIds={current !== undefined ? [String(current)] : []}
      onSelect={(item) => apply({ priority: Number(item.id) as Priority })}
    />
  )
}

export function AssigneePicker({ issueIds, anchor, open, onOpenChange }: PickerProps) {
  const t = useTracker()
  const issues = useIssues(issueIds)
  const apply = useApply(issueIds)
  const current = common(issues, (i) => i.assignee_id)
  const items: MenuItem[] = useMemo(() => {
    const me = t.me
    const others = t.users.filter((u) => u.id !== me?.id).sort((a, b) => a.name.localeCompare(b.name))
    const humans = others.filter((u) => u.kind === 'human')
    const agents = others.filter((u) => u.kind === 'agent')
    const list: MenuItem[] = [
      { id: 'none', label: 'No assignee', icon: <UserRound size={14} color="var(--text-3)" />, kbd: '0', keywords: 'unassigned' },
    ]
    if (me) list.push({ id: me.id, label: me.name, icon: <Avatar user={me} size={16} />, hint: 'You', kbd: '1', keywords: `${me.display_name} me` })
    humans.forEach((u, i) =>
      list.push({
        id: u.id,
        label: u.name,
        icon: <Avatar user={u} size={16} />,
        kbd: i + 2 <= 9 ? String(i + 2) : undefined,
        keywords: `${u.display_name} ${u.email ?? ''}`,
      }),
    )
    agents.forEach((u) =>
      list.push({ id: u.id, label: u.name, icon: <Avatar user={u} size={16} />, group: 'Agents', hint: u.model ?? undefined, keywords: `${u.display_name} agent` }),
    )
    return list
  }, [t.me, t.users])
  return (
    <PickerShell
      anchor={anchor}
      open={open}
      onOpenChange={onOpenChange}
      items={items}
      placeholder="Assign to…"
      selectedIds={current === undefined ? [] : [current ?? 'none']}
      onSelect={(item) => apply({ assignee_id: item.id === 'none' ? null : item.id })}
    />
  )
}

export function LabelPicker({ issueIds, anchor, open, onOpenChange }: PickerProps) {
  const t = useTracker()
  const issues = useIssues(issueIds)
  const apply = useApply(issueIds)
  const [query, setQuery] = useState('')
  const teamId = issues[0]?.team_id ?? null
  const available = useMemo(
    () => t.labels.filter((l) => l.team_id === null || l.team_id === teamId).sort((a, b) => a.name.localeCompare(b.name)),
    [t.labels, teamId],
  )
  const selectedIds = useMemo(
    () => available.filter((l) => issues.length > 0 && issues.every((i) => i.label_ids.includes(l.id))).map((l) => l.id),
    [available, issues],
  )
  const trimmed = query.trim()
  const exact = available.some((l) => l.name.toLowerCase() === trimmed.toLowerCase())
  const items: MenuItem[] = useMemo(() => {
    const byId = new Map(available.map((l) => [l.id, l]))
    const list: MenuItem[] = available.map((l) => ({
      id: l.id,
      label: l.name,
      icon: <LabelDot color={l.color} />,
      group: l.parent_id ? byId.get(l.parent_id)?.name : undefined,
      keywords: l.description ?? undefined,
    }))
    list.sort((a, b) => (a.group ?? '').localeCompare(b.group ?? ''))
    if (trimmed && !exact) {
      list.push({ id: '__create__', label: `Create new label: "${trimmed}"`, icon: <Plus size={14} />, keywords: trimmed })
    }
    return list
  }, [available, trimmed, exact])

  const createLabel = async () => {
    try {
      const color = LABEL_COLORS[Math.floor(Math.random() * LABEL_COLORS.length)]
      const label = await trackerApi.createLabel({ name: trimmed, color, team_id: teamId })
      t.upsertLabel(label)
      setQuery('')
      apply({ add_label_ids: [label.id] })
    } catch (error) {
      toast.error(errorMessage(error))
    }
  }

  return (
    <PickerShell
      anchor={anchor}
      open={open}
      onOpenChange={(next) => {
        if (!next) setQuery('')
        onOpenChange(next)
      }}
      items={items}
      multi
      query={query}
      onQueryChange={setQuery}
      placeholder="Change or add labels…"
      selectedIds={selectedIds}
      emptyText="No labels"
      onSelect={(item) => {
        if (item.id === '__create__') {
          void createLabel()
          return
        }
        if (selectedIds.includes(item.id)) apply({ remove_label_ids: [item.id] })
        else apply({ add_label_ids: [item.id] })
      }}
    />
  )
}

export function ProjectPicker({ issueIds, anchor, open, onOpenChange }: PickerProps) {
  const t = useTracker()
  const issues = useIssues(issueIds)
  const apply = useApply(issueIds)
  const current = common(issues, (i) => i.project_id)
  const teamId = issues[0]?.team_id
  const items: MenuItem[] = useMemo(() => {
    const active = t.projects.filter((p) => !p.archived_at).sort((a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name))
    const list: MenuItem[] = [{ id: 'none', label: 'No project', icon: <CircleDashed size={14} color="var(--text-3)" />, kbd: '0' }]
    for (const p of active) {
      const ofTeam = !teamId || p.team_ids.length === 0 || p.team_ids.includes(teamId)
      list.push({
        id: p.id,
        label: p.name,
        icon: <ProjectIcon icon={p.icon} color={p.color} />,
        group: ofTeam ? undefined : 'Other teams',
      })
    }
    return list.sort((a, b) => Number(Boolean(a.group)) - Number(Boolean(b.group)))
  }, [t.projects, teamId])
  return (
    <PickerShell
      anchor={anchor}
      open={open}
      onOpenChange={onOpenChange}
      items={items}
      placeholder="Add to project…"
      width={260}
      selectedIds={current === undefined ? [] : [current ?? 'none']}
      onSelect={(item) => apply({ project_id: item.id === 'none' ? null : item.id })}
    />
  )
}

export function CyclePicker({ issueIds, anchor, open, onOpenChange }: PickerProps) {
  const t = useTracker()
  const issues = useIssues(issueIds)
  const apply = useApply(issueIds)
  const current = common(issues, (i) => i.cycle_id)
  const teamId = issues[0]?.team_id
  const items: MenuItem[] = useMemo(() => {
    const now = Date.now()
    const cycles = t.cycles
      .filter((c) => c.team_id === teamId && (cycleStatus(c, now) !== 'past' || c.id === current))
      .sort((a, b) => a.number - b.number)
    const list: MenuItem[] = [{ id: 'none', label: 'No cycle', icon: <CircleDashed size={14} color="var(--text-3)" />, kbd: '0' }]
    for (const c of cycles) {
      const status = cycleStatus(c, now)
      const span = `${shortDate(c.starts_at)} – ${shortDate(c.ends_at)}`
      list.push({
        id: c.id,
        label: cycleLabel(c),
        icon: <CycleIcon progress={status === 'current' ? 0.5 : status === 'past' ? 1 : 0} />,
        hint: status === 'current' ? `Current · ${span}` : status === 'upcoming' ? span : 'Completed',
      })
    }
    return list
  }, [t.cycles, teamId, current])
  return (
    <PickerShell
      anchor={anchor}
      open={open}
      onOpenChange={onOpenChange}
      items={items}
      placeholder="Move to cycle…"
      width={300}
      emptyText="No cycles"
      selectedIds={current === undefined ? [] : [current ?? 'none']}
      onSelect={(item) => apply({ cycle_id: item.id === 'none' ? null : item.id })}
    />
  )
}

export function estimateLabel(value: number): string {
  return `${value} ${value === 1 ? 'Point' : 'Points'}`
}

export function EstimatePicker({ issueIds, anchor, open, onOpenChange }: PickerProps) {
  const issues = useIssues(issueIds)
  const apply = useApply(issueIds)
  const current = common(issues, (i) => i.estimate)
  const items: MenuItem[] = useMemo(
    () => [
      { id: 'none', label: 'No estimate', icon: <EstimateIcon />, kbd: '0' },
      ...ESTIMATE_POINTS.map((p, i) => ({ id: String(p), label: estimateLabel(p), icon: <EstimateIcon />, kbd: String(i + 1) })),
    ],
    [],
  )
  return (
    <PickerShell
      anchor={anchor}
      open={open}
      onOpenChange={onOpenChange}
      items={items}
      placeholder="Set estimate to…"
      width={220}
      selectedIds={current === undefined ? [] : [current === null ? 'none' : String(current)]}
      onSelect={(item) => apply({ estimate: item.id === 'none' ? null : Number(item.id) })}
    />
  )
}

const WEEKDAYS = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su']

function Calendar({ value, onPick }: { value: string | null; onPick: (date: string) => void }) {
  const initial = value ? parseDateOnly(value) : new Date()
  const [month, setMonth] = useState(() => new Date(initial.getFullYear(), initial.getMonth(), 1))
  const today = toDateOnly(new Date())
  const days = useMemo(() => {
    const offset = (month.getDay() + 6) % 7
    const start = new Date(month.getFullYear(), month.getMonth(), 1 - offset)
    return Array.from({ length: 42 }, (_, i) => new Date(start.getFullYear(), start.getMonth(), start.getDate() + i))
  }, [month])
  const shift = (delta: number) => setMonth((m) => new Date(m.getFullYear(), m.getMonth() + delta, 1))
  return (
    <div className="lui-cal">
      <div className="lui-cal-head">
        <IconButton label="Previous month" size="sm" icon={<ChevronLeft size={14} />} onClick={() => shift(-1)} />
        <span className="lui-cal-title">{month.toLocaleDateString('en-US', { month: 'long', year: 'numeric' })}</span>
        <IconButton label="Next month" size="sm" icon={<ChevronRight size={14} />} onClick={() => shift(1)} />
      </div>
      <div className="lui-cal-grid">
        {WEEKDAYS.map((d) => (
          <span key={d} className="lui-cal-dow">
            {d}
          </span>
        ))}
        {days.map((day) => {
          const iso = toDateOnly(day)
          const classes = [
            'lui-cal-day',
            day.getMonth() !== month.getMonth() ? 'lui-cal-day--outside' : '',
            iso === today ? 'lui-cal-day--today' : '',
            iso === value ? 'lui-cal-day--selected' : '',
          ]
            .filter(Boolean)
            .join(' ')
          return (
            <button key={iso} type="button" className={classes} onClick={() => onPick(iso)} aria-label={iso} aria-pressed={iso === value}>
              {day.getDate()}
            </button>
          )
        })}
      </div>
    </div>
  )
}

function parseTypedDate(text: string): string | null {
  const trimmed = text.trim()
  if (!trimmed) return null
  if (/^\d{4}-\d{2}-\d{2}$/.test(trimmed)) return trimmed
  const parsed = new Date(trimmed)
  if (Number.isNaN(parsed.getTime())) return null
  if (!/\d{4}/.test(trimmed)) parsed.setFullYear(new Date().getFullYear())
  return toDateOnly(parsed)
}

export function DueDatePicker({ issueIds, anchor, open, onOpenChange }: PickerProps) {
  const issues = useIssues(issueIds)
  const apply = useApply(issueIds)
  const current = common(issues, (i) => i.due_date) ?? null
  const [typed, setTyped] = useState('')
  const pick = (date: string | null) => {
    apply({ due_date: date })
    setTyped('')
    onOpenChange(false)
  }
  const items: MenuItem[] = useMemo(() => {
    const list: MenuItem[] = dueShortcuts().map((s) => ({
      id: `due:${s.id}:${s.date}`,
      label: s.label,
      icon: <CalendarDays size={14} color="var(--text-3)" />,
      hint: parseDateOnly(s.date).toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' }),
    }))
    if (current) list.push({ id: '__remove__', label: 'Remove due date', icon: <X size={14} color="var(--text-3)" /> })
    return list
  }, [current])
  const typedDate = parseTypedDate(typed)
  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        if (!next) setTyped('')
        onOpenChange(next)
      }}
      anchor={anchor}
      width={268}
    >
      <MenuList
        searchable
        placeholder="Try: Tomorrow, Dec 24, 2026-11-01…"
        query={typed}
        onQueryChange={setTyped}
        numberShortcuts={false}
        items={
          typedDate && !items.some((i) => i.label.toLowerCase().startsWith(typed.trim().toLowerCase()))
            ? [{ id: typedDate, label: shortDate(typedDate), icon: <CalendarDays size={14} />, keywords: typed }]
            : items
        }
        onSelect={(item) => pick(item.id === '__remove__' ? null : item.id.startsWith('due:') ? item.id.slice(item.id.lastIndexOf(':') + 1) : item.id)}
        footer={<Calendar value={current} onPick={(date) => pick(date)} />}
      />
    </Popover>
  )
}

export function ParentPicker({ issueIds, anchor, open, onOpenChange }: PickerProps) {
  const t = useTracker()
  const issues = useIssues(issueIds)
  const apply = useApply(issueIds)
  const [query, setQuery] = useState('')
  const current = common(issues, (i) => i.parent_id)
  const teamId = issues[0]?.team_id
  const excluded = useMemo(() => {
    const blocked = new Set(issueIds)
    let grew = true
    while (grew) {
      grew = false
      for (const issue of t.issues) {
        if (issue.parent_id && blocked.has(issue.parent_id) && !blocked.has(issue.id)) {
          blocked.add(issue.id)
          grew = true
        }
      }
    }
    return blocked
  }, [t.issues, issueIds])
  const items: MenuItem[] = useMemo(() => {
    const q = query.trim().toLowerCase()
    const candidates = t.issues
      .filter((i) => !i.archived_at && !excluded.has(i.id) && (!teamId || i.team_id === teamId))
      .filter((i) => !q || `${i.identifier} ${i.title}`.toLowerCase().includes(q))
      .sort((a, b) => b.updated_at.localeCompare(a.updated_at))
      .slice(0, 50)
    const list: MenuItem[] = []
    if (current) list.push({ id: '__remove__', label: 'Remove parent', icon: <X size={14} color="var(--text-3)" />, keywords: q })
    for (const issue of candidates) {
      const state = t.statesById.get(issue.state_id)
      list.push({
        id: issue.id,
        label: `${issue.identifier} ${issue.title}`,
        keywords: q,
        icon: state ? <StatusIcon type={state.type} color={state.color} progress={isClosedType(state.type) ? 1 : undefined} /> : undefined,
      })
    }
    return list
  }, [t.issues, t.statesById, excluded, teamId, query, current])
  return (
    <PickerShell
      anchor={anchor}
      open={open}
      onOpenChange={(next) => {
        if (!next) setQuery('')
        onOpenChange(next)
      }}
      items={items}
      query={query}
      onQueryChange={setQuery}
      placeholder="Set parent issue…"
      width={360}
      emptyText="No issues found"
      selectedIds={current ? [current] : []}
      onSelect={(item) => apply({ parent_id: item.id === '__remove__' ? null : item.id })}
    />
  )
}
