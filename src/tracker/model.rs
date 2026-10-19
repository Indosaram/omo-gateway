use serde::{Deserialize, Deserializer, Serialize};

use crate::sessions::probe::PendingQuestion;

fn deserialize_nullable<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub email: Option<String>,
    pub avatar_color: String,
    pub kind: String,
    pub model: Option<String>,
    pub is_me: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Team {
    pub id: String,
    pub key: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub issue_seq: i64,
    pub system: bool,
    pub cycles_enabled: bool,
    pub cycle_weeks: i64,
    pub triage_enabled: bool,
    pub sort_order: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkflowState {
    pub id: String,
    pub team_id: String,
    pub name: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub state_type: String,
    pub color: String,
    pub position: f64,
    pub lane_status: Option<String>,
    pub description: Option<String>,
    pub session_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Label {
    pub id: String,
    pub team_id: Option<String>,
    pub name: String,
    pub color: String,
    pub parent_id: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub summary: String,
    pub icon: String,
    pub color: String,
    pub status: String,
    pub priority: i64,
    pub lead_id: Option<String>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub sort_order: f64,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
    pub team_ids: Vec<String>,
    pub member_ids: Vec<String>,
    pub issue_count: i64,
    pub completed_issue_count: i64,
    pub latest_health: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectUpdate {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub health: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Milestone {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub target_date: Option<String>,
    pub sort_order: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Cycle {
    pub id: String,
    pub team_id: String,
    pub number: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneSummary {
    pub key: String,
    pub status: String,
    pub model: Option<String>,
    pub turns_run: i64,
    pub last_activity_at: String,
    pub thread_channel_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub team_id: String,
    pub number: i64,
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub state_id: String,
    pub priority: i64,
    pub estimate: Option<i64>,
    pub assignee_id: Option<String>,
    pub creator_id: String,
    pub project_id: Option<String>,
    pub milestone_id: Option<String>,
    pub cycle_id: Option<String>,
    pub parent_id: Option<String>,
    pub due_date: Option<String>,
    pub sort_order: f64,
    pub lane_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub canceled_at: Option<String>,
    pub archived_at: Option<String>,
    pub snoozed_until: Option<String>,
    pub label_ids: Vec<String>,
    pub sub_issue_count: i64,
    pub sub_issue_completed: i64,
    pub comment_count: i64,
    pub lane: Option<LaneSummary>,
    pub session_ref: Option<String>,
    pub session: Option<SessionSummary>,
}

/// Live view of the omo session mirrored by an SES issue; `state` is one of
/// question|blocked|working|idle|ended.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub machine: String,
    pub session_id: String,
    pub state: String,
    pub streaming: bool,
    pub message_count: u64,
    pub pending_messages: u64,
    pub cwd: Option<String>,
    pub updated_at: Option<String>,
    pub questions: Vec<PendingQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    #[serde(flatten)]
    pub summary: SessionSummary,
    pub last_user: String,
    pub last_assistant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub emoji: String,
    pub user_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub issue_id: String,
    pub user_id: String,
    pub body: String,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub edited: bool,
    pub resolved_at: Option<String>,
    pub reactions: Vec<Reaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Relation {
    pub id: String,
    pub issue_id: String,
    pub related_issue_id: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub relation_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationDetail {
    pub id: String,
    pub issue_id: String,
    pub related_issue_id: String,
    #[serde(rename = "type")]
    pub relation_type: String,
    pub created_at: String,
    pub issue: Issue,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Link {
    pub id: String,
    pub issue_id: String,
    pub url: String,
    pub title: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Activity {
    pub id: i64,
    pub issue_id: String,
    pub actor_id: Option<String>,
    pub kind: String,
    pub from_value: Option<String>,
    pub to_value: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneHistoryEntry {
    pub id: i64,
    pub at: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneTranscriptEntry {
    pub id: i64,
    pub at: String,
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneEventEntry {
    pub id: i64,
    pub kind: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneDetail {
    pub lane: serde_json::Value,
    pub history: Vec<LaneHistoryEntry>,
    pub transcript: Vec<LaneTranscriptEntry>,
    pub events: Vec<LaneEventEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueDetail {
    pub issue: Issue,
    pub children: Vec<Issue>,
    pub parent: Option<Issue>,
    pub relations: Vec<RelationDetail>,
    pub comments: Vec<Comment>,
    pub activity: Vec<Activity>,
    pub links: Vec<Link>,
    pub subscriber_ids: Vec<String>,
    pub lane_detail: Option<LaneDetail>,
    pub session_detail: Option<SessionDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub color: Option<String>,
    pub owner_id: String,
    pub team_id: Option<String>,
    pub filters: serde_json::Value,
    pub display: serde_json::Value,
    pub shared: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Favorite {
    pub id: String,
    pub user_id: String,
    pub kind: String,
    pub target_id: String,
    pub sort_order: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub issue_id: Option<String>,
    pub project_id: Option<String>,
    pub kind: String,
    pub actor_id: Option<String>,
    pub body: String,
    pub created_at: String,
    pub read_at: Option<String>,
    pub snoozed_until: Option<String>,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bootstrap {
    pub me: User,
    pub users: Vec<User>,
    pub teams: Vec<Team>,
    pub states: Vec<WorkflowState>,
    pub labels: Vec<Label>,
    pub projects: Vec<Project>,
    pub cycles: Vec<Cycle>,
    pub milestones: Vec<Milestone>,
    pub views: Vec<View>,
    pub favorites: Vec<Favorite>,
    pub unread_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetail {
    pub project: Project,
    pub updates: Vec<ProjectUpdate>,
    pub milestones: Vec<Milestone>,
    pub member_ids: Vec<String>,
    pub team_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct IssueCreate {
    pub team_id: String,
    pub title: String,
    pub description: Option<String>,
    pub state_id: Option<String>,
    pub priority: Option<i64>,
    pub estimate: Option<i64>,
    pub assignee_id: Option<String>,
    pub project_id: Option<String>,
    pub milestone_id: Option<String>,
    pub cycle_id: Option<String>,
    pub parent_id: Option<String>,
    pub due_date: Option<String>,
    pub label_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct IssuePatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub team_id: Option<String>,
    pub state_id: Option<String>,
    pub priority: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub estimate: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub assignee_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub project_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub milestone_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub cycle_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub parent_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub due_date: Option<Option<String>>,
    pub sort_order: Option<f64>,
    pub label_ids: Option<Vec<String>>,
    pub add_label_ids: Option<Vec<String>>,
    pub remove_label_ids: Option<Vec<String>>,
    pub archived: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub snoozed_until: Option<Option<String>>,
    pub force_note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchIssuePatch {
    pub ids: Vec<String>,
    pub patch: IssuePatch,
}

#[derive(Debug, Deserialize)]
pub struct CommentCreate {
    pub body: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommentPatch {
    pub body: Option<String>,
    pub resolved: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ReactionToggle {
    pub emoji: String,
}

#[derive(Debug, Deserialize)]
pub struct RelationCreate {
    #[serde(rename = "type")]
    pub relation_type: String,
    pub related_issue_id: String,
}

#[derive(Debug, Deserialize)]
pub struct LinkCreate {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubscribeInput {
    pub subscribed: bool,
}

#[derive(Debug, Deserialize)]
pub struct DelegateInput {
    pub cwd: String,
    pub done: String,
    pub model: Option<String>,
    pub thread: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LaneActionInput {
    pub action: String,
    pub text: Option<String>,
    pub force: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectInput {
    pub name: String,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub lead_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub start_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub target_date: Option<Option<String>>,
    pub team_ids: Option<Vec<String>>,
    pub member_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProjectPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub lead_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub start_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub target_date: Option<Option<String>>,
    pub sort_order: Option<f64>,
    pub archived: Option<bool>,
    pub team_ids: Option<Vec<String>>,
    pub member_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectUpdateCreate {
    pub health: String,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectUpdatePatch {
    pub health: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MilestoneCreate {
    pub name: String,
    pub target_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MilestonePatch {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub target_date: Option<Option<String>>,
    pub sort_order: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct CycleCreate {
    pub team_id: String,
    pub starts_at: String,
    pub ends_at: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CyclePatch {
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub name: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub completed_at: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct LabelCreate {
    pub name: String,
    pub color: String,
    pub team_id: Option<String>,
    pub parent_id: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LabelPatch {
    pub name: Option<String>,
    pub color: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub parent_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub description: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct TeamCreate {
    pub key: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub cycles_enabled: Option<bool>,
    pub triage_enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TeamPatch {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub cycles_enabled: Option<bool>,
    pub cycle_weeks: Option<i64>,
    pub triage_enabled: Option<bool>,
    pub sort_order: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct StateCreate {
    pub team_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: String,
    pub color: String,
    pub position: Option<f64>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct StatePatch {
    pub name: Option<String>,
    pub color: Option<String>,
    pub position: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub description: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UserCreate {
    pub name: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_color: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UserPatch {
    pub name: Option<String>,
    pub display_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub email: Option<Option<String>>,
    pub avatar_color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ViewCreate {
    pub name: String,
    pub filters: serde_json::Value,
    pub display: serde_json::Value,
    pub description: Option<String>,
    pub team_id: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub shared: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ViewPatch {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub description: Option<Option<String>>,
    pub icon: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub color: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub team_id: Option<Option<String>>,
    pub filters: Option<serde_json::Value>,
    pub display: Option<serde_json::Value>,
    pub shared: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct FavoriteToggle {
    pub kind: String,
    pub target_id: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct NotificationPatch {
    pub read: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    pub snoozed_until: Option<Option<String>>,
    pub archived: Option<bool>,
}
