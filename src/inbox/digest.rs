//! One gateway-owned digest. The scheduler owns outbound delivery and its ledger.
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use sqlx::SqlitePool;

use super::list::{list, render, ClassFilter};
use crate::{CronJob, CronJobSpec, CronScheduler, CronTaskExecutor, OmonError, Result, SessionKey};

pub const JOB_ID: &str = "omon:inbox-digest";

/// The caller supplies the configured owner identity and the persisted home DM.
/// A shared channel or a thread cannot become a digest destination.
pub fn spec(home: &SessionKey, owner: &str) -> Result<CronJobSpec> {
    if home.platform != "discord"
        || home.guild_id.is_some()
        || home.thread_id.is_some()
        || home.user_id != owner
        || owner.is_empty()
        || home.channel_id.is_empty()
    {
        return Err(OmonError::Config(
            "inbox digest requires the owner's home DM".into(),
        ));
    }
    let mut spec = CronJobSpec::new(
        "0 9 * * *",
        json!({
            "id": JOB_ID,
            "schedule": {"kind": "cron", "expr": "0 9 * * *", "timezone": "Asia/Seoul"},
            "deliver": "origin",
            "origin": {"platform": "discord", "chat_id": home.channel_id,
                "user_id": owner, "bot_id": home.bot_id}
        }),
    );
    spec.session_key = Some(home.storage_key());
    Ok(spec)
}

pub async fn register(
    pool: &SqlitePool,
    scheduler: &CronScheduler,
    home: &SessionKey,
    owner: &str,
) -> Result<()> {
    let spec = spec(home, owner)?;
    sqlx::query(
        "INSERT INTO sessions(session_key,platform,channel_id,user_id) VALUES(?,?,?,?) \
         ON CONFLICT(session_key) DO NOTHING",
    )
    .bind(home.storage_key())
    .bind(&home.platform)
    .bind(&home.channel_id)
    .bind(&home.user_id)
    .execute(pool)
    .await?;
    scheduler.register_with_id(JOB_ID, spec).await?;
    Ok(())
}

pub async fn build(pool: &SqlitePool, now: DateTime<Utc>) -> Result<Option<String>> {
    let local = now.with_timezone(&chrono_tz::Asia::Seoul);
    let midnight = chrono_tz::Asia::Seoul
        .from_local_datetime(
            &local
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| OmonError::Config("invalid digest day".into()))?,
        )
        .single()
        .ok_or_else(|| OmonError::Config("ambiguous digest day".into()))?
        .with_timezone(&Utc);
    // Completed delivery survives restarts. Failed deliveries do not suppress retry.
    let delivered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cron_runs r JOIN cron_outputs o ON o.run_id=r.run_id AND o.job_id=r.job_id \
         WHERE r.job_id=? AND r.status='succeeded' AND julianday(r.completed_at)>=julianday(?)",
    ).bind(JOB_ID).bind(midnight).fetch_one(pool).await?;
    if delivered > 0 {
        return Ok(None);
    }
    let owner = list(pool, Some(ClassFilter::Owner), true).await?;
    let bot = list(pool, Some(ClassFilter::Bot), true).await?;
    let errors: Vec<(String,)> = sqlx::query_as(
        "SELECT source FROM inbox_cursors WHERE error IS NOT NULL AND \
         (lower(error) LIKE '%auth_expired%' OR lower(error) LIKE '%unauthorized%' OR lower(error) LIKE '%login%' OR error LIKE '%401%') ORDER BY source",
    ).fetch_all(pool).await?;
    let cron = crate::cron::report::digest_section(pool, now).await?;
    if owner.is_empty() && bot.is_empty() && errors.is_empty() && cron.is_none() {
        return Ok(None);
    }
    let usage = crate::usage::yesterday(pool, now).await?;
    let mut message = format!(
        "## Owner tasks\n{}\n\n## Bot proposals\n{}",
        render(&owner),
        render(&bot)
    );
    message.push_str(&format!("\n\n{}", usage.digest_line()));
    if let Some(cron) = cron {
        message.push_str(&format!("\n\n{cron}"));
    }
    if !errors.is_empty() {
        message.push_str("\n\n## Source authentication\n");
        message.push_str(
            &errors
                .iter()
                .map(|(source,)| format!("- [{source}] login required"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    Ok(Some(message))
}

pub struct DigestExecutor {
    pub pool: SqlitePool,
    pub inner: Arc<dyn CronTaskExecutor>,
}

#[async_trait]
impl CronTaskExecutor for DigestExecutor {
    async fn execute(&self, job: &CronJob) -> Result<Option<String>> {
        if job.id == JOB_ID {
            let home = job
                .session_key
                .as_deref()
                .ok_or_else(|| OmonError::Config("missing digest DM".into()))?
                .parse::<SessionKey>()?;
            let expected = spec(&home, &home.user_id)?;
            let payload = job.payload()?;
            if payload.get("origin") != expected.payload.get("origin")
                || payload.get("deliver") != expected.payload.get("deliver")
            {
                return Err(OmonError::Config(
                    "digest delivery must remain owner DM only".into(),
                ));
            }
            build(&self.pool, Utc::now()).await
        } else {
            self.inner.execute(job).await
        }
    }
}
