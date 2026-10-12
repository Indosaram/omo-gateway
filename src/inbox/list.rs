use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::Result;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum ClassFilter {
    Owner,
    Bot,
    Fyi,
}

impl ClassFilter {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Bot => "bot",
            Self::Fyi => "fyi",
        }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TaskItem {
    pub proposal_number: Option<i64>,
    pub id: String,
    pub source: String,
    pub author: Option<String>,
    pub title: Option<String>,
    pub class: String,
    pub reason: String,
    pub proposed_action: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub received_at: DateTime<Utc>,
    pub link: Option<String>,
    pub status: String,
}

/// Open work includes proposals awaiting approval; approved work is no longer pending.
pub async fn list(
    pool: &SqlitePool,
    class: Option<ClassFilter>,
    open: bool,
) -> Result<Vec<TaskItem>> {
    super::proposals::Proposals::new(pool.clone(), super::proposals::OWNER_ID).await?;
    Ok(sqlx::query_as(
        "SELECT p.number AS proposal_number,i.id,i.source,i.author,i.title,d.class,d.reason,d.proposed_action,d.due_at,i.received_at,i.link,d.status \
         FROM inbox_items i JOIN inbox_decisions d ON d.item_id=i.id \
         LEFT JOIN inbox_proposals p ON p.item_id=i.id \
         WHERE (? IS NULL OR d.class=?) AND (?=0 OR d.status IN ('open','proposed')) \
         ORDER BY d.due_at IS NULL, julianday(d.due_at), julianday(i.received_at), i.id",
    )
    .bind(class.map(ClassFilter::as_str))
    .bind(class.map(ClassFilter::as_str))
    .bind(open)
    .fetch_all(pool)
    .await?)
}

pub fn render(items: &[TaskItem]) -> String {
    items
        .iter()
        .map(|item| {
            let due = item
                .due_at
                .map(|due| {
                    due.with_timezone(&chrono_tz::Asia::Seoul)
                        .format("%Y-%m-%d %H:%M KST")
                        .to_string()
                })
                .unwrap_or_else(|| "-".into());
            format!(
                "- {}[{}] {} | {} | {} | {} | {}",
                item.proposal_number
                    .map(|number| format!("#{number} "))
                    .unwrap_or_default(),
                item.source,
                item.author.as_deref().unwrap_or("-"),
                item.proposed_action
                    .as_deref()
                    .or(item.title.as_deref())
                    .unwrap_or(&item.reason),
                due,
                item.link.as_deref().unwrap_or("-"),
                item.id
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
