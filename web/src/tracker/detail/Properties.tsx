import { useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { milestoneItems, NONE, parseNullable } from '../create/options'
import { cycleLabel, dueState, isClosedType, shortDate, stateProgress } from '../format'
import { useTracker } from '../store'
import type { Issue } from '../types'
import { PRIORITY_LABELS } from '../types'
import {
  AssigneePicker,
  Avatar,
  CycleIcon,
  CyclePicker,
  DueDatePicker,
  EstimateIcon,
  EstimatePicker,
  LabelDot,
  LabelPicker,
  MenuList,
  ParentPicker,
  Popover,
  PriorityIcon,
  PriorityPicker,
  ProjectIcon,
  ProjectPicker,
  StatusIcon,
  StatusPicker,
} from '../ui'
import type { MenuItem } from '../ui'
import { menuId, notifyError } from './shared'

export type PropertyKind =
  | 'status'
  | 'priority'
  | 'assignee'
  | 'labels'
  | 'project'
  | 'milestone'
  | 'cycle'
  | 'estimate'
  | 'due'
  | 'parent'

export interface PickerState {
  kind: PropertyKind
  anchor: HTMLElement | null
}

export function PropertyPickers({
  issue,
  picker,
  onClose,
}: {
  issue: Issue
  picker: PickerState | null
  onClose: () => void
}) {
  const t = useTracker()
  const ids = [issue.id]
  const common = (kind: PropertyKind) => ({
    issueIds: ids,
    anchor: picker?.kind === kind ? picker.anchor : null,
    open: picker?.kind === kind,
    onOpenChange: (open: boolean) => {
      if (!open) onClose()
    },
  })
  const milestoneOpen = picker?.kind === 'milestone'
  return (
    <>
      <StatusPicker {...common('status')} />
      <PriorityPicker {...common('priority')} />
      <AssigneePicker {...common('assignee')} />
      <LabelPicker {...common('labels')} />
      <ProjectPicker {...common('project')} />
      <CyclePicker {...common('cycle')} />
      <EstimatePicker {...common('estimate')} />
      <DueDatePicker {...common('due')} />
      <ParentPicker {...common('parent')} />
      <Popover
        open={milestoneOpen}
        onOpenChange={(open) => {
          if (!open) onClose()
        }}
        anchor={milestoneOpen ? picker?.anchor ?? null : null}
        align="start"
        width={240}
      >
        <MenuList
          items={milestoneItems(t, issue.project_id)}
          searchable
          placeholder="Set milestone…"
          selectedIds={[issue.milestone_id ?? NONE]}
          emptyText="No milestones in this project"
          onSelect={(item: MenuItem | string) => {
            onClose()
            t.updateIssues(ids, { milestone_id: parseNullable(menuId(item)) }).catch(notifyError)
          }}
        />
      </Popover>
    </>
  )
}

function Row({
  label,
  onOpen,
  children,
  rowRef,
  className = '',
}: {
  label: string
  onOpen: (anchor: HTMLElement) => void
  children: ReactNode
  rowRef?: (el: HTMLButtonElement | null) => void
  className?: string
}) {
  return (
    <div className="ld-prop">
      <span className="ld-prop-label">{label}</span>
      <button
        type="button"
        ref={rowRef}
        className={`ld-prop-value ${className}`}
        onClick={(e) => onOpen(e.currentTarget)}
      >
        {children}
      </button>
    </div>
  )
}

export function usePropertyAnchors() {
  const anchors = useRef(new Map<PropertyKind, HTMLElement>())
  const register = (kind: PropertyKind) => (el: HTMLElement | null) => {
    if (el) anchors.current.set(kind, el)
    else anchors.current.delete(kind)
  }
  return { anchors, register }
}

export function PropertiesSidebar({
  issue,
  openPicker,
  register,
}: {
  issue: Issue
  openPicker: (kind: PropertyKind, anchor: HTMLElement) => void
  register: (kind: PropertyKind) => (el: HTMLElement | null) => void
}) {
  const t = useTracker()
  const state = t.statesById.get(issue.state_id)
  const teamStates = t.teamStates(issue.team_id)
  const assignee = issue.assignee_id ? t.usersById.get(issue.assignee_id) : undefined
  const project = issue.project_id ? t.projectsById.get(issue.project_id) : undefined
  const milestone = issue.milestone_id ? t.milestones.find((m) => m.id === issue.milestone_id) : undefined
  const cycle = issue.cycle_id ? t.cyclesById.get(issue.cycle_id) : undefined
  const parent = issue.parent_id ? t.issuesById.get(issue.parent_id) : undefined
  const labels = issue.label_ids.map((id) => t.labelsById.get(id)).filter((l) => l !== undefined)
  const due = dueState(issue.due_date, state ? isClosedType(state.type) : false)

  return (
    <div className="ld-props">
      <div className="ld-props-group">
        <Row label="Status" onOpen={(a) => openPicker('status', a)} rowRef={register('status')}>
          {state && <StatusIcon type={state.type} color={state.color} progress={stateProgress(state, teamStates)} size={14} />}
          <span className="ld-text">{state?.name ?? 'Unknown'}</span>
        </Row>
        <Row label="Priority" onOpen={(a) => openPicker('priority', a)} rowRef={register('priority')}>
          <PriorityIcon priority={issue.priority} size={14} />
          <span className={issue.priority === 0 ? 'ld-muted' : 'ld-text'}>{PRIORITY_LABELS[issue.priority]}</span>
        </Row>
        <Row label="Assignee" onOpen={(a) => openPicker('assignee', a)} rowRef={register('assignee')}>
          <Avatar user={assignee ?? null} size={20} />
          <span className={assignee ? 'ld-text' : 'ld-muted'}>{assignee?.name ?? 'Assign'}</span>
        </Row>
      </div>

      <div className="ld-props-group">
        <Row
          label="Labels"
          className="ld-labels"
          onOpen={(a) => openPicker('labels', a)}
          rowRef={register('labels')}
        >
          {labels.length === 0 ? (
            <span className="ld-muted">Add label</span>
          ) : (
            labels.map((l) => (
              <span key={l.id} className="ld-label-pill">
                <LabelDot color={l.color} />
                <span className="ld-text">{l.name}</span>
              </span>
            ))
          )}
        </Row>
      </div>

      <div className="ld-props-group">
        <Row label="Project" onOpen={(a) => openPicker('project', a)} rowRef={register('project')}>
          {project ? <ProjectIcon icon={project.icon} color={project.color} /> : null}
          <span className={project ? 'ld-text' : 'ld-muted'}>{project?.name ?? 'Add to project'}</span>
        </Row>
        {project && (
          <Row label="Milestone" onOpen={(a) => openPicker('milestone', a)} rowRef={register('milestone')}>
            <span className={milestone ? 'ld-text' : 'ld-muted'}>{milestone?.name ?? 'Add milestone'}</span>
          </Row>
        )}
        <Row label="Cycle" onOpen={(a) => openPicker('cycle', a)} rowRef={register('cycle')}>
          <CycleIcon progress={cycle ? 0.5 : 0} />
          <span className={cycle ? 'ld-text' : 'ld-muted'}>{cycle ? cycleLabel(cycle) : 'Add to cycle'}</span>
        </Row>
        <Row label="Estimate" onOpen={(a) => openPicker('estimate', a)} rowRef={register('estimate')}>
          <EstimateIcon />
          <span className={issue.estimate === null ? 'ld-muted' : 'ld-text'}>
            {issue.estimate === null ? 'Set estimate' : `${issue.estimate} ${issue.estimate === 1 ? 'point' : 'points'}`}
          </span>
        </Row>
        <Row
          label="Due date"
          onOpen={(a) => openPicker('due', a)}
          rowRef={register('due')}
          className={due === 'overdue' ? 'ld-due-overdue' : due === 'soon' ? 'ld-due-soon' : ''}
        >
          <span className={issue.due_date ? 'ld-text' : 'ld-muted'}>
            {issue.due_date ? shortDate(issue.due_date) : 'Set due date'}
          </span>
        </Row>
        <Row label="Parent" onOpen={(a) => openPicker('parent', a)} rowRef={register('parent')}>
          <span className={parent ? 'ld-text' : 'ld-muted'}>
            {parent ? `${parent.identifier} ${parent.title}` : 'Set parent issue'}
          </span>
        </Row>
      </div>
    </div>
  )
}

export function usePickerState() {
  const [picker, setPicker] = useState<PickerState | null>(null)
  return { picker, setPicker, close: () => setPicker(null) }
}
