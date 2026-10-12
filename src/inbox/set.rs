use sqlx::SqlitePool;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Status {
    Done,
    Dismissed,
    Approved,
}

impl Status {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Dismissed => "dismissed",
            Self::Approved => "approved",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SetError {
    #[error("unknown inbox item: {0}")]
    NotFound(String),
    #[error("inbox storage: {0}")]
    Storage(#[from] sqlx::Error),
    #[error("inbox approval: {0}")]
    Approval(#[from] crate::OmonError),
}

pub async fn set(pool: &SqlitePool, id: &str, status: Status) -> Result<(), SetError> {
    if matches!(status, Status::Approved) {
        super::proposals::Proposals::new(pool.clone(), super::proposals::OWNER_ID).await?;
        let exists: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM inbox_proposals WHERE item_id=? AND owner_id=?",
        )
        .bind(id)
        .bind(super::proposals::OWNER_ID)
        .fetch_one(pool)
        .await?;
        if exists > 0 {
            let actions = super::proposals::GatewayActions::from_env(pool.clone())?;
            return set_with_actions(pool, id, status, &actions).await;
        }
        return Err(SetError::Approval(crate::OmonError::Config(
            "no prepared inbox proposal; approval did not execute".into(),
        )));
    }
    update_status(pool, id, status).await
}

pub async fn set_with_actions(
    pool: &SqlitePool,
    id: &str,
    status: Status,
    actions: &dyn super::proposals::Actions,
) -> Result<(), SetError> {
    if matches!(status, Status::Approved) {
        let proposals =
            super::proposals::Proposals::new(pool.clone(), super::proposals::OWNER_ID).await?;
        match proposals.approve_item(id, actions).await? {
            super::proposals::ApprovalResult::Executed { .. }
            | super::proposals::ApprovalResult::AlreadyClaimed => return Ok(()),
            super::proposals::ApprovalResult::NotMatched => {
                return Err(SetError::Approval(crate::OmonError::Config(
                    "no prepared inbox proposal; approval did not execute".into(),
                )))
            }
        }
    }
    update_status(pool, id, status).await
}

async fn update_status(pool: &SqlitePool, id: &str, status: Status) -> Result<(), SetError> {
    let result = sqlx::query("UPDATE inbox_decisions SET status=? WHERE item_id=?")
        .bind(status.as_str())
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(SetError::NotFound(id.to_owned()));
    }
    Ok(())
}
