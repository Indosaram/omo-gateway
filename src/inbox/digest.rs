//! One gateway-owned digest. The scheduler owns outbound delivery and its ledger.
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use sqlx::SqlitePool;

use super::list::{list, render, ClassFilter};
use crate::{
    CronJob, CronJobSpec, CronRunOutcome, CronScheduler, CronTaskExecutor, OmonError, Result,
    SessionKey,
};

pub const JOB_ID: &str = "omon:inbox-digest";

/// The caller supplies the configured owner identity and the persisted home DM.
/// A shared channel or a thread cannot become a digest destination.
pub fn spec(home: &SessionKey, owner: &str) -> Result<CronJobSpec> {
    if !matches!(home.platform.as_str(), "discord" | "slack")
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
            // The digest can carry a one-time owner approval token: it must reach the
            // owner's DM but never the session transcript the agent reads.
            "attach_to_session": false,
            "origin": {"platform": home.platform, "chat_id": home.channel_id,
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

    async fn execute_with_outcome(&self, job: &CronJob) -> Result<CronRunOutcome> {
        if job.id == JOB_ID {
            Ok(CronRunOutcome {
                output: self.execute(job).await?,
                monitor_state: None,
            })
        } else {
            self.inner.execute_with_outcome(job).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_job_is_never_attached_to_the_session_transcript() {
        let home = SessionKey::new("discord", None::<String>, "dm", None::<String>, "owner");
        let spec = spec(&home, "owner").unwrap();
        assert_eq!(
            spec.payload
                .get("attach_to_session")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn slack_owner_dm_is_a_valid_home_and_sets_origin_platform() {
        let home = SessionKey::new(
            "slack",
            None::<String>,
            "D0OWNER",
            None::<String>,
            "U0OWNER",
        )
        .with_bot_id("U0BOT");
        let spec = spec(&home, "U0OWNER").unwrap();
        assert_eq!(spec.payload["origin"]["platform"], "slack");
        assert_eq!(spec.payload["origin"]["chat_id"], "D0OWNER");
        assert_eq!(spec.payload["origin"]["user_id"], "U0OWNER");
        assert_eq!(spec.payload["origin"]["bot_id"], "U0BOT");
        assert_eq!(spec.session_key, Some(home.storage_key()));

        let discord = SessionKey::new("discord", None::<String>, "dm", None::<String>, "owner");
        assert_eq!(spec_platform(&discord, "owner"), "discord");
    }

    #[test]
    fn slack_channel_thread_or_foreign_user_is_rejected() {
        for home in [
            SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U0OWNER"),
            SessionKey::new("slack", None::<String>, "D1", Some("1.2"), "U0OWNER"),
            SessionKey::new("slack", None::<String>, "D1", None::<String>, "U0OTHER"),
            SessionKey::new("web", None::<String>, "D1", None::<String>, "U0OWNER"),
        ] {
            assert!(spec(&home, "U0OWNER").is_err(), "{home:?}");
        }
    }

    #[tokio::test]
    async fn executor_guard_accepts_registered_slack_digest() {
        struct Never;
        #[async_trait]
        impl CronTaskExecutor for Never {
            async fn execute(&self, _: &CronJob) -> Result<Option<String>> {
                panic!("digest must not reach the inner executor")
            }
        }
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let home = SessionKey::new(
            "slack",
            None::<String>,
            "D0OWNER",
            None::<String>,
            "U0OWNER",
        )
        .with_bot_id("U0BOT");
        let spec = spec(&home, "U0OWNER").unwrap();
        let job = CronJob {
            id: JOB_ID.into(),
            session_key: spec.session_key.clone(),
            expression: spec.expression.clone(),
            payload_json: spec.payload.to_string(),
            enabled: true,
            next_run_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            authority: String::new(),
        };
        let executor = DigestExecutor {
            pool,
            inner: Arc::new(Never),
        };
        assert!(executor.execute(&job).await.is_ok());

        let mut tampered = spec.payload.clone();
        tampered["origin"]["chat_id"] = serde_json::json!("C0SHARED");
        let job = CronJob {
            payload_json: tampered.to_string(),
            ..job
        };
        assert!(executor.execute(&job).await.is_err());
    }

    fn spec_platform(home: &SessionKey, owner: &str) -> String {
        spec(home, owner).unwrap().payload["origin"]["platform"]
            .as_str()
            .unwrap()
            .to_owned()
    }
}
