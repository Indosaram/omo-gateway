use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;

use crate::{OmonError, Result};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Lead,
    LaneLarge,
    LaneSmall,
    Kibitzer,
    InboxWrite,
    Digest,
    JudgeFallback,
    CronFix,
}

impl Purpose {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lead => "lead",
            Self::LaneLarge => "lane_large",
            Self::LaneSmall => "lane_small",
            Self::Kibitzer => "kibitzer",
            Self::InboxWrite => "inbox_write",
            Self::Digest => "digest",
            Self::JudgeFallback => "judge_fallback",
            Self::CronFix => "cron_fix",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub judge_calls: i64,
    pub fallback_calls: i64,
    pub fallback_rate: f64,
    pub cap_reached: bool,
    pub inbox_collected: i64,
    pub inbox_passed: i64,
}

pub async fn report(
    pool: &SqlitePool,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Result<Report> {
    // Judge rows count attempts, including separately recorded fallback attempts.
    let (attempts, fallback_calls, capped): (i64, i64, i64) = sqlx::query_as("SELECT COALESCE(sum(used_fallback=0),0),COALESCE(sum(used_fallback=1),0),COALESCE(sum(outcome LIKE 'capped:%'),0) FROM judge_calls WHERE julianday(created_at)>=julianday(?) AND julianday(created_at)<julianday(?)")
        .bind(since).bind(until).fetch_one(pool).await?;
    let judge_calls = attempts + fallback_calls;
    let inbox_passed: i64 = sqlx::query_scalar("SELECT count(*) FROM inbox_items WHERE julianday(collected_at)>=julianday(?) AND julianday(collected_at)<julianday(?)")
        .bind(since).bind(until).fetch_one(pool).await?;
    // Drop counts are stored per KST day, so the window is day-granular and
    // inclusive of the end day; an exclusive end would make intra-day windows empty.
    let dropped: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(count),0) FROM inbox_filter_stats WHERE day>=? AND day<=?",
    )
    .bind(
        since
            .with_timezone(&chrono_tz::Asia::Seoul)
            .date_naive()
            .to_string(),
    )
    .bind(
        until
            .with_timezone(&chrono_tz::Asia::Seoul)
            .date_naive()
            .to_string(),
    )
    .fetch_one(pool)
    .await?;
    Ok(Report {
        judge_calls,
        fallback_calls,
        fallback_rate: if judge_calls == 0 {
            0.0
        } else {
            f64::from(u32::try_from(fallback_calls).map_err(|e| OmonError::Config(e.to_string()))?)
                / f64::from(
                    u32::try_from(judge_calls).map_err(|e| OmonError::Config(e.to_string()))?,
                )
        },
        cap_reached: capped > 0,
        inbox_collected: inbox_passed + dropped,
        inbox_passed,
    })
}

impl Report {
    pub fn digest_line(&self) -> String {
        format!(
            "어제: jev {}회(폴백 {}, 상한 도달 {}), 인박스 수집 {}건 중 필터 통과 {}건",
            self.judge_calls,
            self.fallback_calls,
            self.cap_reached,
            self.inbox_collected,
            self.inbox_passed
        )
    }
}

#[derive(Debug, clap::Args)]
pub struct UsageArgs {
    #[arg(long, default_value = "24h")]
    pub since: String,
    #[arg(long)]
    pub json: bool,
}

pub async fn render(pool: &SqlitePool, args: &UsageArgs, now: DateTime<Utc>) -> Result<String> {
    let (number, seconds) = match args.since.chars().last() {
        Some('h') => (args.since.strip_suffix('h'), 3600),
        Some('d') => (args.since.strip_suffix('d'), 86400),
        Some('m') => (args.since.strip_suffix('m'), 60),
        _ => {
            return Err(OmonError::Config(
                "--since requires positive Nm, Nh or Nd".into(),
            ))
        }
    };
    let duration = number
        .and_then(|n| n.parse::<i64>().ok())
        .filter(|n| *n > 0)
        .and_then(|n| n.checked_mul(seconds))
        .and_then(chrono::Duration::try_seconds)
        .ok_or_else(|| OmonError::Config("invalid --since duration".into()))?;
    let since = now
        .checked_sub_signed(duration)
        .ok_or_else(|| OmonError::Config("--since out of range".into()))?;
    let report = report(pool, since, now).await?;
    if args.json {
        serde_json::to_string(&report).map_err(|e| OmonError::Config(e.to_string()))
    } else {
        Ok(format!(
            "jev: {} calls, {} fallbacks, rate {:.3}, cap reached {}",
            report.judge_calls, report.fallback_calls, report.fallback_rate, report.cap_reached
        ))
    }
}

pub async fn run(args: UsageArgs) -> i32 {
    let result = async {
        let url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://omon_gateway.db".into());
        let db = crate::storage::Database::connect(&url).await?;
        let result = render(db.pool(), &args, Utc::now()).await;
        db.close().await;
        result
    }
    .await;
    match result {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            2
        }
    }
}

pub async fn capture_lane_start(
    pool: &SqlitePool,
    lane_key: &str,
    cwd: &std::path::Path,
    purpose: Purpose,
) -> Result<()> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::process::Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(["rev-parse", "HEAD"])
            .kill_on_drop(true)
            .output(),
    )
    .await;
    let revision = match output {
        Ok(Ok(output)) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        }
        _ => None,
    };
    sqlx::query("INSERT INTO lane_usage_context(lane_key,start_commit,purpose) VALUES(?,?,?) ON CONFLICT(lane_key) DO NOTHING")
        .bind(lane_key).bind(revision).bind(purpose.as_str()).execute(pool).await?;
    Ok(())
}

pub async fn session_purpose(
    pool: &SqlitePool,
    session: &crate::SessionContext,
) -> Result<Option<Purpose>> {
    if let Some(value) = session.state.metadata.get("usage_purpose") {
        return serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|e| OmonError::Config(e.to_string()));
    }
    if session.key.platform == "lane" {
        let purpose: Option<String> =
            sqlx::query_scalar("SELECT purpose FROM lane_usage_context WHERE lane_key=?")
                .bind(&session.key.channel_id)
                .fetch_optional(pool)
                .await?;
        return purpose
            .map(|p| {
                serde_json::from_value(Value::String(p))
                    .map_err(|e| OmonError::Config(e.to_string()))
            })
            .transpose();
    }
    if session.key.platform == "judge" {
        return Ok(Some(Purpose::JudgeFallback));
    }
    if session.key.channel_id == "cron:inbox-triage" {
        return Ok(Some(Purpose::InboxWrite));
    }
    if session.key.platform == "discord" {
        return Ok(Some(Purpose::Lead));
    }
    Ok(None)
}

pub async fn yesterday(pool: &SqlitePool, now: DateTime<Utc>) -> Result<Report> {
    use chrono::TimeZone;
    let day = now.with_timezone(&chrono_tz::Asia::Seoul).date_naive();
    let end = chrono_tz::Asia::Seoul
        .from_local_datetime(
            &day.and_hms_opt(0, 0, 0)
                .ok_or_else(|| OmonError::Config("invalid usage day".into()))?,
        )
        .single()
        .ok_or_else(|| OmonError::Config("ambiguous usage day".into()))?
        .with_timezone(&Utc);
    let mut report = report(pool, end - chrono::Duration::days(1), end).await?;
    let previous = day
        .pred_opt()
        .ok_or_else(|| OmonError::Config("invalid previous day".into()))?;
    let dropped: i64 =
        sqlx::query_scalar("SELECT COALESCE(sum(count),0) FROM inbox_filter_stats WHERE day=?")
            .bind(previous.to_string())
            .fetch_one(pool)
            .await?;
    report.inbox_collected = report.inbox_passed + dropped;
    Ok(report)
}
