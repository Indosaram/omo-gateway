import React, { useRef, useState } from 'react'
import { CalendarDays, CircleDashed, Tag, UserRound } from 'lucide-react'
import { cycleLabel, cycleProgressRatio, dueState, isClosedType, shortDate, stateProgress } from '../format'
import { useTracker } from '../store'
import type { Issue } from '../types'
import { PRIORITY_LABELS } from '../types'
import { Avatar } from './Avatar'
import { CycleIcon, EstimateIcon, LabelDot, PriorityIcon, ProjectIcon, StatusIcon } from './icons'
import {
  AssigneePicker,
  CyclePicker,
  DueDatePicker,
  EstimatePicker,
  LabelPicker,
  ParentPicker,
  PriorityPicker,
  ProjectPicker,
  StatusPicker,
  estimateLabel,
  type PickerProps,
} from './pickers'
import { Tooltip } from './Tooltip'

export type PropertyKind = 'status' | 'priority' | 'assignee' | 'labels' | 'project' | 'cycle' | 'estimate' | 'due' | 'parent'

export interface PropertyChipProps {
  kind: PropertyKind
  issue: Issue
  variant?: 'compact' | 'full'
  issueIds?: string[]
  maxLabels?: number
  className?: string
}

const PICKERS: Record<PropertyKind, React.ComponentType<PickerProps>> = {
  status: StatusPicker,
  priority: PriorityPicker,
  assignee: AssigneePicker,
  labels: LabelPicker,
  project: ProjectPicker,
  cycle: CyclePicker,
  estimate: EstimatePicker,
  due: DueDatePicker,
  parent: ParentPicker,
}

const TOOLTIPS: Record<PropertyKind, { label: string; kbd: string[] }> = {
  status: { label: 'Change status', kbd: ['S'] },
  priority: { label: 'Change priority', kbd: ['P'] },
  assignee: { label: 'Assign to', kbd: ['A'] },
  labels: { label: 'Change labels', kbd: ['L'] },
  project: { label: 'Add to project', kbd: ['shift', 'P'] },
  cycle: { label: 'Move to cycle', kbd: ['shift', 'C'] },
  estimate: { label: 'Set estimate', kbd: ['shift', 'E'] },
  due: { label: 'Set due date', kbd: ['shift', 'D'] },
  parent: { label: 'Set parent issue', kbd: ['mod', 'shift', 'P'] },
}

export function PropertyChip({ kind, issue, variant = 'compact', issueIds, maxLabels = 3, className }: PropertyChipProps) {
  const t = useTracker()
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLButtonElement | null>(null)
  const full = variant === 'full'

  let content: React.ReactNode = null
  let iconOnly = false
  let empty = false
  let tone: string | undefined
  let tooltip: React.ReactNode = TOOLTIPS[kind].label

  switch (kind) {
    case 'status': {
      const state = t.statesById.get(issue.state_id)
      const states = t.teamStates(issue.team_id)
      const icon = state ? (
        <StatusIcon type={state.type} color={state.color} progress={stateProgress(state, states)} />
      ) : (
        <CircleDashed size={14} color="var(--text-3)" />
      )
      iconOnly = !full
      tooltip = state?.name ?? 'Status'
      content = full ? (
        <>
          {icon}
          <span className="lui-chip-text">{state?.name ?? 'Unknown'}</span>
        </>
      ) : (
        icon
      )
      break
    }
    case 'priority': {
      iconOnly = !full
      tooltip = PRIORITY_LABELS[issue.priority]
      empty = issue.priority === 0
      content = full ? (
        <>
          <PriorityIcon priority={issue.priority} />
          <span className="lui-chip-text">{empty ? 'Set priority' : PRIORITY_LABELS[issue.priority]}</span>
        </>
      ) : (
        <PriorityIcon priority={issue.priority} />
      )
      break
    }
    case 'assignee': {
      const user = issue.assignee_id ? t.usersById.get(issue.assignee_id) ?? null : null
      iconOnly = !full
      empty = !user
      tooltip = user ? `Assigned to ${user.name}` : 'Unassigned'
      content = full ? (
        <>
          {user ? <Avatar user={user} size={18} /> : <UserRound size={14} color="var(--text-3)" />}
          <span className="lui-chip-text">{user ? user.name : 'Assign'}</span>
        </>
      ) : (
        <Avatar user={user} size={18} />
      )
      break
    }
    case 'labels': {
      const labels = issue.label_ids.map((id) => t.labelsById.get(id)).filter((l): l is NonNullable<typeof l> => Boolean(l))
      empty = labels.length === 0
      if (empty) {
        if (!full) return null
        content = (
          <>
            <Tag size={14} color="var(--text-3)" />
            <span>Add label</span>
          </>
        )
        break
      }
      tooltip = labels.map((l) => l.name).join(', ')
      if (!full && labels.length > maxLabels) {
        return (
          <ChipButton
            refEl={ref}
            className={className}
            tooltip={tooltip}
            onOpen={() => setOpen(true)}
            picker={<LabelPicker issueIds={issueIds ?? [issue.id]} anchor={ref.current} open={open} onOpenChange={setOpen} />}
          >
            <span className="lui-chip-group">
              {labels.slice(0, 3).map((l) => (
                <LabelDot key={l.id} color={l.color} />
              ))}
            </span>
            <span className="lui-chip-text">{labels.length} labels</span>
          </ChipButton>
        )
      }
      if (full) {
        content = (
          <span className="lui-chip-group" style={{ flexWrap: 'wrap', gap: 6 }}>
            {labels.map((l) => (
              <span key={l.id} className="lui-chip">
                <LabelDot color={l.color} />
                <span className="lui-chip-text">{l.name}</span>
              </span>
            ))}
          </span>
        )
        break
      }
      return (
        <span className="lui-chip-group">
          {labels.map((l, index) => (
            <ChipButton
              key={l.id}
              className={className}
              tooltip={l.description || l.name}
              onOpen={(el) => {
                ref.current = el
                setOpen(true)
              }}
              picker={index === 0 ? <LabelPicker issueIds={issueIds ?? [issue.id]} anchor={ref.current} open={open} onOpenChange={setOpen} /> : null}
            >
              <LabelDot color={l.color} />
              <span className="lui-chip-text">{l.name}</span>
            </ChipButton>
          ))}
        </span>
      )
    }
    case 'project': {
      const project = issue.project_id ? t.projectsById.get(issue.project_id) : undefined
      empty = !project
      if (empty && !full) return null
      tooltip = project?.name ?? TOOLTIPS.project.label
      content = project ? (
        <>
          <ProjectIcon icon={project.icon} color={project.color} />
          <span className="lui-chip-text">{project.name}</span>
        </>
      ) : (
        <>
          <ProjectIcon icon="box" color="var(--text-3)" />
          <span>Add to project</span>
        </>
      )
      break
    }
    case 'cycle': {
      const cycle = issue.cycle_id ? t.cyclesById.get(issue.cycle_id) : undefined
      empty = !cycle
      if (empty && !full) return null
      content = cycle ? (
        <>
          <CycleIcon progress={cycleProgressRatio(cycle)} />
          <span className="lui-chip-text">{cycleLabel(cycle)}</span>
        </>
      ) : (
        <>
          <CycleIcon progress={0} />
          <span>Add to cycle</span>
        </>
      )
      break
    }
    case 'estimate': {
      empty = issue.estimate === null
      if (empty && !full) return null
      tooltip = issue.estimate === null ? TOOLTIPS.estimate.label : `Estimate: ${estimateLabel(issue.estimate)}`
      content = (
        <>
          <EstimateIcon />
          <span className="lui-chip-text">{issue.estimate === null ? 'Set estimate' : full ? estimateLabel(issue.estimate) : issue.estimate}</span>
        </>
      )
      break
    }
    case 'due': {
      empty = !issue.due_date
      if (empty && !full) return null
      const state = t.statesById.get(issue.state_id)
      const due = dueState(issue.due_date, state ? isClosedType(state.type) : false)
      tone = due === 'overdue' ? 'lui-chip--overdue' : due === 'soon' ? 'lui-chip--soon' : undefined
      tooltip = issue.due_date ? `Due ${shortDate(issue.due_date)}${due === 'overdue' ? ' (overdue)' : ''}` : TOOLTIPS.due.label
      content = (
        <>
          <CalendarDays size={14} />
          <span className="lui-chip-text">{issue.due_date ? shortDate(issue.due_date) : 'Set due date'}</span>
        </>
      )
      break
    }
    case 'parent': {
      const parent = issue.parent_id ? t.issuesById.get(issue.parent_id) : undefined
      empty = !parent
      if (empty && !full) return null
      const state = parent ? t.statesById.get(parent.state_id) : undefined
      tooltip = parent ? `${parent.identifier} ${parent.title}` : TOOLTIPS.parent.label
      content = parent ? (
        <>
          {state && <StatusIcon type={state.type} color={state.color} progress={stateProgress(state, t.teamStates(parent.team_id))} />}
          <span className="t-3">{parent.identifier}</span>
          <span className="lui-chip-text">{parent.title}</span>
        </>
      ) : (
        <span>Set parent issue</span>
      )
      break
    }
  }

  const Picker = PICKERS[kind]
  const picker = <Picker issueIds={issueIds ?? [issue.id]} anchor={ref.current} open={open} onOpenChange={setOpen} />
  const baseClass = full ? `lui-prop${empty ? ' lui-prop--empty' : ''}` : iconOnly ? 'lui-chip-icon' : `lui-chip${tone ? ` ${tone}` : ''}`

  return (
    <ChipButton
      refEl={ref}
      className={[baseClass, full && tone ? tone : '', className ?? ''].filter(Boolean).join(' ')}
      raw
      tooltip={full ? null : tooltip}
      kbd={TOOLTIPS[kind].kbd}
      onOpen={() => setOpen(true)}
      picker={picker}
    >
      {content}
    </ChipButton>
  )
}

interface ChipButtonProps {
  refEl?: React.MutableRefObject<HTMLButtonElement | null>
  className?: string
  raw?: boolean
  tooltip: React.ReactNode
  kbd?: string[]
  onOpen: (el: HTMLButtonElement) => void
  picker: React.ReactNode
  children: React.ReactNode
}

function ChipButton({ refEl, className, raw, tooltip, kbd, onOpen, picker, children }: ChipButtonProps) {
  const button = (
    <button
      type="button"
      ref={
        refEl
          ? (node) => {
              refEl.current = node
            }
          : undefined
      }
      className={raw ? className : ['lui-chip', className ?? ''].filter(Boolean).join(' ')}
      onClick={(event) => {
        event.stopPropagation()
        event.preventDefault()
        onOpen(event.currentTarget)
      }}
      onDoubleClick={(event) => event.stopPropagation()}
      onPointerDown={(event) => event.stopPropagation()}
      onMouseDown={(event) => event.stopPropagation()}
    >
      {children}
    </button>
  )
  return (
    <>
      {tooltip ? (
        <Tooltip content={tooltip} kbd={kbd}>
          {button}
        </Tooltip>
      ) : (
        button
      )}
      {picker}
    </>
  )
}
