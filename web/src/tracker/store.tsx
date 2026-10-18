import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react'
import { TrackerApiError, trackerApi } from './api'
import { cycleStatus, dueState, isClosedType, parseDateOnly, toDateOnly } from './format'
import type {
  Bootstrap,
  Cycle,
  Favorite,
  FavoriteKind,
  Grouping,
  ID,
  Issue,
  IssueCreate,
  IssuePatch,
  Label,
  Milestone,
  Notification,
  Ordering,
  Priority,
  Project,
  Team,
  User,
  View,
  ViewDisplay,
  ViewFilters,
  WorkflowState,
} from './types'
import { DEFAULT_DISPLAY, PRIORITY_LABELS, STATE_TYPE_ORDER } from './types'

export type Theme = 'dark' | 'light'

export interface CreateDraft extends Partial<IssueCreate> {
  team_id?: ID
}

export interface TrackerState {
  ready: boolean
  error: string | null
  me: User | null
  users: User[]
  teams: Team[]
  states: WorkflowState[]
  labels: Label[]
  projects: Project[]
  cycles: Cycle[]
  milestones: Milestone[]
  views: View[]
  favorites: Favorite[]
  issues: Issue[]
  notifications: Notification[]
  unreadCount: number

  usersById: Map<ID, User>
  teamsById: Map<ID, Team>
  teamsByKey: Map<string, Team>
  statesById: Map<ID, WorkflowState>
  labelsById: Map<ID, Label>
  projectsById: Map<ID, Project>
  cyclesById: Map<ID, Cycle>
  issuesById: Map<ID, Issue>
  issuesByIdentifier: Map<string, Issue>
  teamStates: (teamId: ID) => WorkflowState[]

  theme: Theme
  setTheme: (theme: Theme) => void
  sidebarCollapsed: boolean
  setSidebarCollapsed: (value: boolean) => void
  mobileNavOpen: boolean
  setMobileNavOpen: (value: boolean) => void

  peekIssueId: ID | null
  setPeekIssueId: (id: ID | null) => void
  createDraft: CreateDraft | null
  openCreate: (draft?: CreateDraft) => void
  closeCreate: () => void
  paletteOpen: boolean
  setPaletteOpen: (value: boolean) => void
  shortcutsOpen: boolean
  setShortcutsOpen: (value: boolean) => void

  selection: Set<ID>
  setSelection: (ids: Set<ID>) => void
  focusedIssueId: ID | null
  setFocusedIssueId: (id: ID | null) => void

  displayFor: (viewKey: string, fallback?: Partial<ViewDisplay>) => ViewDisplay
  setDisplayFor: (viewKey: string, display: ViewDisplay) => void
  filtersFor: (viewKey: string) => ViewFilters
  setFiltersFor: (viewKey: string, filters: ViewFilters) => void

  refresh: () => Promise<void>
  refreshIssues: () => Promise<void>
  refreshNotifications: () => Promise<void>
  createIssue: (input: IssueCreate) => Promise<Issue>
  updateIssues: (ids: ID[], patch: IssuePatch) => Promise<Issue[]>
  deleteIssue: (id: ID) => Promise<void>
  toggleFavorite: (kind: FavoriteKind, targetId: ID) => Promise<void>
  isFavorite: (kind: FavoriteKind, targetId: ID) => boolean
  upsertIssue: (issue: Issue) => void
  upsertProject: (project: Project) => void
  removeProject: (id: ID) => void
  upsertView: (view: View) => void
  removeView: (id: ID) => void
  upsertCycle: (cycle: Cycle) => void
  removeCycle: (id: ID) => void
  upsertLabel: (label: Label) => void
  removeLabel: (id: ID) => void
  upsertTeam: (team: Team) => void
  upsertState: (state: WorkflowState) => void
  removeState: (id: ID) => void
  upsertUser: (user: User) => void
  upsertNotification: (notification: Notification) => void
  reportError: (error: unknown) => void
  lastError: { message: string; at: number } | null
}

const TrackerContext = createContext<TrackerState | null>(null)

function indexBy<T, K>(items: T[], key: (item: T) => K): Map<K, T> {
  const map = new Map<K, T>()
  for (const item of items) map.set(key(item), item)
  return map
}

function upsert<T extends { id: ID }>(items: T[], next: T): T[] {
  const index = items.findIndex((item) => item.id === next.id)
  if (index < 0) return [...items, next]
  const copy = items.slice()
  copy[index] = next
  return copy
}

function readStorage<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    return raw ? (JSON.parse(raw) as T) : fallback
  } catch {
    return fallback
  }
}

function writeStorage(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    // Storage may be unavailable (private mode); preferences then last for the session only.
  }
}

function optimisticPatch(issue: Issue, patch: IssuePatch): Issue {
  const next: Issue = { ...issue }
  const keys: (keyof IssuePatch & keyof Issue)[] = [
    'title',
    'description',
    'team_id',
    'state_id',
    'priority',
    'estimate',
    'assignee_id',
    'project_id',
    'milestone_id',
    'cycle_id',
    'parent_id',
    'due_date',
    'sort_order',
    'snoozed_until',
  ]
  for (const key of keys) {
    if (patch[key] !== undefined) (next as unknown as Record<string, unknown>)[key] = patch[key]
  }
  if (patch.label_ids) next.label_ids = patch.label_ids
  if (patch.add_label_ids) next.label_ids = Array.from(new Set([...next.label_ids, ...patch.add_label_ids]))
  if (patch.remove_label_ids) next.label_ids = next.label_ids.filter((id) => !patch.remove_label_ids!.includes(id))
  if (patch.archived !== undefined) next.archived_at = patch.archived ? new Date().toISOString() : null
  next.updated_at = new Date().toISOString()
  return next
}

export function TrackerProvider({ children }: { children: React.ReactNode }) {
  const [data, setData] = useState<Bootstrap | null>(null)
  const [issues, setIssues] = useState<Issue[]>([])
  const [notifications, setNotifications] = useState<Notification[]>([])
  const [error, setError] = useState<string | null>(null)
  const [lastError, setLastError] = useState<{ message: string; at: number } | null>(null)
  const [theme, setThemeState] = useState<Theme>(() => readStorage<Theme>('tracker.theme', 'dark'))
  const [sidebarCollapsed, setSidebarCollapsedState] = useState<boolean>(() => readStorage('tracker.sidebarCollapsed', false))
  const [mobileNavOpen, setMobileNavOpen] = useState(false)
  const [peekIssueId, setPeekIssueId] = useState<ID | null>(null)
  const [createDraft, setCreateDraft] = useState<CreateDraft | null>(null)
  const [paletteOpen, setPaletteOpen] = useState(false)
  const [shortcutsOpen, setShortcutsOpen] = useState(false)
  const [selection, setSelection] = useState<Set<ID>>(new Set())
  const [focusedIssueId, setFocusedIssueId] = useState<ID | null>(null)
  const [displays, setDisplays] = useState<Record<string, ViewDisplay>>(() => readStorage('tracker.displays', {}))
  const [filters, setFilters] = useState<Record<string, ViewFilters>>(() => readStorage('tracker.filters', {}))
  const loading = useRef(false)

  const reportError = useCallback((err: unknown) => {
    const message = err instanceof TrackerApiError || err instanceof Error ? err.message : String(err)
    setLastError({ message, at: Date.now() })
  }, [])

  const refreshIssues = useCallback(async () => {
    const result = await trackerApi.issues({ include_archived: true })
    setIssues(result.items)
  }, [])

  const refreshNotifications = useCallback(async () => {
    const result = await trackerApi.notifications()
    setNotifications(result.items)
  }, [])

  const refresh = useCallback(async () => {
    if (loading.current) return
    loading.current = true
    try {
      const [boot, list, notes] = await Promise.all([
        trackerApi.bootstrap(),
        trackerApi.issues({ include_archived: true }),
        trackerApi.notifications(),
      ])
      setData(boot)
      setIssues(list.items)
      setNotifications(notes.items)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      loading.current = false
    }
  }, [])

  useEffect(() => {
    void refresh()
    const interval = window.setInterval(() => {
      if (document.visibilityState === 'visible') void refresh()
    }, 10_000)
    return () => window.clearInterval(interval)
  }, [refresh])

  const setTheme = useCallback((next: Theme) => {
    setThemeState(next)
    writeStorage('tracker.theme', next)
  }, [])

  const setSidebarCollapsed = useCallback((value: boolean) => {
    setSidebarCollapsedState(value)
    writeStorage('tracker.sidebarCollapsed', value)
  }, [])

  const users = data?.users ?? []
  const teams = useMemo(() => (data?.teams ?? []).slice().sort((a, b) => a.sort_order - b.sort_order), [data?.teams])
  const states = useMemo(() => (data?.states ?? []).slice().sort((a, b) => a.position - b.position), [data?.states])
  const labels = data?.labels ?? []
  const projects = data?.projects ?? []
  const cycles = data?.cycles ?? []
  const views = data?.views ?? []
  const favorites = data?.favorites ?? []

  const usersById = useMemo(() => indexBy(users, (u) => u.id), [users])
  const teamsById = useMemo(() => indexBy(teams, (t) => t.id), [teams])
  const teamsByKey = useMemo(() => indexBy(teams, (t) => t.key), [teams])
  const statesById = useMemo(() => indexBy(states, (s) => s.id), [states])
  const labelsById = useMemo(() => indexBy(labels, (l) => l.id), [labels])
  const projectsById = useMemo(() => indexBy(projects, (p) => p.id), [projects])
  const cyclesById = useMemo(() => indexBy(cycles, (c) => c.id), [cycles])
  const issuesById = useMemo(() => indexBy(issues, (i) => i.id), [issues])
  const issuesByIdentifier = useMemo(() => indexBy(issues, (i) => i.identifier), [issues])
  const teamStates = useCallback(
    (teamId: ID) =>
      states
        .filter((s) => s.team_id === teamId)
        .sort((a, b) => STATE_TYPE_ORDER.indexOf(a.type) - STATE_TYPE_ORDER.indexOf(b.type) || a.position - b.position),
    [states],
  )

  const patchData = useCallback((fn: (current: Bootstrap) => Bootstrap) => {
    setData((current) => (current ? fn(current) : current))
  }, [])

  const upsertIssue = useCallback((issue: Issue) => setIssues((current) => upsert(current, issue)), [])

  const createIssue = useCallback(
    async (input: IssueCreate) => {
      const issue = await trackerApi.createIssue(input)
      setIssues((current) => upsert(current, issue))
      patchData((d) => ({
        ...d,
        teams: d.teams.map((t) => (t.id === issue.team_id ? { ...t, issue_seq: Math.max(t.issue_seq, issue.number) } : t)),
      }))
      return issue
    },
    [patchData],
  )

  const updateIssues = useCallback(
    async (ids: ID[], patch: IssuePatch) => {
      if (ids.length === 0) return []
      const previous = new Map<ID, Issue>()
      setIssues((current) =>
        current.map((issue) => {
          if (!ids.includes(issue.id)) return issue
          previous.set(issue.id, issue)
          return optimisticPatch(issue, patch)
        }),
      )
      try {
        const updated = ids.length === 1 ? [await trackerApi.updateIssue(ids[0], patch)] : (await trackerApi.batchUpdate(ids, patch)).items
        setIssues((current) => updated.reduce((acc, issue) => upsert(acc, issue), current))
        return updated
      } catch (err) {
        setIssues((current) => current.map((issue) => previous.get(issue.id) ?? issue))
        reportError(err)
        throw err
      }
    },
    [reportError],
  )

  const deleteIssue = useCallback(
    async (id: ID) => {
      await trackerApi.deleteIssue(id)
      setIssues((current) => current.filter((issue) => issue.id !== id))
    },
    [],
  )

  const toggleFavorite = useCallback(
    async (kind: FavoriteKind, targetId: ID) => {
      const result = await trackerApi.toggleFavorite(kind, targetId)
      patchData((d) => ({
        ...d,
        favorites: result.favorited && result.favorite
          ? upsert(d.favorites, result.favorite)
          : d.favorites.filter((f) => !(f.kind === kind && f.target_id === targetId)),
      }))
    },
    [patchData],
  )

  const isFavorite = useCallback(
    (kind: FavoriteKind, targetId: ID) => favorites.some((f) => f.kind === kind && f.target_id === targetId),
    [favorites],
  )

  const displayFor = useCallback(
    (viewKey: string, fallback?: Partial<ViewDisplay>) => displays[viewKey] ?? { ...DEFAULT_DISPLAY, ...fallback },
    [displays],
  )
  const setDisplayFor = useCallback((viewKey: string, display: ViewDisplay) => {
    setDisplays((current) => {
      const next = { ...current, [viewKey]: display }
      writeStorage('tracker.displays', next)
      return next
    })
  }, [])
  const filtersFor = useCallback((viewKey: string) => filters[viewKey] ?? {}, [filters])
  const setFiltersFor = useCallback((viewKey: string, value: ViewFilters) => {
    setFilters((current) => {
      const next = { ...current, [viewKey]: value }
      writeStorage('tracker.filters', next)
      return next
    })
  }, [])

  const state: TrackerState = {
    ready: data !== null,
    error,
    me: data?.me ?? null,
    users,
    teams,
    states,
    labels,
    projects,
    cycles,
    milestones: data?.milestones ?? [],
    views,
    favorites,
    issues,
    notifications,
    unreadCount: notifications.filter((n) => !n.read_at && !n.archived_at).length,
    usersById,
    teamsById,
    teamsByKey,
    statesById,
    labelsById,
    projectsById,
    cyclesById,
    issuesById,
    issuesByIdentifier,
    teamStates,
    theme,
    setTheme,
    sidebarCollapsed,
    setSidebarCollapsed,
    mobileNavOpen,
    setMobileNavOpen,
    peekIssueId,
    setPeekIssueId,
    createDraft,
    openCreate: (draft) => setCreateDraft(draft ?? {}),
    closeCreate: () => setCreateDraft(null),
    paletteOpen,
    setPaletteOpen,
    shortcutsOpen,
    setShortcutsOpen,
    selection,
    setSelection,
    focusedIssueId,
    setFocusedIssueId,
    displayFor,
    setDisplayFor,
    filtersFor,
    setFiltersFor,
    refresh,
    refreshIssues,
    refreshNotifications,
    createIssue,
    updateIssues,
    deleteIssue,
    toggleFavorite,
    isFavorite,
    upsertIssue,
    upsertProject: (project) => patchData((d) => ({ ...d, projects: upsert(d.projects, project) })),
    removeProject: (id) => patchData((d) => ({ ...d, projects: d.projects.filter((p) => p.id !== id) })),
    upsertView: (view) => patchData((d) => ({ ...d, views: upsert(d.views, view) })),
    removeView: (id) => patchData((d) => ({ ...d, views: d.views.filter((v) => v.id !== id) })),
    upsertCycle: (cycle) => patchData((d) => ({ ...d, cycles: upsert(d.cycles, cycle) })),
    removeCycle: (id) => patchData((d) => ({ ...d, cycles: d.cycles.filter((c) => c.id !== id) })),
    upsertLabel: (label) => patchData((d) => ({ ...d, labels: upsert(d.labels, label) })),
    removeLabel: (id) => patchData((d) => ({ ...d, labels: d.labels.filter((l) => l.id !== id) })),
    upsertTeam: (team) => patchData((d) => ({ ...d, teams: upsert(d.teams, team) })),
    upsertState: (s) => patchData((d) => ({ ...d, states: upsert(d.states, s) })),
    removeState: (id) => patchData((d) => ({ ...d, states: d.states.filter((s) => s.id !== id) })),
    upsertUser: (user) => patchData((d) => ({ ...d, users: upsert(d.users, user) })),
    upsertNotification: (n) => setNotifications((current) => upsert(current, n)),
    reportError,
    lastError,
  }

  return <TrackerContext.Provider value={state}>{children}</TrackerContext.Provider>
}

export function useTracker(): TrackerState {
  const value = useContext(TrackerContext)
  if (!value) throw new Error('useTracker must be used inside TrackerProvider')
  return value
}

export function applyFilters(issues: Issue[], filters: ViewFilters, t: Pick<TrackerState, 'statesById' | 'me' | 'cyclesById'>): Issue[] {
  const today = toDateOnly(new Date())
  const weekAhead = toDateOnly(new Date(Date.now() + 7 * 86_400_000))
  const text = filters.text?.trim().toLowerCase()
  return issues.filter((issue) => {
    const state = t.statesById.get(issue.state_id)
    if (filters.state_ids?.length && !filters.state_ids.includes(issue.state_id)) return false
    if (filters.state_types?.length && (!state || !filters.state_types.includes(state.type))) return false
    if (filters.priorities?.length && !filters.priorities.includes(issue.priority)) return false
    if (filters.assignee_ids?.length) {
      const ok = filters.assignee_ids.some((a) =>
        a === 'none' ? issue.assignee_id === null : a === 'me' ? issue.assignee_id === t.me?.id : issue.assignee_id === a,
      )
      if (!ok) return false
    }
    if (filters.creator_ids?.length && !filters.creator_ids.includes(issue.creator_id)) return false
    if (filters.label_ids?.length && !filters.label_ids.some((l) => issue.label_ids.includes(l))) return false
    if (filters.project_ids?.length) {
      const ok = filters.project_ids.some((p) => (p === 'none' ? issue.project_id === null : issue.project_id === p))
      if (!ok) return false
    }
    if (filters.cycle_ids?.length) {
      const ok = filters.cycle_ids.some((c) => {
        if (c === 'none') return issue.cycle_id === null
        if (c === 'current') {
          const cycle = issue.cycle_id ? t.cyclesById.get(issue.cycle_id) : undefined
          return cycle ? cycleStatus(cycle) === 'current' : false
        }
        return issue.cycle_id === c
      })
      if (!ok) return false
    }
    if (filters.team_ids?.length && !filters.team_ids.includes(issue.team_id)) return false
    if (filters.has_lane !== undefined && (issue.lane_key !== null) !== filters.has_lane) return false
    if (filters.lane_statuses?.length && (!issue.lane || !filters.lane_statuses.includes(issue.lane.status))) return false
    if (filters.due) {
      const due = issue.due_date
      if (filters.due === 'none' && due) return false
      if (filters.due === 'overdue' && !(due && dueState(due, state ? isClosedType(state.type) : false) === 'overdue')) return false
      if (filters.due === 'today' && due !== today) return false
      if (filters.due === 'week' && !(due && due >= today && due <= weekAhead)) return false
    }
    if (text && !`${issue.identifier} ${issue.title}`.toLowerCase().includes(text)) return false
    return true
  })
}

export function filterCount(filters: ViewFilters): number {
  return Object.entries(filters).filter(([key, value]) => {
    if (key === 'text') return Boolean(value)
    if (Array.isArray(value)) return value.length > 0
    return value !== undefined && value !== null
  }).length
}

export function applyCompletedWindow(issues: Issue[], display: ViewDisplay, statesById: Map<ID, WorkflowState>): Issue[] {
  if (display.show_completed === 'all') return issues
  const limit =
    display.show_completed === 'week' ? Date.now() - 7 * 86_400_000 : display.show_completed === 'month' ? Date.now() - 30 * 86_400_000 : Infinity
  return issues.filter((issue) => {
    const state = statesById.get(issue.state_id)
    if (!state || !isClosedType(state.type)) return true
    if (display.show_completed === 'none') return false
    const closedAt = issue.completed_at ?? issue.canceled_at ?? issue.updated_at
    return new Date(closedAt).getTime() >= limit
  })
}

export function sortIssues(issues: Issue[], ordering: Ordering, statesById: Map<ID, WorkflowState>): Issue[] {
  const copy = issues.slice()
  const priorityRank = (p: Priority) => (p === 0 ? 5 : p)
  const stateRank = (issue: Issue) => {
    const state = statesById.get(issue.state_id)
    return state ? STATE_TYPE_ORDER.indexOf(state.type) * 1000 + state.position : 99999
  }
  copy.sort((a, b) => {
    switch (ordering) {
      case 'manual':
        return a.sort_order - b.sort_order
      case 'priority':
        return priorityRank(a.priority) - priorityRank(b.priority) || b.updated_at.localeCompare(a.updated_at)
      case 'updated':
        return b.updated_at.localeCompare(a.updated_at)
      case 'created':
        return b.created_at.localeCompare(a.created_at)
      case 'due':
        return (a.due_date ?? '9999').localeCompare(b.due_date ?? '9999')
      case 'title':
        return a.title.localeCompare(b.title)
      case 'status':
        return stateRank(a) - stateRank(b)
    }
  })
  return copy
}

export interface IssueGroup {
  key: string
  label: string
  grouping: Grouping
  value: ID | Priority | null
  issues: Issue[]
}

export function groupIssues(
  issues: Issue[],
  grouping: Grouping,
  t: Pick<TrackerState, 'statesById' | 'usersById' | 'projectsById' | 'cyclesById' | 'labelsById' | 'teamsById' | 'teamStates'>,
  opts: { teamId?: ID | null; showEmpty?: boolean } = {},
): IssueGroup[] {
  if (grouping === 'none') return [{ key: 'all', label: 'All issues', grouping, value: null, issues }]
  const buckets = new Map<string, IssueGroup>()
  const ensure = (key: string, label: string, value: IssueGroup['value']) => {
    let group = buckets.get(key)
    if (!group) {
      group = { key, label, grouping, value, issues: [] }
      buckets.set(key, group)
    }
    return group
  }
  if (grouping === 'status' && opts.teamId && opts.showEmpty) {
    for (const state of t.teamStates(opts.teamId)) ensure(state.id, state.name, state.id)
  }
  if (grouping === 'priority' && opts.showEmpty) {
    for (const p of [1, 2, 3, 4, 0] as Priority[]) ensure(`p${p}`, PRIORITY_LABELS[p], p)
  }
  for (const issue of issues) {
    switch (grouping) {
      case 'status': {
        const state = t.statesById.get(issue.state_id)
        ensure(issue.state_id, state?.name ?? 'Unknown', issue.state_id).issues.push(issue)
        break
      }
      case 'assignee': {
        const user = issue.assignee_id ? t.usersById.get(issue.assignee_id) : undefined
        ensure(issue.assignee_id ?? 'none', user?.name ?? 'No assignee', issue.assignee_id).issues.push(issue)
        break
      }
      case 'project': {
        const project = issue.project_id ? t.projectsById.get(issue.project_id) : undefined
        ensure(issue.project_id ?? 'none', project?.name ?? 'No project', issue.project_id).issues.push(issue)
        break
      }
      case 'priority':
        ensure(`p${issue.priority}`, PRIORITY_LABELS[issue.priority], issue.priority).issues.push(issue)
        break
      case 'cycle': {
        const cycle = issue.cycle_id ? t.cyclesById.get(issue.cycle_id) : undefined
        ensure(issue.cycle_id ?? 'none', cycle ? cycle.name || `Cycle ${cycle.number}` : 'No cycle', issue.cycle_id).issues.push(issue)
        break
      }
      case 'label': {
        if (issue.label_ids.length === 0) ensure('none', 'No labels', null).issues.push(issue)
        for (const id of issue.label_ids) ensure(id, t.labelsById.get(id)?.name ?? 'Label', id).issues.push(issue)
        break
      }
      case 'team': {
        const team = t.teamsById.get(issue.team_id)
        ensure(issue.team_id, team?.name ?? 'Team', issue.team_id).issues.push(issue)
        break
      }
    }
  }
  const groups = [...buckets.values()]
  const order = (g: IssueGroup): number => {
    switch (grouping) {
      case 'status': {
        const s = typeof g.value === 'string' ? t.statesById.get(g.value) : undefined
        return s ? STATE_TYPE_ORDER.indexOf(s.type) * 1000 + s.position : 99999
      }
      case 'priority':
        return g.value === 0 ? 5 : Number(g.value)
      case 'cycle': {
        const c = typeof g.value === 'string' ? t.cyclesById.get(g.value) : undefined
        return c ? -c.number : 99999
      }
      default:
        return g.value === null ? 99999 : 0
    }
  }
  groups.sort((a, b) => order(a) - order(b) || a.label.localeCompare(b.label))
  return opts.showEmpty ? groups : groups.filter((g) => g.issues.length > 0)
}

export function patchForGroup(group: IssueGroup): IssuePatch | null {
  switch (group.grouping) {
    case 'status':
      return typeof group.value === 'string' ? { state_id: group.value } : null
    case 'assignee':
      return { assignee_id: (group.value as ID | null) ?? null }
    case 'project':
      return { project_id: (group.value as ID | null) ?? null }
    case 'priority':
      return { priority: (group.value as Priority) ?? 0 }
    case 'cycle':
      return { cycle_id: (group.value as ID | null) ?? null }
    case 'label':
      return group.value ? { add_label_ids: [group.value as ID] } : null
    default:
      return null
  }
}

export function sortOrderBetween(before: number | null, after: number | null): number {
  if (before === null && after === null) return Date.now()
  if (before === null) return (after as number) - 1024
  if (after === null) return before + 1024
  return (before + after) / 2
}

export function teamDefaultState(teamId: ID, teamStates: (teamId: ID) => WorkflowState[], prefer: 'backlog' | 'unstarted' | 'triage' = 'unstarted'): WorkflowState | undefined {
  const states = teamStates(teamId)
  return states.find((s) => s.type === prefer) ?? states.find((s) => s.type === 'backlog') ?? states[0]
}

export function issueIsOverdue(issue: Issue, statesById: Map<ID, WorkflowState>): boolean {
  const state = statesById.get(issue.state_id)
  return dueState(issue.due_date, state ? isClosedType(state.type) : false) === 'overdue'
}

export { parseDateOnly }
