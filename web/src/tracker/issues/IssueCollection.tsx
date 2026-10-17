import { Columns3, LayoutList, ListFilter, Plus, SlidersHorizontal } from 'lucide-react'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { isEditableTarget, useHotkey } from '../hotkeys'
import { hrefFor, navigate } from '../router'
import { filterCount, useTracker } from '../store'
import type { ID, Issue, StateType, ViewDisplay, ViewFilters } from '../types'
import {
  AssigneePicker,
  Button,
  CyclePicker,
  Dialog,
  DueDatePicker,
  EstimatePicker,
  Kbd,
  LabelPicker,
  PriorityPicker,
  ProjectPicker,
  StatusPicker,
  Tooltip,
  toast,
  type PickerProps,
} from '../ui'
import { BoardView } from './BoardView'
import { BulkBar } from './BulkBar'
import { DisplayOptions } from './DisplayOptions'
import { FilterChips, FilterPopover, type FilterField } from './FilterMenu'
import { ListView, listOrder } from './ListView'
import {
  CollectionContext,
  draftForGroups,
  errorMessage,
  readLocal,
  useCollectionModel,
  writeLocal,
  type CollectionContextValue,
  type CollectionLayout,
  type PickerKind,
  type PickerRequest,
  updateWithLaneToast,
} from './model'
import { SaveViewDialog } from './SaveViewDialog'
import { TimelineView } from './TimelineView'
import './issues.css'

export interface IssueCollectionProps {
  issues: Issue[]
  viewKey: string
  defaultDisplay?: Partial<ViewDisplay>
  emptyState?: React.ReactNode
  showTeamTabs?: boolean
  teamId?: ID | null
  stateTypes?: StateType[]
  renderHeader?: (toolbar: React.ReactNode) => React.ReactNode
  toolbarExtra?: React.ReactNode
  onSaveView?: (filters: ViewFilters, display: ViewDisplay) => void
}

const PICKERS: Record<PickerKind, React.ComponentType<PickerProps>> = {
  status: StatusPicker,
  priority: PriorityPicker,
  assignee: AssigneePicker,
  labels: LabelPicker,
  project: ProjectPicker,
  cycle: CyclePicker,
  estimate: EstimatePicker,
  due: DueDatePicker,
}

function useGPrefix(): boolean {
  const [pending, setPending] = useState(false)
  useEffect(() => {
    let timer: number | undefined
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey || isEditableTarget(event.target)) return
      window.clearTimeout(timer)
      if (event.code === 'KeyG' && !event.shiftKey) {
        setPending(true)
        timer = window.setTimeout(() => setPending(false), 900)
      } else {
        timer = window.setTimeout(() => setPending(false), 0)
      }
    }
    window.addEventListener('keydown', onKeyDown, true)
    return () => {
      window.clearTimeout(timer)
      window.removeEventListener('keydown', onKeyDown, true)
    }
  }, [])
  return pending
}

export function IssueCollection({
  issues,
  viewKey,
  defaultDisplay,
  emptyState,
  showTeamTabs = false,
  teamId = null,
  stateTypes,
  renderHeader,
  toolbarExtra,
  onSaveView,
}: IssueCollectionProps) {
  const t = useTracker()
  const rawDisplay = t.displayFor(viewKey, defaultDisplay)
  const rawFilters = t.filtersFor(viewKey)
  const displaySig = JSON.stringify(rawDisplay)
  const filtersSig = JSON.stringify(rawFilters)
  const display = useMemo(() => rawDisplay, [displaySig])
  const filters = useMemo(() => rawFilters, [filtersSig])
  const setDisplay = (next: ViewDisplay) => t.setDisplayFor(viewKey, next)
  const setFilters = (next: ViewFilters) => t.setFiltersFor(viewKey, next)

  const timelineKey = `tracker.timeline.${viewKey}`
  const collapseKey = `tracker.collapsed.${viewKey}`
  const [timeline, setTimeline] = useState<boolean>(() => readLocal(timelineKey, false))
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set(readLocal<string[]>(collapseKey, [])))
  useEffect(() => {
    setTimeline(readLocal(timelineKey, false))
    setCollapsed(new Set(readLocal<string[]>(collapseKey, [])))
  }, [timelineKey, collapseKey])
  const layout: CollectionLayout = timeline ? 'timeline' : display.layout

  const setLayout = (next: CollectionLayout) => {
    const isTimeline = next === 'timeline'
    setTimeline(isTimeline)
    writeLocal(timelineKey, isTimeline)
    if (!isTimeline && next !== display.layout) setDisplay({ ...display, layout: next })
  }

  const toggleCollapsed = useCallback(
    (key: string) => {
      setCollapsed((current) => {
        const next = new Set(current)
        if (next.has(key)) next.delete(key)
        else next.add(key)
        writeLocal(collapseKey, [...next])
        return next
      })
    },
    [collapseKey],
  )

  const { visible, groups } = useCollectionModel(issues, filters, display, layout, teamId, stateTypes, t)

  const order = useMemo(() => {
    if (layout === 'list') return listOrder(groups, collapsed, display.sub_grouping, t, teamId)
    const seen = new Set<ID>()
    const out: Issue[] = []
    for (const group of groups)
      for (const issue of group.issues) {
        if (seen.has(issue.id)) continue
        seen.add(issue.id)
        out.push(issue)
      }
    return out
  }, [layout, groups, collapsed, display.sub_grouping, t, teamId])

  const [picker, setPicker] = useState<PickerRequest | null>(null)
  const [filterOpen, setFilterOpen] = useState<{ anchor: HTMLElement | null; field: FilterField | null } | null>(null)
  const [displayAnchor, setDisplayAnchor] = useState<HTMLElement | null>(null)
  const [saveOpen, setSaveOpen] = useState(false)
  const [deleteIds, setDeleteIds] = useState<ID[] | null>(null)
  const anchorRef = useRef<ID | null>(null)
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const filterBtnRef = useRef<HTMLButtonElement | null>(null)
  const displayBtnRef = useRef<HTMLButtonElement | null>(null)

  const selection = t.selection
  const focusedId = t.focusedIssueId && order.some((i) => i.id === t.focusedIssueId) ? t.focusedIssueId : null

  useEffect(() => {
    t.setSelection(new Set())
    anchorRef.current = null
  }, [viewKey])

  useEffect(() => {
    const ids = new Set(visible.map((i) => i.id))
    let changed = false
    const next = new Set<ID>()
    for (const id of t.selection) {
      if (ids.has(id)) next.add(id)
      else changed = true
    }
    if (changed) t.setSelection(next)
  }, [visible])

  const rowElement = (id: ID): HTMLElement | null =>
    scrollRef.current?.querySelector<HTMLElement>(`[data-issue-id="${CSS.escape(id)}"]`) ?? null

  const focusIssue = (id: ID | null, scroll = true) => {
    t.setFocusedIssueId(id)
    if (!id || !scroll) return
    requestAnimationFrame(() => {
      const el = rowElement(id)
      if (el) {
        el.focus({ preventScroll: true })
        el.scrollIntoView({ block: 'nearest' })
      }
    })
  }

  const targetIds = (issue?: Issue): ID[] => {
    if (issue) return selection.has(issue.id) && selection.size > 1 ? [...selection] : [issue.id]
    if (selection.size > 0) return [...selection]
    return focusedId ? [focusedId] : []
  }

  const openPicker = (kind: PickerKind, issue?: Issue, anchor?: HTMLElement | DOMRect | null) => {
    const ids = targetIds(issue)
    if (ids.length === 0) return
    const fallback = rowElement(issue?.id ?? focusedId ?? ids[0])
    setPicker({ kind, ids, anchor: anchor ?? fallback ?? null })
  }

  const toggleSelected = (issue: Issue) => {
    const next = new Set(selection)
    if (next.has(issue.id)) next.delete(issue.id)
    else next.add(issue.id)
    anchorRef.current = issue.id
    t.setSelection(next)
  }

  const selectRange = (to: Issue) => {
    const fromId = anchorRef.current ?? focusedId ?? to.id
    const a = order.findIndex((i) => i.id === fromId)
    const b = order.findIndex((i) => i.id === to.id)
    if (a < 0 || b < 0) {
      toggleSelected(to)
      return
    }
    const [lo, hi] = a < b ? [a, b] : [b, a]
    const next = new Set(selection)
    for (let i = lo; i <= hi; i++) next.add(order[i].id)
    t.setSelection(next)
  }

  const openIssue = (issue: Issue) => navigate({ name: 'issue', identifier: issue.identifier })

  const onRowClick = (issue: Issue, event: React.MouseEvent) => {
    focusIssue(issue.id, false)
    if (event.shiftKey) {
      event.preventDefault()
      selectRange(issue)
      return
    }
    if (event.metaKey || event.ctrlKey) {
      toggleSelected(issue)
      return
    }
    if (selection.size > 0) {
      toggleSelected(issue)
      return
    }
    openIssue(issue)
  }

  const copy = (text: string, message: string) => {
    navigator.clipboard
      .writeText(text)
      .then(() => toast(message))
      .catch((err: unknown) => toast.error(errorMessage(err)))
  }

  const assignToMe = (ids: ID[]) => {
    if (!t.me || ids.length === 0) return
    const allMine = ids.every((id) => t.issuesById.get(id)?.assignee_id === t.me?.id)
    void updateWithLaneToast(t, ids, { assignee_id: allMine ? null : t.me.id })
  }

  const archive = (ids: ID[]) => {
    if (ids.length === 0) return
    const archived = ids.every((id) => t.issuesById.get(id)?.archived_at)
    void updateWithLaneToast(t, ids, { archived: !archived }).then((ok) => {
      if (ok) toast(`${ids.length === 1 ? 'Issue' : `${ids.length} issues`} ${archived ? 'unarchived' : 'archived'}`)
    })
  }

  const contextAction = (issue: Issue, actionId: string) => {
    const ids = targetIds(issue)
    switch (actionId) {
      case 'status':
      case 'priority':
      case 'assignee':
      case 'labels':
      case 'due':
        openPicker(actionId as PickerKind, issue)
        break
      case 'assign_me':
        assignToMe(ids)
        break
      case 'copy_id':
        copy(issue.identifier, `Copied ${issue.identifier}`)
        break
      case 'copy_link':
        copy(`${window.location.origin}${window.location.pathname}${hrefFor({ name: 'issue', identifier: issue.identifier })}`, 'Link copied')
        break
      case 'archive':
        archive(ids)
        break
      case 'delete':
        setDeleteIds(ids)
        break
    }
  }

  const ctx: CollectionContextValue = {
    viewKey,
    display,
    teamId,
    focusedId,
    selection,
    collapsed,
    toggleCollapsed,
    openPicker: (kind, issue, anchor) => openPicker(kind, issue, anchor),
    onRowClick,
    onRowHover: (issue) => {
      if (t.focusedIssueId !== issue.id) t.setFocusedIssueId(issue.id)
    },
    toggleSelected,
    createInGroup: (gs) => t.openCreate(draftForGroups(gs, teamId, t)),
    contextAction,
  }

  const overlayOpen =
    picker !== null ||
    filterOpen !== null ||
    displayAnchor !== null ||
    saveOpen ||
    deleteIds !== null ||
    t.createDraft !== null ||
    t.paletteOpen ||
    t.shortcutsOpen
  const listKeys = { enabled: !overlayOpen }
  const issueKeys = { enabled: !overlayOpen && t.peekIssueId === null }

  const move = (delta: number) => {
    if (order.length === 0) return
    const index = focusedId ? order.findIndex((i) => i.id === focusedId) : -1
    const next = index < 0 ? (delta > 0 ? 0 : order.length - 1) : Math.min(order.length - 1, Math.max(0, index + delta))
    const target = order[next]
    focusIssue(target.id)
    if (t.peekIssueId) t.setPeekIssueId(target.id)
    return target
  }

  const moveColumn = (delta: number) => {
    if (layout !== 'board' || groups.length === 0) return
    const col = focusedId ? groups.findIndex((g) => g.issues.some((i) => i.id === focusedId)) : -1
    const row = col >= 0 ? groups[col].issues.findIndex((i) => i.id === focusedId) : 0
    for (let c = col + delta; c >= 0 && c < groups.length; c += delta) {
      const issues = groups[c].issues
      if (issues.length === 0) continue
      focusIssue(issues[Math.min(row, issues.length - 1)].id)
      return
    }
  }

  useHotkey(['j', 'arrowdown'], () => move(1), listKeys)
  useHotkey(['k', 'arrowup'], () => move(-1), listKeys)
  const extendSelection = (delta: number) => {
    const before = focusedId
    const target = move(delta)
    if (!target) return
    const next = new Set(selection)
    if (before) next.add(before)
    next.add(target.id)
    t.setSelection(next)
  }
  useHotkey(['shift+arrowdown', 'shift+j'], () => extendSelection(1), issueKeys)
  useHotkey(['shift+arrowup', 'shift+k'], () => extendSelection(-1), issueKeys)
  useHotkey('arrowright', () => moveColumn(1), { enabled: listKeys.enabled && layout === 'board' })
  useHotkey('arrowleft', () => moveColumn(-1), { enabled: listKeys.enabled && layout === 'board' })
  useHotkey(['enter', 'o'], () => {
    const issue = focusedId ? t.issuesById.get(focusedId) : undefined
    if (issue) openIssue(issue)
  }, issueKeys)
  useHotkey('space', () => {
    if (t.peekIssueId) {
      t.setPeekIssueId(null)
      return
    }
    if (focusedId) t.setPeekIssueId(focusedId)
  }, { enabled: !overlayOpen && (focusedId !== null || t.peekIssueId !== null) })
  useHotkey('x', () => {
    const issue = focusedId ? t.issuesById.get(focusedId) : undefined
    if (issue) toggleSelected(issue)
  }, issueKeys)
  useHotkey('shift+x', () => {
    const issue = focusedId ? t.issuesById.get(focusedId) : undefined
    if (issue) selectRange(issue)
  }, issueKeys)
  useHotkey('mod+a', () => t.setSelection(new Set(order.map((i) => i.id))), issueKeys)
  useHotkey('escape', () => {
    if (selection.size > 0) t.setSelection(new Set())
    else if (filterCount(filters) > 0) setFilters({})
  }, { enabled: !overlayOpen && t.peekIssueId === null && (selection.size > 0 || filterCount(filters) > 0) })
  useHotkey('mod+b', () => setLayout(layout === 'board' ? 'list' : 'board'), { enabled: !overlayOpen, allowInInputs: false })
  useHotkey('f', () => setFilterOpen({ anchor: filterBtnRef.current, field: null }), listKeys)
  useHotkey('shift+v', () => setDisplayAnchor(displayBtnRef.current), listKeys)

  const hasTarget = selection.size > 0 || focusedId !== null
  const gPending = useGPrefix()
  const targetKeys = { enabled: issueKeys.enabled && hasTarget && !gPending }
  useHotkey('s', () => openPicker('status'), targetKeys)
  useHotkey('p', () => openPicker('priority'), targetKeys)
  useHotkey('a', () => openPicker('assignee'), targetKeys)
  useHotkey('l', () => openPicker('labels'), targetKeys)
  useHotkey('shift+p', () => openPicker('project'), targetKeys)
  useHotkey('shift+c', () => openPicker('cycle'), targetKeys)
  useHotkey('shift+e', () => openPicker('estimate'), targetKeys)
  useHotkey('shift+d', () => openPicker('due'), targetKeys)
  useHotkey('i', () => assignToMe(targetIds()), targetKeys)
  useHotkey('mod+.', () => {
    const ids = targetIds()
    const text = ids.map((id) => t.issuesById.get(id)?.identifier).filter(Boolean).join(', ')
    if (text) copy(text, `Copied ${text}`)
  }, targetKeys)
  useHotkey('mod+shift+a', () => archive(targetIds()), targetKeys)
  useHotkey('mod+backspace', () => setDeleteIds(targetIds()), targetKeys)

  const runDelete = async (ids: ID[]) => {
    setDeleteIds(null)
    let failed = 0
    let last = ''
    for (const id of ids) {
      try {
        await t.deleteIssue(id)
      } catch (err) {
        failed += 1
        last = errorMessage(err)
      }
    }
    if (failed > 0) toast.error(`${failed} of ${ids.length} could not be deleted: ${last}`)
    else toast(`${ids.length === 1 ? 'Issue' : `${ids.length} issues`} deleted`)
    t.setSelection(new Set())
  }

  const activeFilters = filterCount(filters)
  const toolbar = (
    <div className="lic-toolbar">
      <Tooltip content="Filter" kbd={['F']}>
        <button
          ref={filterBtnRef}
          type="button"
          className={`lic-tool-btn${activeFilters > 0 ? ' lic-tool-btn--on' : ''}`}
          data-filter-button
          onClick={(e) => setFilterOpen({ anchor: e.currentTarget, field: null })}
        >
          <ListFilter size={14} />
          <span className="lic-tool-label">Filter</span>
          {activeFilters > 0 && <span className="lic-tool-count tabular">{activeFilters}</span>}
        </button>
      </Tooltip>
      <span className="lic-grow" />
      {toolbarExtra}
      {activeFilters > 0 && (
        <Button
          size="sm"
          variant="ghost"
          onClick={() => (onSaveView ? onSaveView(filters, display) : setSaveOpen(true))}
          data-save-view
        >
          Save view
        </Button>
      )}
      <div className="lic-seg" role="group" aria-label="Layout">
        <Tooltip content="List" kbd={['⌘', 'B']}>
          <button
            type="button"
            className={`lic-seg-btn${layout === 'list' ? ' lic-seg-btn--on' : ''}`}
            aria-pressed={layout === 'list'}
            aria-label="List layout"
            data-layout="list"
            onClick={() => setLayout('list')}
          >
            <LayoutList size={14} />
          </button>
        </Tooltip>
        <Tooltip content="Board" kbd={['⌘', 'B']}>
          <button
            type="button"
            className={`lic-seg-btn${layout === 'board' ? ' lic-seg-btn--on' : ''}`}
            aria-pressed={layout === 'board'}
            aria-label="Board layout"
            data-layout="board"
            onClick={() => setLayout('board')}
          >
            <Columns3 size={14} />
          </button>
        </Tooltip>
      </div>
      <Tooltip content="Display options" kbd={['⇧', 'V']}>
        <button
          ref={displayBtnRef}
          type="button"
          className="lic-tool-btn"
          data-display-button
          onClick={(e) => setDisplayAnchor(e.currentTarget)}
        >
          <SlidersHorizontal size={14} />
          <span className="lic-tool-label">Display</span>
        </button>
      </Tooltip>
    </div>
  )

  const Picker = picker ? PICKERS[picker.kind] : null

  return (
    <CollectionContext.Provider value={ctx}>
      {renderHeader ? renderHeader(toolbar) : <div className="lic-toolbar-row">{toolbar}</div>}
      {(activeFilters > 0) && (
        <FilterChips
          filters={filters}
          onChange={setFilters}
          t={t}
          onEditField={(field, anchor) => setFilterOpen({ anchor, field })}
          onAddFilter={(anchor) => setFilterOpen({ anchor, field: null })}
        />
      )}
      <div ref={scrollRef} className={`lic-scroll lic-scroll--${layout}`} data-layout-current={layout} data-view-key={viewKey}>
        {visible.length === 0 && layout !== 'board' ? (
          <div className="lic-empty">
            {emptyState ?? (
              <div className="lic-empty-inner">
                <div className="t-1 w-510">{activeFilters > 0 ? 'No issues match these filters' : 'No issues'}</div>
                <div className="t-3">
                  {activeFilters > 0 ? (
                    <button type="button" className="lic-link" onClick={() => setFilters({})}>
                      Clear filters
                    </button>
                  ) : (
                    <>
                      Press <Kbd keys={['C']} /> to create an issue.
                    </>
                  )}
                </div>
                {activeFilters === 0 && (
                  <Button size="sm" variant="secondary" icon={<Plus size={14} />} onClick={() => t.openCreate(draftForGroups([], teamId, t))}>
                    Create issue
                  </Button>
                )}
              </div>
            )}
          </div>
        ) : layout === 'list' ? (
          <ListView groups={groups} t={t} />
        ) : layout === 'board' ? (
          <BoardView groups={groups} t={t} />
        ) : (
          <TimelineView groups={groups} t={t} />
        )}
      </div>
      <BulkBar ids={[...selection]} t={t} onClear={() => t.setSelection(new Set())} />
      {Picker && picker && (
        <Picker
          issueIds={picker.ids}
          anchor={picker.anchor}
          open
          onOpenChange={(open) => {
            if (!open) {
              const id = picker.ids[0]
              setPicker(null)
              if (focusedId === id) requestAnimationFrame(() => rowElement(id)?.focus({ preventScroll: true }))
            }
          }}
        />
      )}
      <FilterPopover
        open={filterOpen !== null}
        onOpenChange={(open) => {
          if (!open) setFilterOpen(null)
        }}
        anchor={filterOpen?.anchor ?? filterBtnRef.current}
        initialField={filterOpen?.field ?? null}
        filters={filters}
        onChange={setFilters}
        t={t}
        teamId={teamId}
        showTeam={showTeamTabs || teamId === null}
      />
      <DisplayOptions
        open={displayAnchor !== null}
        onOpenChange={(open) => {
          if (!open) setDisplayAnchor(null)
        }}
        anchor={displayAnchor}
        display={display}
        layout={layout}
        onLayout={setLayout}
        onChange={setDisplay}
      />
      <SaveViewDialog open={saveOpen} onOpenChange={setSaveOpen} filters={filters} display={display} teamId={teamId} t={t} />
      <Dialog
        open={deleteIds !== null}
        onOpenChange={(open) => {
          if (!open) setDeleteIds(null)
        }}
        title={deleteIds && deleteIds.length > 1 ? `Delete ${deleteIds.length} issues?` : 'Delete issue?'}
        width={420}
        footer={
          <>
            <Button variant="ghost" onClick={() => setDeleteIds(null)}>
              Cancel
            </Button>
            <Button variant="danger" data-autofocus onClick={() => deleteIds && void runDelete(deleteIds)}>
              Delete
            </Button>
          </>
        }
      >
        <p className="t-2" style={{ margin: 0 }}>
          This action cannot be undone.
        </p>
      </Dialog>
    </CollectionContext.Provider>
  )
}
