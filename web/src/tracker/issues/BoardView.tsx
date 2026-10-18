import {
  DndContext,
  DragOverlay,
  PointerSensor,
  closestCorners,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from '@dnd-kit/core'
import { SortableContext, useSortable, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { Ellipsis, Plus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { sortOrderBetween, type IssueGroup, type TrackerState } from '../store'
import type { Issue, IssuePatch } from '../types'
import { ContextMenu, PriorityIcon } from '../ui'
import { issueContextItems } from './contextItems'
import { mapPatchToIssueTeam, patchForMove, pickId, updateWithLaneToast, useCollection } from './model'
import { AssigneeButton, GroupIcon, IssueChips, IssueStatusIcon } from './shared'

const COLUMN_PREFIX = 'col:'

function CardBody({ issue, t, overlay = false }: { issue: Issue; t: TrackerState; overlay?: boolean }) {
  const c = useCollection()
  const props = c.display.properties
  const has = (p: (typeof props)[number]) => props.includes(p)
  return (
    <div className={`lic-card-inner${overlay ? ' lic-card-inner--overlay' : ''}`}>
      <div className="lic-card-top">
        {has('id') && <span className="lic-card-id t-3 tabular">{issue.identifier}</span>}
        <span className="lic-grow" />
        {has('assignee') && <AssigneeButton issue={issue} t={t} />}
      </div>
      <div className="lic-card-title">
        {has('status') && (
          <button
            type="button"
            className="lic-row-icon"
            title="Change status"
            onClick={(event) => {
              event.stopPropagation()
              c.openPicker('status', issue, event.currentTarget)
            }}
          >
            <IssueStatusIcon issue={issue} t={t} />
          </button>
        )}
        <span className="w-510">{issue.title}</span>
      </div>
      <div className="lic-card-props">
        {has('priority') && (
          <button
            type="button"
            className="lic-chip lic-chip--icon"
            title="Set priority"
            onClick={(event) => {
              event.stopPropagation()
              c.openPicker('priority', issue, event.currentTarget)
            }}
          >
            <PriorityIcon priority={issue.priority} size={14} />
          </button>
        )}
        <IssueChips issue={issue} t={t} properties={props} compact />
      </div>
    </div>
  )
}

function BoardCard({ issue, t }: { issue: Issue; t: TrackerState }) {
  const c = useCollection()
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: issue.id })
  const selected = c.selection.has(issue.id)
  const focused = c.focusedId === issue.id
  return (
    <ContextMenu items={() => issueContextItems(issue, t)} onSelect={(value: unknown) => c.contextAction(issue, pickId(value))}>
      <div
        ref={setNodeRef}
        style={{ transform: CSS.Transform.toString(transform), transition }}
        className={`lic-card${selected ? ' lic-card--selected' : ''}${focused ? ' lic-card--focused' : ''}${isDragging ? ' lic-card--dragging' : ''}`}
        data-issue-card={issue.identifier}
        data-issue-id={issue.id}
        onClick={(event) => c.onRowClick(issue, event)}
        onMouseMove={() => c.onRowHover(issue)}
        {...attributes}
        {...listeners}
        tabIndex={focused ? 0 : -1}
        role="option"
        aria-selected={selected}
      >
        <CardBody issue={issue} t={t} />
      </div>
    </ContextMenu>
  )
}

function BoardColumn({ group, t }: { group: IssueGroup; t: TrackerState }) {
  const c = useCollection()
  const { setNodeRef, isOver } = useDroppable({ id: `${COLUMN_PREFIX}${group.key}` })
  return (
    <div
      className={`lic-col${isOver ? ' lic-col--over' : ''}`}
      data-board-column={group.key}
      data-board-column-name={group.label}
    >
      <div className="lic-col-head">
        <GroupIcon group={group} t={t} />
        <span className="w-510 lic-col-name">{group.label}</span>
        <span className="t-3 tabular">{group.issues.length}</span>
        <span className="lic-grow" />
        <button type="button" className="lic-col-btn" aria-label="Column options" title="Column options">
          <Ellipsis size={14} />
        </button>
        <button
          type="button"
          className="lic-col-btn"
          aria-label="Create issue"
          title="Create issue"
          onClick={() => c.createInGroup([group])}
        >
          <Plus size={14} />
        </button>
      </div>
      <SortableContext items={group.issues.map((i) => i.id)} strategy={verticalListSortingStrategy}>
        <div ref={setNodeRef} className="lic-col-body">
          {group.issues.map((issue) => (
            <BoardCard key={issue.id} issue={issue} t={t} />
          ))}
          {group.issues.length === 0 && <div className="lic-col-empty t-4">No issues</div>}
        </div>
      </SortableContext>
    </div>
  )
}

export function BoardView({ groups, t }: { groups: IssueGroup[]; t: TrackerState }) {
  const c = useCollection()
  const [activeId, setActiveId] = useState<string | null>(null)
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }))
  const groupOfIssue = useMemo(() => {
    const map = new Map<string, IssueGroup>()
    for (const group of groups) for (const issue of group.issues) if (!map.has(issue.id)) map.set(issue.id, group)
    return map
  }, [groups])
  const active = activeId ? t.issuesById.get(activeId) : undefined

  const onDragStart = (event: DragStartEvent) => setActiveId(String(event.active.id))

  const onDragEnd = (event: DragEndEvent) => {
    setActiveId(null)
    const overId = event.over ? String(event.over.id) : null
    if (!overId) return
    const issueId = String(event.active.id)
    const issue = t.issuesById.get(issueId)
    if (!issue) return
    const from = groupOfIssue.get(issueId)
    const to = overId.startsWith(COLUMN_PREFIX)
      ? groups.find((g) => g.key === overId.slice(COLUMN_PREFIX.length))
      : groupOfIssue.get(overId)
    if (!to) return

    let patch: IssuePatch = {}
    if (from?.key !== to.key) {
      const movePatch = patchForMove(from, to)
      if (!movePatch) return
      patch = mapPatchToIssueTeam(movePatch, issue, t)
    }
    if (c.display.ordering === 'manual') {
      const siblings = to.issues.filter((i) => i.id !== issueId)
      const overIndex = overId.startsWith(COLUMN_PREFIX) ? siblings.length : siblings.findIndex((i) => i.id === overId)
      const index = overIndex < 0 ? siblings.length : overIndex
      const before = index > 0 ? siblings[index - 1].sort_order : null
      const after = index < siblings.length ? siblings[index].sort_order : null
      patch.sort_order = sortOrderBetween(before, after)
    }
    if (Object.keys(patch).length === 0) return
    void updateWithLaneToast(t, [issueId], patch)
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onDragCancel={() => setActiveId(null)}
    >
      <div className="lic-board">
        {groups.map((group) => (
          <BoardColumn key={group.key} group={group} t={t} />
        ))}
      </div>
      <DragOverlay dropAnimation={{ duration: 160, easing: 'cubic-bezier(.2,0,0,1)' }}>
        {active ? (
          <div className="lic-card lic-card--overlay">
            <CardBody issue={active} t={t} overlay />
          </div>
        ) : null}
      </DragOverlay>
    </DndContext>
  )
}
