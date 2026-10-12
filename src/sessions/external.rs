use std::{sync::Arc, time::Duration};

use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::lanes::{
    activity::LaneActivity,
    signals::{parse_signals, DoneSignal},
};
use crate::storage::{get_lane, Lane};
use crate::work_heartbeat::{Clock, SystemClock};
use crate::{OmonError, Result};

use super::control::{SendMode, SessionControl, Target};
use super::registry::{CommandRunner, ProcessCommandRunner};

pub const MIGRATION: &str = include_str!("external.sql");
pub const FALLBACK_INTERVAL: Duration = Duration::from_secs(300);

fn invalid(error: impl std::fmt::Display) -> OmonError {
    OmonError::Config(error.to_string())
}

#[derive(sqlx::FromRow)]
struct Adoption {
    sent_at: Option<chrono::DateTime<Utc>>,
    cursor: Option<String>,
}

pub struct ExternalDriver<R = ProcessCommandRunner> {
    pool: SqlitePool,
    control: SessionControl<R>,
    clock: Arc<dyn Clock>,
}

impl ExternalDriver<ProcessCommandRunner> {
    pub fn from_environment(pool: SqlitePool) -> Result<Self> {
        Ok(Self::new(
            pool,
            SessionControl::from_environment().map_err(invalid)?,
            Arc::new(SystemClock),
        ))
    }
}

impl<R: CommandRunner + 'static> ExternalDriver<R> {
    pub fn new(pool: SqlitePool, control: SessionControl<R>, clock: Arc<dyn Clock>) -> Self {
        Self {
            pool,
            control,
            clock,
        }
    }

    pub async fn adopt(
        &self,
        target: &Target,
        goal: &str,
        done: &str,
        lead: &str,
    ) -> Result<String> {
        if goal.trim().is_empty() || lead.trim().is_empty() {
            return Err(invalid("goal and lead must not be empty"));
        }
        parse_signals(done).map_err(invalid)?;
        let session = self
            .control
            .session(target, "send")
            .await
            .map_err(invalid)?;
        let cwd = session
            .cwd
            .ok_or_else(|| invalid("session cwd is required to verify Done signals"))?;
        let key = format!("external-{}", uuid::Uuid::new_v4().simple());
        let mut lane = Lane::new(&key, goal, lead, &cwd, "", target.to_string(), done);
        lane.status = "working".into();
        lane.omo_thread_id = Some(target.session().into());
        let baseline = self.control.read(target, 1, None).await.map_err(invalid)?;
        let cursor = next_cursor(&baseline).ok_or_else(|| {
            invalid("read response has no cursor; cannot safely adopt historical output")
        })?;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("INSERT INTO lanes(key,title,lead_session_key,cwd,brief_path,session_key,omo_thread_id,status,done_signals,last_activity_at,kind,external_target) VALUES(?,?,?,?,?,?,?,'working',?,?,'external',?)")
                .bind(&lane.key).bind(&lane.title).bind(&lane.lead_session_key).bind(&lane.cwd)
                .bind(&lane.brief_path).bind(&lane.session_key).bind(&lane.omo_thread_id)
                .bind(&lane.done_signals).bind(self.clock.now()).bind(target.to_string()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO external_sessions(lane_key,goal,send_key,cursor) VALUES(?,?,?,?)")
            .bind(&key)
            .bind(goal)
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(cursor)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO lane_history(lane_key,at,to_status,reason) VALUES(?,?,'working','external session adopted')")
                .bind(&key).bind(self.clock.now()).execute(&mut *tx).await?;
        tx.commit().await?;
        crate::lanes::runner::wake(&self.pool).await?;
        Ok(key)
    }

    pub async fn release(&self, target: &Target) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let key: Option<String> =
            sqlx::query_scalar("SELECT key FROM lanes WHERE external_target=?")
                .bind(target.to_string())
                .fetch_optional(&mut *tx)
                .await?;
        let key = key.ok_or_else(|| invalid("session is not adopted"))?;
        sqlx::query("UPDATE external_sessions SET released_at=? WHERE lane_key=?")
            .bind(self.clock.now())
            .bind(&key)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE lanes SET external_target=NULL WHERE key=?")
            .bind(&key)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        crate::lanes::runner::wake(&self.pool).await
    }

    pub async fn is_external(pool: &SqlitePool, key: &str) -> Result<bool> {
        Ok(
            sqlx::query_scalar("SELECT kind='external' FROM lanes WHERE key=?")
                .bind(key)
                .fetch_one(pool)
                .await?,
        )
    }

    async fn target(&self, key: &str) -> Result<Option<Target>> {
        let target: Option<String> =
            sqlx::query_scalar("SELECT external_target FROM lanes WHERE key=? AND kind='external'")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?
                .flatten();
        target
            .map(|value| value.parse().map_err(invalid))
            .transpose()
    }

    pub async fn send_goal(&self, key: &str) -> Result<()> {
        let lane = get_lane(&self.pool, key)
            .await?
            .ok_or_else(|| invalid("lane not found"))?;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let row: Option<(String, String, String, String, Option<String>)> = sqlx::query_as("SELECT l.external_target,l.done_signals,e.goal,e.send_key,e.sent_at FROM lanes l JOIN external_sessions e ON e.lane_key=l.key WHERE l.key=? AND e.released_at IS NULL AND l.external_target IS NOT NULL AND l.status='working'")
            .bind(key).fetch_optional(&mut *tx).await?;
        if let Some((target, done, goal, send_key, None)) = row {
            let target: Target = target.parse().map_err(invalid)?;
            let separator = if target.machine() == "local" {
                "\n\n"
            } else {
                " | "
            };
            let footer = crate::lanes::runner::brief_footer(&lane);
            let footer = if target.machine() == "local" {
                footer
            } else {
                footer.replace(['\n', '\r'], " | ")
            };
            let text = format!("{goal}{separator}{footer}{separator}Done = {done}{separator}For this external session, claim completion with a standalone line: [CLAIM_DONE] {key} | <checkable evidence>. A [REPORT] done line is not a claim.");
            self.control
                .send(
                    &target,
                    &text,
                    SendMode::Auto,
                    send_key.parse().map_err(invalid)?,
                )
                .await
                .map_err(invalid)?;
            sqlx::query(
                "UPDATE external_sessions SET sent_at=? WHERE lane_key=? AND sent_at IS NULL",
            )
            .bind(self.clock.now())
            .bind(key)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn read_new(&self, key: &str) -> Result<usize> {
        let Some(target) = self.target(key).await? else {
            return Ok(0);
        };
        let adoption: Adoption =
            sqlx::query_as("SELECT sent_at,cursor FROM external_sessions WHERE lane_key=?")
                .bind(key)
                .fetch_one(&self.pool)
                .await?;
        if adoption.sent_at.is_none() {
            return Ok(0);
        }
        let output = self
            .control
            .read(&target, 100, adoption.cursor.as_deref())
            .await
            .map_err(invalid)?;
        let cursor = next_cursor(&output).ok_or_else(|| invalid("read response has no cursor"))?;
        let items = assistant_items(&output)?;
        let activity = LaneActivity::new(self.pool.clone(), self.clock.clone());
        let mut count = 0;
        for (id, text) in items {
            let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
            let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM external_sessions WHERE lane_key=? AND released_at IS NULL)")
                .bind(key).fetch_one(&mut *tx).await?;
            if !active {
                tx.rollback().await?;
                return Ok(count);
            }
            let seen: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM external_session_items WHERE lane_key=? AND item_id=?)")
                .bind(key).bind(&id).fetch_one(&mut *tx).await?;
            if seen {
                tx.rollback().await?;
                continue;
            }
            tx.commit().await?;
            activity.captured_item(key, &text, 0, &id).await?;
            let prefix = format!("[CLAIM_DONE] {key} | ");
            if let Some(evidence) = text
                .lines()
                .find_map(|line| line.strip_prefix(&prefix))
                .filter(|s| !s.trim().is_empty())
            {
                let lane = get_lane(&self.pool, key)
                    .await?
                    .ok_or_else(|| invalid("lane not found"))?;
                if matches!(lane.status.as_str(), "working" | "blocked" | "not_done")
                    && self.target(key).await?.is_some()
                {
                    crate::lanes::cli::change_claim_at(
                        &self.pool,
                        key,
                        true,
                        evidence,
                        self.clock.now(),
                    )
                    .await?;
                    crate::lanes::runner::wake(&self.pool).await?;
                }
            }
            let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
            let inserted = sqlx::query(
                "INSERT OR IGNORE INTO external_session_items(lane_key,item_id) VALUES(?,?)",
            )
            .bind(key)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
            if inserted.rows_affected() == 1 {
                sqlx::query(
                    "INSERT INTO lane_transcript(lane_key,at,role,text) VALUES(?,?,'assistant',?)",
                )
                .bind(key)
                .bind(self.clock.now())
                .bind(&text)
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            count += 1;
        }
        sqlx::query(
            "UPDATE external_sessions SET cursor=? WHERE lane_key=? AND released_at IS NULL",
        )
        .bind(cursor)
        .bind(key)
        .execute(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn drive(
        &self,
        key: &str,
        notify: Arc<Notify>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        self.send_goal(key).await?;
        let mut fallback = tokio::time::interval_at(
            tokio::time::Instant::now() + FALLBACK_INTERVAL,
            FALLBACK_INTERVAL,
        );
        fallback.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            if self.target(key).await?.is_none() {
                return Ok(());
            }
            let lane = get_lane(&self.pool, key)
                .await?
                .ok_or_else(|| invalid("lane not found"))?;
            if matches!(lane.status.as_str(), "verified_done" | "closed" | "failed") {
                return Ok(());
            }
            tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                _ = notify.notified() => {},
                _ = fallback.tick() => {},
            }
            self.read_new(key).await?;
            self.send_pending(key).await?;
        }
    }

    async fn send_pending(&self, key: &str) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let target: Option<String> =
            sqlx::query_scalar("SELECT external_target FROM lanes WHERE key=?")
                .bind(key)
                .fetch_one(&mut *tx)
                .await?;
        let Some(target) = target else {
            tx.rollback().await?;
            return Ok(());
        };
        let target: Target = target.parse().map_err(invalid)?;
        sqlx::query(
            "UPDATE lanes SET status='working', updated_at=? WHERE key=? AND status='not_done'",
        )
        .bind(self.clock.now())
        .bind(key)
        .execute(&mut *tx)
        .await?;
        let messages: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id,text FROM lane_inbox WHERE lane_key=? AND consumed_at IS NULL ORDER BY id",
        )
        .bind(key)
        .fetch_all(&mut *tx)
        .await?;
        for (id, text) in messages {
            let text = if target.machine() == "local" {
                text
            } else {
                text.replace(['\n', '\r'], " | ")
            };
            let digest = Sha256::digest(format!("{key}:inbox:{id}"));
            let bytes: [u8; 16] = digest[..16].try_into().map_err(invalid)?;
            self.control
                .send(
                    &target,
                    &text,
                    SendMode::Auto,
                    uuid::Uuid::from_bytes(bytes),
                )
                .await
                .map_err(invalid)?;
            sqlx::query("UPDATE lane_inbox SET consumed_at=? WHERE id=? AND consumed_at IS NULL")
                .bind(self.clock.now())
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

pub async fn remote_checks(pool: &SqlitePool, lane: &Lane) -> Result<Option<Vec<String>>> {
    let target: Option<String> =
        sqlx::query_scalar("SELECT external_target FROM lanes WHERE key=? AND kind='external'")
            .bind(&lane.key)
            .fetch_optional(pool)
            .await?
            .flatten();
    let Some(target) = target else {
        return Ok(None);
    };
    let target: Target = target.parse().map_err(invalid)?;
    if target.machine() == "local" {
        return Ok(None);
    }
    let control = SessionControl::from_environment().map_err(invalid)?;
    remote_checks_with(&control, &target, lane).await.map(Some)
}

pub async fn remote_checks_with<R: CommandRunner + 'static>(
    control: &SessionControl<R>,
    target: &Target,
    lane: &Lane,
) -> Result<Vec<String>> {
    let mut reasons = Vec::new();
    for signal in parse_signals(&lane.done_signals).map_err(invalid)? {
        let argv = match signal {
            DoneSignal::Command { argv, stdout_regex } => {
                let mut command = vec!["python3".into(), "-c".into(), "import os,subprocess,sys; os.chdir(sys.argv[1]); r=subprocess.run(sys.argv[3:],capture_output=True,text=True); print(r.stdout,end=''); print(r.stderr,file=sys.stderr,end=''); sys.exit(r.returncode)".into(), lane.cwd.clone(), "--".into()];
                command.extend(argv);
                let output = control.output(target, &command).await.map_err(invalid)?;
                if output.code != 0 { reasons.push(format!("remote command exited {}", output.code)); }
                if let Some(pattern) = stdout_regex {
                    let pattern = regex::Regex::new(&pattern).map_err(invalid)?;
                    if !pattern.is_match(&String::from_utf8_lossy(&output.stdout)) { reasons.push("remote command stdout did not match".into()); }
                }
                continue;
            }
            DoneSignal::File { path, sha256, json_path, json_value } => vec![
                "python3".into(), "-c".into(),
                "import os,sys,json,hashlib; os.chdir(sys.argv[1]); b=open(sys.argv[2],'rb').read(); spec=json.loads(sys.argv[3]); ok=not spec['sha256'] or hashlib.sha256(b).hexdigest()==spec['sha256']; v=json.loads(b) if spec['json_path'] else None\nif spec['json_path']:\n for p in spec['json_path'].lstrip('$.').split('.'):\n  v=v[int(p)] if isinstance(v,list) else v[p]\n ok=ok and v==spec['json_value']\nsys.exit(0 if ok else 1)".into(),
                lane.cwd.clone(), path.to_string_lossy().into_owned(),
                serde_json::json!({"sha256":sha256,"json_path":json_path,"json_value":json_value}).to_string(),
            ],
            DoneSignal::Merged { .. } | DoneSignal::Closed { .. } | DoneSignal::Published { .. } | DoneSignal::Url { .. } => {
                use crate::lanes::signals::SignalRunner;
                let result = crate::lanes::signals::SystemSignalRunner::new().check(&signal, std::path::Path::new(".")).await;
                if !result.ok { reasons.push(format!("{}: {}", result.signal, result.detail)); }
                continue;
            }
        };
        let output = control.output(target, &argv).await.map_err(invalid)?;
        if output.code != 0 {
            reasons.push(format!(
                "remote file check failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }
    let script = "import os,subprocess,sys; os.chdir(sys.argv[1])\ndef git(path,*args):\n r=subprocess.run(['git','-C',path,*args],capture_output=True,text=True)\n if r.returncode: raise RuntimeError(r.stderr)\n return r.stdout\np=subprocess.run(['git','rev-parse','--is-inside-work-tree'],capture_output=True,text=True)\nif p.returncode:\n if 'not a git repository' in p.stderr: sys.exit(0)\n raise RuntimeError(p.stderr)\npaths=[os.getcwd()]+[l[9:] for l in git(os.getcwd(),'worktree','list','--porcelain').splitlines() if l.startswith('worktree ')]\nfor path in dict.fromkeys(paths):\n if git(path,'status','--porcelain','--untracked-files=no').strip(): raise RuntimeError('uncommitted '+path)\n if git(path,'remote').strip() and git(path,'log','--branches','--not','--remotes','--oneline').strip(): raise RuntimeError('unpushed '+path)";
    let output = control
        .output(
            target,
            &[
                "python3".into(),
                "-c".into(),
                script.into(),
                lane.cwd.clone(),
            ],
        )
        .await
        .map_err(invalid)?;
    if output.code != 0 {
        reasons.push(format!(
            "remote worktree check failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(reasons)
}

fn next_cursor(value: &Value) -> Option<String> {
    value
        .get("nextCursor")
        .or_else(|| value.get("next_cursor"))
        .or_else(|| value.get("cursor"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn assistant_items(value: &Value) -> Result<Vec<(String, String)>> {
    let entries = value
        .get("messages")
        .or_else(|| value.get("items"))
        .or_else(|| value.get("entries"))
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .ok_or_else(|| invalid("read response has no message array"))?;
    let mut items = Vec::new();
    for entry in entries {
        if entry.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let text = entry
            .get("text")
            .or_else(|| entry.get("content"))
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("assistant message has no text"))?;
        let id = entry
            .get("id")
            .or_else(|| entry.get("itemId"))
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("assistant message has no stable id"))?;
        items.push((id.to_owned(), text.to_owned()));
    }
    Ok(items)
}
