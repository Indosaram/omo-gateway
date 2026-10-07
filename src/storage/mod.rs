mod db;
mod message_search;
mod messenger_policy;

use sqlx::SqlitePool;

use crate::Result;
pub use db::{
    approve_pending_write, approve_pending_write_scoped, clear_session_resume_pending,
    count_resume_pending_sessions, delete_pending_write, fetch_resume_pending_session_keys,
    find_last_unfinished_user_turn, get_pending_write, get_pending_write_scoped,
    get_stale_in_progress_work_items, get_work_item, has_platform_message_id, insert_work_item,
    is_session_suspended, list_pending_writes, list_pending_writes_scoped, load_dead_targets,
    mark_session_resume_pending, mark_session_suspended, mark_work_item_done, persist_dead_target,
    reject_pending_write, reject_pending_write_scoped, remove_dead_target,
    remove_dead_targets_for_channel, stage_pending_write, update_work_item_progress,
    write_approval_enabled, Database, PendingWrite, PendingWriteScope, UnfinishedTurn, WorkItem,
};
pub use message_search::{MessageSearchDocument, MessageSearchHit, MessageSearchIndex};
pub use messenger_policy::MessengerPolicyStore;

pub async fn init_pool(database_url: &str) -> Result<SqlitePool> {
    Ok(Database::connect(database_url).await?.pool().clone())
}
