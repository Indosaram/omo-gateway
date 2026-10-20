export type ID = string

export type StateType = 'triage' | 'backlog' | 'unstarted' | 'started' | 'completed' | 'canceled'
export type Priority = 0 | 1 | 2 | 3 | 4
export type LaneStatus =
  | 'queued'
  | 'working'
  | 'blocked'
  | 'done_claimed'
  | 'not_done'
  | 'verified_done'
  | 'closed'
  | 'failed'

export interface User {
  id: ID
  name: string
  display_name: string
  email: string | null
  avatar_color: string
  kind: 'human' | 'agent'
  model: string | null
  is_me: boolean
  created_at: string
}

export interface Team {
  id: ID
  key: string
  name: string
  icon: string
  color: string
  issue_seq: number
  system: boolean
  cycles_enabled: boolean
  cycle_weeks: number
  triage_enabled: boolean
  sort_order: number
  created_at: string
}

export interface WorkflowState {
  id: ID
  team_id: ID
  name: string
  type: StateType
  color: string
  position: number
  lane_status: LaneStatus | null
  /** Set on SES team states: the omo session state this column mirrors. */
  session_state: string | null
  description: string | null
}

export interface Label {
  id: ID
  team_id: ID | null
  name: string
  color: string
  parent_id: ID | null
  description: string | null
  created_at: string
}

export type ProjectStatus = 'backlog' | 'planned' | 'started' | 'paused' | 'completed' | 'canceled'

export interface Project {
  id: ID
  name: string
  description: string
  summary: string
  icon: string
  color: string
  status: ProjectStatus
  priority: Priority
  lead_id: ID | null
  start_date: string | null
  target_date: string | null
  sort_order: number
  created_at: string
  updated_at: string
  archived_at: string | null
  team_ids: ID[]
  member_ids: ID[]
  issue_count: number
  completed_issue_count: number
  latest_health: ProjectHealth | null
}

export type ProjectHealth = 'on_track' | 'at_risk' | 'off_track'

export interface ProjectUpdate {
  id: ID
  project_id: ID
  user_id: ID
  health: ProjectHealth
  body: string
  created_at: string
  updated_at: string
}

export interface Milestone {
  id: ID
  project_id: ID
  name: string
  target_date: string | null
  sort_order: number
}

export interface Cycle {
  id: ID
  team_id: ID
  number: number
  name: string | null
  description: string | null
  starts_at: string
  ends_at: string
  completed_at: string | null
}

export interface LaneSummary {
  key: string
  status: LaneStatus
  model: string | null
  turns_run: number
  last_activity_at: string
  thread_channel_id: string | null
}

export interface Issue {
  id: ID
  team_id: ID
  number: number
  identifier: string
  title: string
  description: string
  state_id: ID
  priority: Priority
  estimate: number | null
  assignee_id: ID | null
  creator_id: ID
  project_id: ID | null
  milestone_id: ID | null
  cycle_id: ID | null
  parent_id: ID | null
  due_date: string | null
  sort_order: number
  lane_key: string | null
  created_at: string
  updated_at: string
  started_at: string | null
  completed_at: string | null
  canceled_at: string | null
  archived_at: string | null
  snoozed_until: string | null
  label_ids: ID[]
  /** Users subscribed to this issue; the server sends it on every issue payload. */
  subscriber_ids: ID[]
  sub_issue_count: number
  sub_issue_completed: number
  comment_count: number
  lane: LaneSummary | null
  /** "<machine>:<session_id>" for issues mirrored from an omo session. */
  session_ref: string | null
  session: SessionSummary | null
}

export interface IssueCreate {
  team_id: ID
  title: string
  description?: string
  state_id?: ID
  priority?: Priority
  estimate?: number | null
  assignee_id?: ID | null
  project_id?: ID | null
  milestone_id?: ID | null
  cycle_id?: ID | null
  parent_id?: ID | null
  due_date?: string | null
  label_ids?: ID[]
}

export interface IssuePatch {
  title?: string
  description?: string
  team_id?: ID
  state_id?: ID
  priority?: Priority
  estimate?: number | null
  assignee_id?: ID | null
  project_id?: ID | null
  milestone_id?: ID | null
  cycle_id?: ID | null
  parent_id?: ID | null
  due_date?: string | null
  sort_order?: number
  label_ids?: ID[]
  add_label_ids?: ID[]
  remove_label_ids?: ID[]
  archived?: boolean
  snoozed_until?: string | null
  /** Required to force-close a lane-linked issue that the gateway has not verified. */
  force_note?: string
}

export interface Reaction {
  emoji: string
  user_ids: ID[]
}

export interface Comment {
  id: ID
  issue_id: ID
  user_id: ID
  body: string
  parent_id: ID | null
  created_at: string
  updated_at: string
  edited: boolean
  resolved_at: string | null
  reactions: Reaction[]
}

export type RelationType = 'blocks' | 'related' | 'duplicate'

export interface Relation {
  id: ID
  issue_id: ID
  related_issue_id: ID
  type: RelationType
  created_at: string
  /** The other side of the relation, from the perspective of the requested issue. */
  issue: Issue
  direction: 'blocks' | 'blocked_by' | 'related' | 'duplicate' | 'duplicated_by'
}

export interface Link {
  id: ID
  issue_id: ID
  url: string
  title: string
  created_at: string
}

export interface Activity {
  id: number
  issue_id: ID
  actor_id: ID | null
  kind: string
  from_value: string | null
  to_value: string | null
  created_at: string
}

export interface LaneDetail {
  lane: Record<string, unknown> & {
    key: string
    title: string
    status: LaneStatus
    cwd: string
    brief_path: string
    model: string | null
    done_signals: string
    claim_evidence: string | null
    objection: string | null
    turns_run: number
    last_turn_outcome: string | null
    last_report: string | null
    last_activity_at: string
    thread_channel_id: string | null
    created_at: string | null
  }
  history: { id: number; at: string; from_status: string | null; to_status: string; reason: string | null }[]
  transcript: { id: number; at: string; role: string; text: string }[]
  events: { id: number; kind: string; body: string; created_at: string }[]
}

export interface IssueDetail {
  issue: Issue
  children: Issue[]
  parent: Issue | null
  relations: Relation[]
  comments: Comment[]
  activity: Activity[]
  links: Link[]
  subscriber_ids: ID[]
  lane_detail: LaneDetail | null
  session_detail: SessionDetail | null
}

export interface ViewFilters {
  /** Every key is AND-ed; values inside one key are OR-ed. Empty/absent = no constraint. */
  state_ids?: ID[]
  state_types?: StateType[]
  priorities?: Priority[]
  assignee_ids?: (ID | 'none' | 'me')[]
  creator_ids?: ID[]
  label_ids?: ID[]
  project_ids?: (ID | 'none')[]
  cycle_ids?: (ID | 'none' | 'current')[]
  team_ids?: ID[]
  has_lane?: boolean
  lane_statuses?: LaneStatus[]
  due?: 'overdue' | 'today' | 'week' | 'none'
  text?: string
}

export type Grouping = 'status' | 'assignee' | 'project' | 'priority' | 'cycle' | 'label' | 'team' | 'none'
export type Ordering = 'manual' | 'priority' | 'updated' | 'created' | 'due' | 'title' | 'status'
export type DisplayProperty =
  | 'priority'
  | 'id'
  | 'status'
  | 'labels'
  | 'project'
  | 'cycle'
  | 'estimate'
  | 'due'
  | 'created'
  | 'updated'
  | 'assignee'
  | 'lane'
  | 'sub_issues'

export interface ViewDisplay {
  layout: 'list' | 'board'
  grouping: Grouping
  sub_grouping: Grouping
  ordering: Ordering
  show_completed: 'all' | 'week' | 'month' | 'none'
  show_sub_issues: boolean
  show_empty_groups: boolean
  properties: DisplayProperty[]
}

export interface View {
  id: ID
  name: string
  description: string | null
  icon: string
  color: string | null
  owner_id: ID
  team_id: ID | null
  filters: ViewFilters
  display: ViewDisplay
  shared: boolean
  created_at: string
  updated_at: string
}

export type FavoriteKind = 'issue' | 'project' | 'view' | 'cycle' | 'team' | 'label'

export interface Favorite {
  id: ID
  user_id: ID
  kind: FavoriteKind
  target_id: ID
  sort_order: number
}

export interface Notification {
  id: ID
  user_id: ID
  issue_id: ID | null
  project_id: ID | null
  kind: string
  actor_id: ID | null
  body: string
  created_at: string
  read_at: string | null
  snoozed_until: string | null
  archived_at: string | null
}

export interface Bootstrap {
  me: User
  users: User[]
  teams: Team[]
  states: WorkflowState[]
  labels: Label[]
  projects: Project[]
  cycles: Cycle[]
  milestones: Milestone[]
  views: View[]
  favorites: Favorite[]
  unread_count: number
}

export interface ProjectDetail {
  project: Project
  updates: ProjectUpdate[]
  milestones: Milestone[]
}

export interface DelegateInput {
  cwd: string
  done: string
  model?: 'auto' | 'small' | 'large'
  thread?: string
}

export type LaneAction = 'send' | 'object' | 'close' | 'reopen'

export const DEFAULT_DISPLAY: ViewDisplay = {
  layout: 'list',
  grouping: 'status',
  sub_grouping: 'none',
  ordering: 'priority',
  show_completed: 'all',
  show_sub_issues: true,
  show_empty_groups: false,
  properties: ['priority', 'id', 'status', 'labels', 'project', 'due', 'created', 'assignee', 'lane'],
}

export const PRIORITY_LABELS: Record<Priority, string> = {
  0: 'No priority',
  1: 'Urgent',
  2: 'High',
  3: 'Medium',
  4: 'Low',
}

export const STATE_TYPE_ORDER: StateType[] = ['triage', 'backlog', 'unstarted', 'started', 'completed', 'canceled']

export const ESTIMATE_POINTS = [0, 1, 2, 3, 5, 8] as const

export const PROJECT_STATUS_LABELS: Record<ProjectStatus, string> = {
  backlog: 'Backlog',
  planned: 'Planned',
  started: 'In Progress',
  paused: 'Paused',
  completed: 'Completed',
  canceled: 'Canceled',
}

export const HEALTH_LABELS: Record<ProjectHealth, string> = {
  on_track: 'On track',
  at_risk: 'At risk',
  off_track: 'Off track',
}

export const LANE_STATUS_LABELS: Record<LaneStatus, string> = {
  queued: 'Queued',
  working: 'Working',
  blocked: 'Blocked',
  done_claimed: 'Claims done',
  not_done: 'Changes requested',
  verified_done: 'Verified',
  closed: 'Closed',
  failed: 'Failed',
}

export type SessionState = 'working' | 'question' | 'blocked' | 'idle' | 'ended'

export interface SessionQuestionItem {
  id: string
  header: string | null
  question: string | null
  options: string[]
}

export interface SessionQuestion {
  request_id: string
  questions: SessionQuestionItem[]
}

export interface OmoSession {
  machine: string
  session_id: string
  name: string | null
  cwd: string | null
  state: SessionState
  streaming: boolean
  compacting: boolean
  message_count: number
  pending_messages: number
  last_user: string
  last_assistant: string
  questions: SessionQuestion[]
  updated_at: string | null
}

export interface SessionSummary {
  machine: string
  session_id: string
  state: SessionState
  streaming: boolean
  message_count: number
  pending_messages: number
  cwd: string | null
  updated_at: string | null
  questions: SessionQuestion[]
}

export interface SessionDetail extends SessionSummary {
  last_user: string
  last_assistant: string
}

export interface SessionMachine {
  machine: string
  ok: boolean
  error: string | null
  session_count: number
}

export interface SessionCounts {
  working: number
  question: number
  blocked: number
  idle: number
  total: number
}

export interface SessionsOverview {
  collected_at: string
  machines: SessionMachine[]
  sessions: OmoSession[]
  counts: SessionCounts
}

export interface SessionActionResult {
  ok: boolean
  result: unknown
}
