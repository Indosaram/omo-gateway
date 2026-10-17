import { ChevronRight, Plus } from 'lucide-react'
import { Fragment } from 'react'
import { groupIssues, type IssueGroup, type TrackerState } from '../store'
import type { Issue } from '../types'
import { Checkbox, ContextMenu, PriorityIcon } from '../ui'
import { pickId, useCollection } from './model'
import { AssigneeButton, GroupIcon, IssueChips, IssueStatusIcon } from './shared'
import { issueContextItems } from './contextItems'

function GroupHeader({
  group,
  parents,
  depth,
  collapsed,
  t,
}: {
  group: IssueGroup
  parents: IssueGroup[]
  depth: number
  collapsed: boolean
  t: TrackerState
}) {
  const c = useCollection()
  const key = [...parents.map((p) => p.key), group.key].join('/')
  const estimate = group.issues.reduce((sum, issue) => sum + (issue.estimate ?? 0), 0)
  return (
    <div
      className={`lic-group${depth > 0 ? ' lic-group--sub' : ''}`}
      data-group-header={group.key}
      onClick={() => c.toggleCollapsed(key)}
      role="button"
      aria-expanded={!collapsed}
    >
      <span className={`lic-group-chevron${collapsed ? '' : ' lic-group-chevron--open'}`}>
        <ChevronRight size={12} />
      </span>
      <GroupIcon group={group} t={t} />
      <span className="lic-group-name w-510">{group.label}</span>
      <span className="lic-group-count t-3 tabular">{group.issues.length}</span>
      {estimate > 0 && <span className="lic-group-count t-4 tabular">· {estimate} pts</span>}
      <span className="lic-grow" />
      <button
        type="button"
        className="lic-group-add"
        title="Create issue in this group"
        aria-label="Create issue in this group"
        onClick={(event) => {
          event.stopPropagation()
          c.createInGroup([...parents, group])
        }}
      >
        <Plus size={14} />
      </button>
    </div>
  )
}

export function IssueRow({ issue, t }: { issue: Issue; t: TrackerState }) {
  const c = useCollection()
  const props = c.display.properties
  const selected = c.selection.has(issue.id)
  const focused = c.focusedId === issue.id
  const has = (p: (typeof props)[number]) => props.includes(p)
  const parent = issue.parent_id ? t.issuesById.get(issue.parent_id) : undefined
  return (
    <ContextMenu items={() => issueContextItems(issue, t)} onSelect={(value: unknown) => c.contextAction(issue, pickId(value))}>
      <div
        className={`lic-row${selected ? ' lic-row--selected' : ''}${focused ? ' lic-row--focused' : ''}`}
        data-issue-row={issue.identifier}
        data-issue-id={issue.id}
        tabIndex={focused ? 0 : -1}
        aria-selected={selected}
        role="option"
        onClick={(event) => c.onRowClick(issue, event)}
        onMouseMove={() => c.onRowHover(issue)}
        onFocus={() => c.onRowHover(issue)}
      >
        <span
          className={`lic-row-check${selected || c.selection.size > 0 ? ' lic-row-check--on' : ''}`}
          onClick={(event) => {
            event.stopPropagation()
            c.toggleSelected(issue)
          }}
        >
          <Checkbox checked={selected} onChange={() => c.toggleSelected(issue)} />
        </span>
        {has('priority') && (
          <button
            type="button"
            className="lic-row-icon"
            title="Set priority"
            onClick={(event) => {
              event.stopPropagation()
              c.openPicker('priority', issue, event.currentTarget)
            }}
          >
            <PriorityIcon priority={issue.priority} size={14} />
          </button>
        )}
        {has('id') && <span className="lic-row-id t-3 tabular">{issue.identifier}</span>}
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
        <span className="lic-row-title">
          <span className="lic-row-title-text w-510">{issue.title}</span>
          {parent && (
            <span className="lic-row-parent t-3">
              <ChevronRight size={11} />
              {parent.title}
            </span>
          )}
        </span>
        <span className="lic-row-props">
          <IssueChips issue={issue} t={t} properties={props} />
        </span>
        {has('assignee') && <AssigneeButton issue={issue} t={t} />}
      </div>
    </ContextMenu>
  )
}

function GroupBlock({
  group,
  parents,
  t,
}: {
  group: IssueGroup
  parents: IssueGroup[]
  t: TrackerState
}) {
  const c = useCollection()
  const key = [...parents.map((p) => p.key), group.key].join('/')
  const collapsed = c.collapsed.has(key)
  const depth = parents.length
  const sub = depth === 0 ? c.display.sub_grouping : 'none'
  const subGroups =
    sub !== 'none' && sub !== group.grouping
      ? groupIssues(group.issues, sub, t, { teamId: c.teamId, showEmpty: false })
      : null
  return (
    <section className="lic-group-block" data-group={group.key}>
      {group.grouping !== 'none' && <GroupHeader group={group} parents={parents} depth={depth} collapsed={collapsed} t={t} />}
      {!collapsed &&
        (subGroups
          ? subGroups.map((child) => <GroupBlock key={child.key} group={child} parents={[...parents, group]} t={t} />)
          : group.issues.map((issue) => <IssueRow key={issue.id} issue={issue} t={t} />))}
    </section>
  )
}

export function ListView({ groups, t }: { groups: IssueGroup[]; t: TrackerState }) {
  return (
    <div className="lic-list" role="listbox" aria-multiselectable="true">
      {groups.map((group) => (
        <Fragment key={group.key}>
          <GroupBlock group={group} parents={[]} t={t} />
        </Fragment>
      ))}
    </div>
  )
}

export function listOrder(groups: IssueGroup[], collapsed: Set<string>, subGrouping: IssueGroup['grouping'], t: TrackerState, teamId: string | null): Issue[] {
  const out: Issue[] = []
  const seen = new Set<string>()
  const walk = (group: IssueGroup, parents: IssueGroup[]) => {
    const key = [...parents.map((p) => p.key), group.key].join('/')
    if (group.grouping !== 'none' && collapsed.has(key)) return
    const sub = parents.length === 0 ? subGrouping : 'none'
    if (sub !== 'none' && sub !== group.grouping) {
      for (const child of groupIssues(group.issues, sub, t, { teamId, showEmpty: false })) walk(child, [...parents, group])
      return
    }
    for (const issue of group.issues) {
      if (seen.has(issue.id)) continue
      seen.add(issue.id)
      out.push(issue)
    }
  }
  for (const group of groups) walk(group, [])
  return out
}
