import type {
  Bootstrap,
  Comment,
  Cycle,
  DelegateInput,
  Favorite,
  FavoriteKind,
  ID,
  Issue,
  IssueCreate,
  IssueDetail,
  IssuePatch,
  Label,
  LaneAction,
  Link,
  Milestone,
  Notification,
  Project,
  ProjectDetail,
  ProjectUpdate,
  Relation,
  RelationType,
  Team,
  User,
  View,
  WorkflowState,
} from './types'

export class TrackerApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly code: string | null,
  ) {
    super(message)
  }
}

const BASE = '/api/tracker'

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
  const response = await fetch(`${BASE}${path}`, {
    method,
    headers: body === undefined ? undefined : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  if (response.status === 204) return undefined as T
  const text = await response.text()
  const payload = text ? JSON.parse(text) : null
  if (!response.ok) {
    throw new TrackerApiError(
      payload?.error ?? `${response.status} ${response.statusText}`,
      response.status,
      payload?.code ?? null,
    )
  }
  return payload as T
}

const get = <T>(path: string) => call<T>('GET', path)
const post = <T>(path: string, body: unknown = {}) => call<T>('POST', path, body)
const patch = <T>(path: string, body: unknown) => call<T>('PATCH', path, body)
const del = (path: string) => call<void>('DELETE', path)
const enc = encodeURIComponent

export const trackerApi = {
  bootstrap: () => get<Bootstrap>('/bootstrap'),
  issues: (opts: { team_id?: ID; include_archived?: boolean } = {}) => {
    const q = new URLSearchParams()
    if (opts.team_id) q.set('team_id', opts.team_id)
    if (opts.include_archived) q.set('include_archived', '1')
    const s = q.toString()
    return get<{ items: Issue[] }>(`/issues${s ? `?${s}` : ''}`)
  },
  issue: (idOrIdentifier: string) => get<IssueDetail>(`/issues/${enc(idOrIdentifier)}`),
  createIssue: (input: IssueCreate) => post<Issue>('/issues', input),
  updateIssue: (id: ID, input: IssuePatch) => patch<Issue>(`/issues/${enc(id)}`, input),
  batchUpdate: (ids: ID[], input: IssuePatch) => post<{ items: Issue[] }>('/issues/batch', { ids, patch: input }),
  deleteIssue: (id: ID) => del(`/issues/${enc(id)}`),

  addComment: (issueId: ID, body: string, parentId?: ID | null) =>
    post<Comment>(`/issues/${enc(issueId)}/comments`, { body, parent_id: parentId ?? null }),
  updateComment: (id: ID, input: { body?: string; resolved?: boolean }) => patch<Comment>(`/comments/${enc(id)}`, input),
  deleteComment: (id: ID) => del(`/comments/${enc(id)}`),
  toggleReaction: (commentId: ID, emoji: string) => post<Comment>(`/comments/${enc(commentId)}/reactions`, { emoji }),

  addRelation: (issueId: ID, type: RelationType, relatedIssueId: ID) =>
    post<Relation>(`/issues/${enc(issueId)}/relations`, { type, related_issue_id: relatedIssueId }),
  deleteRelation: (id: ID) => del(`/relations/${enc(id)}`),
  addLink: (issueId: ID, url: string, title?: string) => post<Link>(`/issues/${enc(issueId)}/links`, { url, title }),
  deleteLink: (id: ID) => del(`/links/${enc(id)}`),
  subscribe: (issueId: ID, subscribed: boolean) =>
    post<{ subscribed: boolean }>(`/issues/${enc(issueId)}/subscribe`, { subscribed }),

  delegate: (issueId: ID, input: DelegateInput) => post<IssueDetail>(`/issues/${enc(issueId)}/delegate`, input),
  laneAction: (issueId: ID, action: LaneAction, opts: { text?: string; force?: boolean } = {}) =>
    post<IssueDetail>(`/issues/${enc(issueId)}/lane-action`, { action, ...opts }),

  projects: () => get<{ items: Project[] }>('/projects'),
  project: (id: ID) => get<ProjectDetail>(`/projects/${enc(id)}`),
  createProject: (input: Partial<Project> & { name: string }) => post<Project>('/projects', input),
  updateProject: (id: ID, input: Partial<Project>) => patch<Project>(`/projects/${enc(id)}`, input),
  deleteProject: (id: ID) => del(`/projects/${enc(id)}`),
  addProjectUpdate: (projectId: ID, input: { health: ProjectUpdate['health']; body: string }) =>
    post<ProjectUpdate>(`/projects/${enc(projectId)}/updates`, input),
  updateProjectUpdate: (id: ID, input: Partial<Pick<ProjectUpdate, 'health' | 'body'>>) =>
    patch<ProjectUpdate>(`/project-updates/${enc(id)}`, input),
  deleteProjectUpdate: (id: ID) => del(`/project-updates/${enc(id)}`),
  addMilestone: (projectId: ID, input: { name: string; target_date?: string | null }) =>
    post<Milestone>(`/projects/${enc(projectId)}/milestones`, input),
  updateMilestone: (id: ID, input: Partial<Milestone>) => patch<Milestone>(`/milestones/${enc(id)}`, input),
  deleteMilestone: (id: ID) => del(`/milestones/${enc(id)}`),

  createCycle: (input: { team_id: ID; starts_at: string; ends_at: string; name?: string | null }) =>
    post<Cycle>('/cycles', input),
  updateCycle: (id: ID, input: Partial<Cycle>) => patch<Cycle>(`/cycles/${enc(id)}`, input),
  deleteCycle: (id: ID) => del(`/cycles/${enc(id)}`),

  createLabel: (input: { name: string; color: string; team_id?: ID | null; parent_id?: ID | null }) =>
    post<Label>('/labels', input),
  updateLabel: (id: ID, input: Partial<Label>) => patch<Label>(`/labels/${enc(id)}`, input),
  deleteLabel: (id: ID) => del(`/labels/${enc(id)}`),

  createTeam: (input: { key: string; name: string; icon?: string; color?: string }) => post<Team>('/teams', input),
  updateTeam: (id: ID, input: Partial<Team>) => patch<Team>(`/teams/${enc(id)}`, input),
  createState: (input: Omit<WorkflowState, 'id' | 'lane_status' | 'description' | 'position'> & { position?: number }) =>
    post<WorkflowState>('/states', input),
  updateState: (id: ID, input: Partial<WorkflowState>) => patch<WorkflowState>(`/states/${enc(id)}`, input),
  deleteState: (id: ID) => del(`/states/${enc(id)}`),

  createUser: (input: { name: string; display_name?: string; email?: string; avatar_color?: string }) =>
    post<User>('/users', input),
  updateUser: (id: ID, input: Partial<User>) => patch<User>(`/users/${enc(id)}`, input),

  createView: (input: Omit<View, 'id' | 'owner_id' | 'created_at' | 'updated_at' | 'shared' | 'description' | 'icon' | 'color'> &
    Partial<Pick<View, 'description' | 'icon' | 'color' | 'shared'>>) => post<View>('/views', input),
  updateView: (id: ID, input: Partial<View>) => patch<View>(`/views/${enc(id)}`, input),
  deleteView: (id: ID) => del(`/views/${enc(id)}`),

  toggleFavorite: (kind: FavoriteKind, targetId: ID) =>
    post<{ favorited: boolean; favorite: Favorite | null }>('/favorites', { kind, target_id: targetId }),

  notifications: () => get<{ items: Notification[] }>('/notifications'),
  updateNotification: (id: ID, input: { read?: boolean; snoozed_until?: string | null; archived?: boolean }) =>
    patch<Notification>(`/notifications/${enc(id)}`, input),
  readAllNotifications: () => post<{ updated: number }>('/notifications/read-all'),

  search: (q: string) => get<{ issues: Issue[]; projects: Project[] }>(`/search?q=${enc(q)}`),
}
