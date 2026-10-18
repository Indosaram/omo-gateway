import { Box, CircleDashed, Hexagon, Tag, Users } from 'lucide-react'
import { compactRelative, cycleLabel, dueState, isClosedType, shortDate, stateProgress } from '../format'
import type { IssueGroup, TrackerState } from '../store'
import type { DisplayProperty, Issue, Priority } from '../types'
import { LANE_STATUS_LABELS } from '../types'
import { Avatar, CycleIcon, LabelDot, PriorityIcon, ProjectIcon, StatusIcon, Tooltip } from '../ui'
import { useCollection, type PickerKind } from './model'

export function IssueStatusIcon({ issue, t, size = 14 }: { issue: Issue; t: TrackerState; size?: number }) {
  const state = t.statesById.get(issue.state_id)
  if (!state) return <CircleDashed size={size} className="t-4" />
  return <StatusIcon type={state.type} color={state.color} progress={stateProgress(state, t.teamStates(state.team_id))} size={size} />
}

export function GroupIcon({ group, t }: { group: IssueGroup; t: TrackerState }) {
  switch (group.grouping) {
    case 'status': {
      const state = typeof group.value === 'string' ? t.statesById.get(group.value) : undefined
      return state ? (
        <StatusIcon type={state.type} color={state.color} progress={stateProgress(state, t.teamStates(state.team_id))} size={14} />
      ) : (
        <CircleDashed size={14} className="t-4" />
      )
    }
    case 'priority':
      return <PriorityIcon priority={(group.value as Priority) ?? 0} size={14} />
    case 'assignee':
      return <Avatar user={typeof group.value === 'string' ? t.usersById.get(group.value) ?? null : null} size={16} />
    case 'project': {
      const project = typeof group.value === 'string' ? t.projectsById.get(group.value) : undefined
      return project ? <ProjectIcon icon={project.icon} color={project.color} /> : <Box size={14} className="t-4" />
    }
    case 'cycle':
      return <CycleIcon progress={0.5} />
    case 'label': {
      const label = typeof group.value === 'string' ? t.labelsById.get(group.value) : undefined
      return label ? <LabelDot color={label.color} /> : <Tag size={14} className="t-4" />
    }
    case 'team':
      return <Users size={14} className="t-3" />
    default:
      return null
  }
}

function Chip({
  children,
  onClick,
  title,
  className,
}: {
  children: React.ReactNode
  onClick?: (event: React.MouseEvent<HTMLButtonElement>) => void
  title?: string
  className?: string
}) {
  return (
    <button
      type="button"
      className={`lic-chip${className ? ` ${className}` : ''}`}
      title={title}
      onClick={(event) => {
        if (!onClick) return
        event.stopPropagation()
        onClick(event)
      }}
    >
      {children}
    </button>
  )
}

export function IssueChips({
  issue,
  t,
  properties,
  compact = false,
}: {
  issue: Issue
  t: TrackerState
  properties: DisplayProperty[]
  compact?: boolean
}) {
  const c = useCollection()
  const has = (p: DisplayProperty) => properties.includes(p)
  const open = (kind: PickerKind) => (event: React.MouseEvent<HTMLButtonElement>) => c.openPicker(kind, issue, event.currentTarget)
  const state = t.statesById.get(issue.state_id)
  const closed = state ? isClosedType(state.type) : false
  const project = issue.project_id ? t.projectsById.get(issue.project_id) : undefined
  const cycle = issue.cycle_id ? t.cyclesById.get(issue.cycle_id) : undefined
  const due = dueState(issue.due_date, closed)
  const labels = issue.label_ids.map((id) => t.labelsById.get(id)).filter((l) => l !== undefined)
  return (
    <>
      {has('sub_issues') && issue.sub_issue_count > 0 && (
        <span className="lic-chip lic-chip--static t-3 tabular" title="Sub-issues">
          <Hexagon size={11} />
          {issue.sub_issue_completed}/{issue.sub_issue_count}
        </span>
      )}
      {has('lane') && issue.lane && (
        <span className={`lic-chip lic-chip--static lic-lane lic-lane--${issue.lane.status}`} title={`Agent lane ${issue.lane.key}`}>
          <span className="lic-lane-dot" />
          {LANE_STATUS_LABELS[issue.lane.status]}
        </span>
      )}
      {has('labels') &&
        (labels.length > 2 && compact ? (
          <Chip onClick={open('labels')} title={labels.map((l) => l.name).join(', ')}>
            <span className="lic-label-dots">
              {labels.slice(0, 3).map((l) => (
                <LabelDot key={l.id} color={l.color} />
              ))}
            </span>
            {labels.length} labels
          </Chip>
        ) : (
          labels.map((label) => (
            <Chip key={label.id} onClick={open('labels')} className="lic-chip--label">
              <LabelDot color={label.color} />
              <span className="lic-chip-text">{label.name}</span>
            </Chip>
          ))
        ))}
      {has('project') && project && (
        <Chip onClick={open('project')} className="lic-chip--project">
          <ProjectIcon icon={project.icon} color={project.color} />
          <span className="lic-chip-text">{project.name}</span>
        </Chip>
      )}
      {has('cycle') && cycle && (
        <Chip onClick={open('cycle')}>
          <CycleIcon progress={0.5} />
          <span className="lic-chip-text">{cycleLabel(cycle)}</span>
        </Chip>
      )}
      {has('estimate') && issue.estimate !== null && (
        <Chip onClick={open('estimate')} title="Estimate">
          <span className="tabular">{issue.estimate}</span>
        </Chip>
      )}
      {has('due') && issue.due_date && (
        <Chip onClick={open('due')} className={due ? `lic-due--${due}` : undefined} title={`Due ${shortDate(issue.due_date)}`}>
          {shortDate(issue.due_date)}
        </Chip>
      )}
      {has('created') && !compact && (
        <Tooltip content={`Created ${shortDate(issue.created_at)}`}>
          <span className="lic-date t-3">{shortDate(issue.created_at)}</span>
        </Tooltip>
      )}
      {has('updated') && !compact && (
        <Tooltip content={`Updated ${shortDate(issue.updated_at)}`}>
          <span className="lic-date t-3">{compactRelative(issue.updated_at)}</span>
        </Tooltip>
      )}
    </>
  )
}

export function AssigneeButton({ issue, t }: { issue: Issue; t: TrackerState }) {
  const c = useCollection()
  const user = issue.assignee_id ? t.usersById.get(issue.assignee_id) ?? null : null
  return (
    <button
      type="button"
      className="lic-avatar-btn"
      title={user ? `Assigned to ${user.name}` : 'Unassigned'}
      onClick={(event) => {
        event.stopPropagation()
        c.openPicker('assignee', issue, event.currentTarget)
      }}
    >
      <Avatar user={user} size={20} />
    </button>
  )
}
