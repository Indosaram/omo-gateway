import { Bot, Box, Calendar, CircleUser, Hexagon, ListFilter, Signal, Tag, Type, User, Users, X } from 'lucide-react'
import { useMemo, useState } from 'react'
import { cycleLabel, cycleStatus, stateProgress } from '../format'
import type { TrackerState } from '../store'
import type { ID, LaneStatus, Priority, StateType, ViewFilters } from '../types'
import { LANE_STATUS_LABELS, PRIORITY_LABELS, STATE_TYPE_ORDER } from '../types'
import { Avatar, CycleIcon, LabelDot, MenuList, Popover, PriorityIcon, ProjectIcon, StatusIcon, type Anchor, type MenuItem } from '../ui'

export type FilterField =
  | 'status'
  | 'status_type'
  | 'priority'
  | 'assignee'
  | 'creator'
  | 'labels'
  | 'project'
  | 'cycle'
  | 'team'
  | 'lane'
  | 'due'

interface FieldDef {
  id: FilterField
  label: string
  icon: React.ReactNode
  plural: string
  single?: boolean
}

const FIELD_DEFS: FieldDef[] = [
  { id: 'status', label: 'Status', plural: 'statuses', icon: <Hexagon size={14} /> },
  { id: 'status_type', label: 'Status type', plural: 'status types', icon: <Hexagon size={14} /> },
  { id: 'priority', label: 'Priority', plural: 'priorities', icon: <Signal size={14} /> },
  { id: 'assignee', label: 'Assignee', plural: 'assignees', icon: <CircleUser size={14} /> },
  { id: 'creator', label: 'Creator', plural: 'creators', icon: <User size={14} /> },
  { id: 'labels', label: 'Labels', plural: 'labels', icon: <Tag size={14} /> },
  { id: 'project', label: 'Project', plural: 'projects', icon: <Box size={14} /> },
  { id: 'cycle', label: 'Cycle', plural: 'cycles', icon: <CycleIcon progress={0.5} /> },
  { id: 'team', label: 'Team', plural: 'teams', icon: <Users size={14} /> },
  { id: 'lane', label: 'Agent lane', plural: 'lane statuses', icon: <Bot size={14} /> },
  { id: 'due', label: 'Due date', plural: 'due dates', icon: <Calendar size={14} />, single: true },
]

const STATE_TYPE_LABELS: Record<StateType, string> = {
  triage: 'Triage',
  backlog: 'Backlog',
  unstarted: 'Unstarted',
  started: 'Started',
  completed: 'Completed',
  canceled: 'Canceled',
}

const DUE_OPTIONS: { id: NonNullable<ViewFilters['due']>; label: string }[] = [
  { id: 'overdue', label: 'Overdue' },
  { id: 'today', label: 'Due today' },
  { id: 'week', label: 'Due within a week' },
  { id: 'none', label: 'No due date' },
]

const LANE_ANY = '__any__'
const LANE_NONE = '__none__'

export function fieldDef(field: FilterField): FieldDef {
  return FIELD_DEFS.find((f) => f.id === field)!
}

export function selectedValues(filters: ViewFilters, field: FilterField): string[] {
  switch (field) {
    case 'status':
      return filters.state_ids ?? []
    case 'status_type':
      return filters.state_types ?? []
    case 'priority':
      return (filters.priorities ?? []).map(String)
    case 'assignee':
      return filters.assignee_ids ?? []
    case 'creator':
      return filters.creator_ids ?? []
    case 'labels':
      return filters.label_ids ?? []
    case 'project':
      return filters.project_ids ?? []
    case 'cycle':
      return filters.cycle_ids ?? []
    case 'team':
      return filters.team_ids ?? []
    case 'lane': {
      const values: string[] = [...(filters.lane_statuses ?? [])]
      if (filters.has_lane === true) values.push(LANE_ANY)
      if (filters.has_lane === false) values.push(LANE_NONE)
      return values
    }
    case 'due':
      return filters.due ? [filters.due] : []
  }
}

function setValues(filters: ViewFilters, field: FilterField, values: string[]): ViewFilters {
  const next: ViewFilters = { ...filters }
  const assign = <K extends keyof ViewFilters>(key: K, value: ViewFilters[K] | undefined) => {
    if (value === undefined || (Array.isArray(value) && value.length === 0)) delete next[key]
    else next[key] = value
  }
  switch (field) {
    case 'status':
      assign('state_ids', values)
      break
    case 'status_type':
      assign('state_types', values as StateType[])
      break
    case 'priority':
      assign('priorities', values.map((v) => Number(v) as Priority))
      break
    case 'assignee':
      assign('assignee_ids', values)
      break
    case 'creator':
      assign('creator_ids', values)
      break
    case 'labels':
      assign('label_ids', values)
      break
    case 'project':
      assign('project_ids', values)
      break
    case 'cycle':
      assign('cycle_ids', values)
      break
    case 'team':
      assign('team_ids', values)
      break
    case 'lane': {
      assign('lane_statuses', values.filter((v) => v !== LANE_ANY && v !== LANE_NONE) as LaneStatus[])
      assign('has_lane', values.includes(LANE_ANY) ? true : values.includes(LANE_NONE) ? false : undefined)
      break
    }
    case 'due':
      assign('due', (values[values.length - 1] as ViewFilters['due']) ?? undefined)
      break
  }
  return next
}

export function clearField(filters: ViewFilters, field: FilterField): ViewFilters {
  return setValues(filters, field, [])
}

export function toggleValue(filters: ViewFilters, field: FilterField, value: string): ViewFilters {
  const current = selectedValues(filters, field)
  if (fieldDef(field).single) return setValues(filters, field, current.includes(value) ? [] : [value])
  if (field === 'lane' && (value === LANE_ANY || value === LANE_NONE)) {
    const rest = current.filter((v) => v !== LANE_ANY && v !== LANE_NONE)
    return setValues(filters, field, current.includes(value) ? rest : [...rest, value])
  }
  return setValues(filters, field, current.includes(value) ? current.filter((v) => v !== value) : [...current, value])
}

export function activeFields(filters: ViewFilters): FilterField[] {
  return FIELD_DEFS.map((f) => f.id).filter((field) => selectedValues(filters, field).length > 0)
}

export function valueItems(field: FilterField, t: TrackerState, teamId: ID | null): MenuItem[] {
  switch (field) {
    case 'status': {
      const states = teamId ? t.teamStates(teamId) : t.states
      return states.map((state) => {
        const team = t.teamsById.get(state.team_id)
        return {
          id: state.id,
          label: state.name,
          hint: teamId ? undefined : team?.key,
          keywords: team?.name,
          icon: <StatusIcon type={state.type} color={state.color} progress={stateProgress(state, t.teamStates(state.team_id))} />,
        }
      })
    }
    case 'status_type':
      return STATE_TYPE_ORDER.map((type) => ({
        id: type,
        label: STATE_TYPE_LABELS[type],
        icon: <StatusIcon type={type} color="var(--text-3)" progress={0.5} />,
      }))
    case 'priority':
      return ([0, 1, 2, 3, 4] as Priority[]).map((p) => ({ id: String(p), label: PRIORITY_LABELS[p], icon: <PriorityIcon priority={p} /> }))
    case 'assignee':
    case 'creator': {
      const base: MenuItem[] = field === 'assignee'
        ? [
            { id: 'none', label: 'No assignee', icon: <Avatar user={null} size={16} /> },
            { id: 'me', label: 'Current user', icon: <Avatar user={t.me} size={16} /> },
          ]
        : []
      return [
        ...base,
        ...t.users.map((user) => ({
          id: user.id,
          label: user.name,
          keywords: `${user.display_name} ${user.email ?? ''}`,
          icon: <Avatar user={user} size={16} />,
          hint: user.kind === 'agent' ? 'Agent' : undefined,
        })),
      ]
    }
    case 'labels':
      return t.labels
        .filter((label) => !teamId || label.team_id === null || label.team_id === teamId)
        .map((label) => ({ id: label.id, label: label.name, icon: <LabelDot color={label.color} /> }))
    case 'project':
      return [
        { id: 'none', label: 'No project', icon: <Box size={14} /> },
        ...t.projects
          .filter((p) => !p.archived_at)
          .map((p) => ({ id: p.id, label: p.name, icon: <ProjectIcon icon={p.icon} color={p.color} /> })),
      ]
    case 'cycle':
      return [
        { id: 'none', label: 'No cycle', icon: <CycleIcon progress={0} /> },
        { id: 'current', label: 'Current cycle', icon: <CycleIcon progress={0.5} /> },
        ...t.cycles
          .filter((c) => !teamId || c.team_id === teamId)
          .map((c) => ({
            id: c.id,
            label: cycleLabel(c),
            hint: cycleStatus(c) === 'current' ? 'Current' : t.teamsById.get(c.team_id)?.key,
            icon: <CycleIcon progress={cycleStatus(c) === 'past' ? 1 : cycleStatus(c) === 'current' ? 0.5 : 0} />,
          })),
      ]
    case 'team':
      return t.teams.map((team) => ({ id: team.id, label: team.name, hint: team.key, icon: <Users size={14} /> }))
    case 'lane':
      return [
        { id: LANE_ANY, label: 'Has agent lane', icon: <Bot size={14} /> },
        { id: LANE_NONE, label: 'No agent lane', icon: <Bot size={14} /> },
        ...(Object.keys(LANE_STATUS_LABELS) as LaneStatus[]).map((status) => ({
          id: status,
          label: LANE_STATUS_LABELS[status],
          icon: <span className={`lic-lane-dot lic-lane--${status}`} />,
        })),
      ]
    case 'due':
      return DUE_OPTIONS.map((o) => ({ id: o.id, label: o.label, icon: <Calendar size={14} /> }))
  }
}

export function FilterPopover({
  open,
  onOpenChange,
  anchor,
  filters,
  onChange,
  t,
  teamId,
  initialField = null,
  showTeam,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  anchor: Anchor
  filters: ViewFilters
  onChange: (filters: ViewFilters) => void
  t: TrackerState
  teamId: ID | null
  initialField?: FilterField | null
  showTeam: boolean
}) {
  return (
    <Popover open={open} onOpenChange={onOpenChange} anchor={anchor} width={260}>
      <FilterMenuBody
        key={initialField ?? 'root'}
        filters={filters}
        onChange={onChange}
        t={t}
        teamId={teamId}
        initialField={initialField}
        showTeam={showTeam}
        onDone={() => onOpenChange(false)}
      />
    </Popover>
  )
}

function FilterMenuBody({
  filters,
  onChange,
  t,
  teamId,
  initialField,
  showTeam,
  onDone,
}: {
  filters: ViewFilters
  onChange: (filters: ViewFilters) => void
  t: TrackerState
  teamId: ID | null
  initialField: FilterField | null
  showTeam: boolean
  onDone: () => void
}) {
  const [field, setField] = useState<FilterField | null>(initialField)
  const [query, setQuery] = useState('')
  const fieldItems = useMemo<MenuItem[]>(() => {
    const items: MenuItem[] = FIELD_DEFS.filter((f) => (f.id === 'team' ? showTeam || !teamId : true)).map((f) => ({
      id: f.id,
      label: f.label,
      icon: f.icon,
      hint: selectedValues(filters, f.id).length > 0 ? String(selectedValues(filters, f.id).length) : undefined,
    }))
    const q = query.trim()
    if (q) items.unshift({ id: `text:${q}`, label: `Title contains "${q}"`, icon: <Type size={14} />, keywords: q })
    return items
  }, [filters, query, showTeam, teamId])

  if (!field) {
    return (
      <MenuList
        items={fieldItems}
        searchable
        placeholder="Filter…"
        query={query}
        onQueryChange={setQuery}
        onSelect={(item) => {
          if (item.id.startsWith('text:')) {
            onChange({ ...filters, text: item.id.slice(5) })
            onDone()
            return
          }
          setQuery('')
          setField(item.id as FilterField)
        }}
      />
    )
  }
  const def = fieldDef(field)
  const items = valueItems(field, t, teamId)
  return (
    <MenuList
      key={field}
      items={items}
      searchable
      multi={!def.single}
      placeholder={`${def.label}…`}
      selectedIds={selectedValues(filters, field)}
      onBack={() => setField(null)}
      onSelect={(item) => {
        onChange(toggleValue(filters, field, item.id))
        if (def.single) onDone()
      }}
    />
  )
}

function valueSummary(field: FilterField, values: string[], t: TrackerState): React.ReactNode {
  const items = valueItems(field, t, null)
  const def = fieldDef(field)
  if (values.length === 1) {
    const item = items.find((i) => i.id === values[0])
    return (
      <>
        {item?.icon}
        <span>{item?.label ?? values[0]}</span>
      </>
    )
  }
  const icons = values
    .slice(0, 3)
    .map((v) => items.find((i) => i.id === v)?.icon)
    .filter(Boolean)
  return (
    <>
      <span className="lic-fchip-icons">
        {icons.map((icon, index) => (
          <span key={index} className="lic-fchip-stack">
            {icon}
          </span>
        ))}
      </span>
      <span>
        {values.length} {def.plural}
      </span>
    </>
  )
}

function operatorFor(field: FilterField, count: number): string {
  if (field === 'labels') return count > 1 ? 'include any of' : 'include'
  if (field === 'due') return 'is'
  return count > 1 ? 'is any of' : 'is'
}

export function FilterChips({
  filters,
  onChange,
  onEditField,
  onAddFilter,
  t,
}: {
  filters: ViewFilters
  onChange: (filters: ViewFilters) => void
  onEditField: (field: FilterField, anchor: HTMLElement) => void
  onAddFilter: (anchor: HTMLElement) => void
  t: TrackerState
}) {
  const fields = activeFields(filters)
  const text = filters.text?.trim()
  return (
    <div className="lic-fchips" data-filter-chips>
      {text && (
        <div className="lic-fchip">
          <span className="lic-fchip-seg">
            <Type size={13} />
            Title
          </span>
          <span className="lic-fchip-seg t-3">contains</span>
          <span className="lic-fchip-seg">{text}</span>
          <button
            type="button"
            className="lic-fchip-seg lic-fchip-x"
            aria-label="Remove filter"
            onClick={() => {
              const next = { ...filters }
              delete next.text
              onChange(next)
            }}
          >
            <X size={13} />
          </button>
        </div>
      )}
      {fields.map((field) => {
        const def = fieldDef(field)
        const values = selectedValues(filters, field)
        return (
          <div key={field} className="lic-fchip" data-filter-chip={field}>
            <span className="lic-fchip-seg">
              {def.icon}
              {def.label}
            </span>
            <span className="lic-fchip-seg t-3">{operatorFor(field, values.length)}</span>
            <button type="button" className="lic-fchip-seg lic-fchip-value" onClick={(e) => onEditField(field, e.currentTarget)}>
              {valueSummary(field, values, t)}
            </button>
            <button
              type="button"
              className="lic-fchip-seg lic-fchip-x"
              aria-label={`Remove ${def.label} filter`}
              onClick={() => onChange(clearField(filters, field))}
            >
              <X size={13} />
            </button>
          </div>
        )
      })}
      <button type="button" className="lic-fchip-add" aria-label="Add filter" title="Add filter" onClick={(e) => onAddFilter(e.currentTarget)}>
        <ListFilter size={14} />
      </button>
      <span className="lic-grow" />
      <span className="lic-fchips-match t-2">Match all filters</span>
    </div>
  )
}
