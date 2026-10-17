import { createContext, useContext, useMemo } from 'react'
import { TrackerApiError } from '../api'
import { isClosedType } from '../format'
import {
  applyCompletedWindow,
  applyFilters,
  groupIssues,
  patchForGroup,
  sortIssues,
  type IssueGroup,
  type TrackerState,
} from '../store'
import type { ID, Issue, IssueCreate, IssuePatch, StateType, ViewDisplay, ViewFilters } from '../types'
import { toast } from '../ui'

/** Layouts rendered by IssueCollection. `timeline` is kept outside ViewDisplay (whose union is list|board). */
export type CollectionLayout = 'list' | 'board' | 'timeline'

export type PickerKind = 'status' | 'priority' | 'assignee' | 'labels' | 'project' | 'cycle' | 'estimate' | 'due'

export interface PickerRequest {
  kind: PickerKind
  ids: ID[]
  anchor: HTMLElement | DOMRect | null
}

export interface CollectionContextValue {
  viewKey: string
  display: ViewDisplay
  teamId: ID | null
  focusedId: ID | null
  selection: Set<ID>
  collapsed: Set<string>
  toggleCollapsed: (key: string) => void
  openPicker: (kind: PickerKind, issue: Issue, anchor: HTMLElement | DOMRect | null) => void
  onRowClick: (issue: Issue, event: React.MouseEvent) => void
  onRowHover: (issue: Issue) => void
  toggleSelected: (issue: Issue) => void
  createInGroup: (groups: IssueGroup[]) => void
  contextAction: (issue: Issue, actionId: string) => void
}

export const CollectionContext = createContext<CollectionContextValue | null>(null)

export function useCollection(): CollectionContextValue {
  const value = useContext(CollectionContext)
  if (!value) throw new Error('useCollection must be used inside IssueCollection')
  return value
}

export function pickId(value: unknown): string {
  if (typeof value === 'string') return value
  if (value && typeof value === 'object' && 'id' in value) return String((value as { id: unknown }).id)
  return String(value)
}

export function readLocal<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    return raw ? (JSON.parse(raw) as T) : fallback
  } catch {
    return fallback
  }
}

export function writeLocal(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    // Storage unavailable: the preference lasts for this session only.
  }
}

export interface CollectionModel {
  visible: Issue[]
  groups: IssueGroup[]
}

export function useCollectionModel(
  issues: Issue[],
  filters: ViewFilters,
  display: ViewDisplay,
  layout: CollectionLayout,
  teamId: ID | null,
  stateTypes: StateType[] | undefined,
  t: TrackerState,
): CollectionModel {
  return useMemo(() => {
    let list = applyFilters(issues, filters, t)
    list = applyCompletedWindow(list, display, t.statesById)
    if (!display.show_sub_issues) list = list.filter((issue) => !issue.parent_id)
    list = sortIssues(list, display.ordering, t.statesById)
    const grouping = layout === 'board' && display.grouping === 'none' ? 'status' : display.grouping
    const showEmpty = display.show_empty_groups || layout === 'board'
    let groups = groupIssues(list, grouping, t, { teamId, showEmpty })
    if (grouping === 'status') {
      groups = groups.filter((group) => {
        if (group.issues.length > 0) return true
        const state = typeof group.value === 'string' ? t.statesById.get(group.value) : undefined
        if (!state) return false
        if (stateTypes && !stateTypes.includes(state.type)) return false
        if (display.show_completed === 'none' && isClosedType(state.type)) return false
        return true
      })
    }
    return { visible: list, groups }
  }, [issues, filters, display, layout, teamId, stateTypes, t])
}

/** Patch moving an issue from one group to another (labels swap instead of only adding). */
export function patchForMove(from: IssueGroup | undefined, to: IssueGroup): IssuePatch | null {
  const base = patchForGroup(to)
  if (to.grouping === 'label') {
    const patch: IssuePatch = {}
    if (to.value) patch.add_label_ids = [to.value as ID]
    if (from && from.grouping === 'label' && from.value && from.value !== to.value) patch.remove_label_ids = [from.value as ID]
    return patch.add_label_ids || patch.remove_label_ids ? patch : null
  }
  return base
}

/** When grouping by status across teams, a column's state belongs to one team; map it onto the issue's team. */
export function mapPatchToIssueTeam(patch: IssuePatch, issue: Issue, t: TrackerState): IssuePatch {
  if (!patch.state_id) return patch
  const target = t.statesById.get(patch.state_id)
  if (!target || target.team_id === issue.team_id) return patch
  const states = t.teamStates(issue.team_id)
  const match = states.find((s) => s.name === target.name) ?? states.find((s) => s.type === target.type)
  return match ? { ...patch, state_id: match.id } : withoutState(patch)
}

function withoutState(patch: IssuePatch): IssuePatch {
  const copy = { ...patch }
  delete copy.state_id
  return copy
}

export function draftForGroups(groups: IssueGroup[], teamId: ID | null, t: TrackerState): Partial<IssueCreate> {
  const draft: Partial<IssueCreate> = {}
  for (const group of groups) {
    const patch = patchForGroup(group)
    if (!patch) continue
    if (patch.state_id) {
      draft.state_id = patch.state_id
      const state = t.statesById.get(patch.state_id)
      if (state) draft.team_id = state.team_id
    }
    if (patch.priority !== undefined) draft.priority = patch.priority
    if (patch.assignee_id !== undefined) draft.assignee_id = patch.assignee_id
    if (patch.project_id !== undefined) draft.project_id = patch.project_id
    if (patch.cycle_id !== undefined) draft.cycle_id = patch.cycle_id
    if (patch.add_label_ids) draft.label_ids = [...(draft.label_ids ?? []), ...patch.add_label_ids]
  }
  if (!draft.team_id) {
    draft.team_id = teamId ?? t.teams.find((team) => !team.system)?.id ?? t.teams[0]?.id
  }
  return draft
}

export function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}

/**
 * Applies a patch and surfaces lane rejections (409) as toasts. Lane-linked issues that are not
 * verified can be force-closed from the toast action.
 */
export async function updateWithLaneToast(t: TrackerState, ids: ID[], patch: IssuePatch): Promise<boolean> {
  try {
    await t.updateIssues(ids, patch)
    return true
  } catch (err) {
    if (err instanceof TrackerApiError && err.code === 'lane_not_verified') {
      toast.error(err.message, {
        action: {
          label: 'Force close',
          onClick: () => {
            t.updateIssues(ids, { ...patch, force_note: 'closed from tracker board' }).catch((e) => toast.error(errorMessage(e)))
          },
        },
      })
    } else if (!(err instanceof TrackerApiError)) {
      toast.error(errorMessage(err))
    }
    return false
  }
}
