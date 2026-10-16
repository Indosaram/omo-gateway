use std::{
    sync::Arc,
    time::{Duration, Instant},
};

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

use super::control::{ControlError, SendMode, SessionControl, Target};
use super::registry::{CommandRunner, ProcessCommandRunner};

pub const MIGRATION: &str = include_str!("../../migrations/0040_external_sessions.sql");
pub const FALLBACK_INTERVAL: Duration = Duration::from_secs(300);
const MAX_CONSECUTIVE_ERRORS: u32 = 3;
/// Consecutive errors only fail a lane once they span this long, so a burst of wake-ups
/// during one outage cannot exhaust the error budget.
const ERROR_WINDOW: Duration = Duration::from_secs(120);
/// Remote command Done signals get the same budget as the local runner.
const REMOTE_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Default)]
struct ErrorStreak {
    count: u32,
    since: Option<Instant>,
}

impl ErrorStreak {
    /// Records one failed step; true once enough errors have accumulated over enough time.
    fn record(&mut self, now: Instant) -> bool {
        self.count += 1;
        let since = *self.since.get_or_insert(now);
        self.count >= MAX_CONSECUTIVE_ERRORS && now.saturating_duration_since(since) >= ERROR_WINDOW
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

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
        let lead = lead.trim();
        if goal.trim().is_empty() || lead.is_empty() {
            return Err(invalid("goal and lead must not be empty"));
        }
        // Same rule as `lanes::cli`: an unparsable lead would be stored and never woken.
        if crate::SessionKey::from_storage_key(lead.trim()).is_err() {
            return Err(invalid("lead must be a session storage key"));
        }
        parse_signals(done).map_err(invalid)?;
        let (resolved, _) = self.control.resolve(target).await.map_err(invalid)?;
        let target = &resolved;
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
        let lane: Option<(String, String)> =
            sqlx::query_as("SELECT key,status FROM lanes WHERE external_target=?")
                .bind(target.to_string())
                .fetch_optional(&mut *tx)
                .await?;
        let (key, status) = lane.ok_or_else(|| invalid("session is not adopted"))?;
        if !matches!(status.as_str(), "verified_done" | "closed" | "failed") {
            sqlx::query("UPDATE lanes SET status='closed',updated_at=? WHERE key=? AND status=?")
                .bind(self.clock.now())
                .bind(&key)
                .bind(&status)
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO lane_history(lane_key,at,from_status,to_status,reason) VALUES(?,?,?,'closed','external session released')")
                .bind(&key).bind(self.clock.now()).bind(&status).execute(&mut *tx).await?;
        }
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
        let row: Option<(String, String, String, String, Option<String>)> = sqlx::query_as("SELECT l.external_target,l.done_signals,e.goal,e.send_key,e.sent_at FROM lanes l JOIN external_sessions e ON e.lane_key=l.key WHERE l.key=? AND e.released_at IS NULL AND l.external_target IS NOT NULL AND l.status='working'")
            .bind(key).fetch_optional(&self.pool).await?;
        if let Some((target, done, goal, send_key, None)) = row {
            let target: Target = target.parse().map_err(invalid)?;
            let separator = if target.machine() == "local" {
                "\n\n"
            } else {
                " | "
            };
            let footer = crate::lanes::runner::brief_footer(&lane);
            let mut text = format!("{goal}{separator}{footer}{separator}Done = {done}{separator}For this external session, claim completion with a standalone line: [CLAIM_DONE] {key} | <checkable evidence>. A [REPORT] done line is not a claim.");
            if target.machine() != "local" {
                // Remote sends reject multi-line text; the goal and Done text may hold newlines too.
                text = text.replace(['\n', '\r'], " | ");
            }
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
            .execute(&self.pool)
            .await?;
        }
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
        let is_numeric_cursor = adoption
            .cursor
            .as_deref()
            .and_then(|c| c.parse::<i64>().ok());
        let read_cursor = if is_numeric_cursor.is_some() {
            None
        } else {
            adoption.cursor.as_deref()
        };
        let output = self
            .control
            .read(&target, 100, read_cursor)
            .await
            .map_err(invalid)?;
        let cursor = next_cursor(&output).ok_or_else(|| invalid("read response has no cursor"))?;
        let items = assistant_items(&output)?;
        let items: Vec<(String, String)> = if let Some(last_seq) = is_numeric_cursor {
            items
                .into_iter()
                .filter(|(id, _)| id.parse::<i64>().map(|seq| seq > last_seq).unwrap_or(true))
                .collect()
        } else {
            items
        };
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
            let generation: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened'")
                .bind(key).fetch_one(&mut *tx).await?;
            let seen: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM external_session_items WHERE lane_key=? AND item_id=?)")
                .bind(key).bind(&id).fetch_one(&mut *tx).await?;
            if seen {
                tx.rollback().await?;
                continue;
            }
            tx.commit().await?;
            activity.captured_item(key, &text, generation, &id).await?;
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
        let mut streak = ErrorStreak::default();
        let mut step = self.send_goal(key).await;
        let mut fallback = tokio::time::interval_at(
            tokio::time::Instant::now() + FALLBACK_INTERVAL,
            FALLBACK_INTERVAL,
        );
        fallback.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            if let Err(error) = step {
                let exhausted = streak.record(Instant::now());
                tracing::warn!(lane = key, errors = streak.count, %error, "external session step failed");
                let not_running = matches!(&error, OmonError::Config(message) if *message == ControlError::NotRunning.to_string());
                if not_running || exhausted {
                    return Err(error);
                }
            } else {
                streak.clear();
            }
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
            step = async {
                self.send_goal(key).await?;
                self.read_new(key).await?;
                self.send_pending(key).await
            }
            .await;
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
        tx.commit().await?;
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
                .execute(&self.pool)
                .await?;
        }
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
                let output = match control.output_with_timeout(target, &command, REMOTE_COMMAND_TIMEOUT).await {
                    Ok(output) => output,
                    Err(error) => { reasons.push(format!("remote command unavailable: {error}")); continue; }
                };
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
        let output = match control.output(target, &argv).await {
            Ok(output) => output,
            Err(error) => {
                reasons.push(format!("remote file check unavailable: {error}"));
                continue;
            }
        };
        if output.code != 0 {
            reasons.push(format!(
                "remote file check failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }
    let script = "import os,subprocess,sys; os.chdir(sys.argv[1])\ndef git(path,*args):\n r=subprocess.run(['git','-C',path,*args],capture_output=True,text=True)\n if r.returncode: raise RuntimeError(r.stderr)\n return r.stdout\np=subprocess.run(['git','rev-parse','--is-inside-work-tree'],capture_output=True,text=True)\nif p.returncode:\n if 'not a git repository' in p.stderr: sys.exit(0)\n raise RuntimeError(p.stderr)\nstart=float(sys.argv[2]) if len(sys.argv)>2 and sys.argv[2] else None\ncwd=os.getcwd()\ndef include(path):\n if path==cwd or path.startswith(cwd+os.sep): return True\n if not os.path.exists(path): return False\n st=os.stat(path)\n return start is None or getattr(st,'st_birthtime',st.st_ctime)>=start\npaths=[cwd]+[l[9:] for l in git(cwd,'worktree','list','--porcelain').splitlines() if l.startswith('worktree ')]\nfor path in dict.fromkeys(paths):\n if not include(path): continue\n if git(path,'status','--porcelain','--untracked-files=no').strip(): raise RuntimeError('uncommitted '+path)\n if git(path,'remote').strip() and git(path,'log','HEAD','--not','--remotes','--oneline').strip(): raise RuntimeError('unpushed '+path)";
    let output = control
        .output(
            target,
            &[
                "python3".into(),
                "-c".into(),
                script.into(),
                lane.cwd.clone(),
                // Linked worktrees outside the cwd count only if created after the lane started.
                lane.created_at
                    .map(|at| format!("{:.3}", at.timestamp_millis() as f64 / 1000.0))
                    .unwrap_or_default(),
            ],
        )
        .await;
    match output {
        Ok(output) if output.code != 0 => reasons.push(format!(
            "remote worktree check failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Ok(_) => {}
        Err(error) => reasons.push(format!("remote worktree check unavailable: {error}")),
    }
    Ok(reasons)
}

fn next_cursor(value: &Value) -> Option<String> {
    let items = value
        .get("items")
        .or_else(|| value.get("messages"))
        .or_else(|| value.get("entries"))
        .and_then(Value::as_array)
        .or_else(|| value.as_array());
    let seqs: Vec<i64> = items
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.get("seq").and_then(|s| {
                s.as_i64()
                    .or_else(|| s.as_str().and_then(|text| text.parse::<i64>().ok()))
            })
        })
        .collect();
    // A seq-numbered transcript is tracked by its highest seq, never by the reader's opaque
    // paging cursor; an empty transcript starts at 0 so a fresh session can be adopted.
    if let Some(max) = seqs.iter().max() {
        return Some(max.to_string());
    }
    let opaque = value
        .get("nextCursor")
        .or_else(|| value.get("next_cursor"))
        .or_else(|| value.get("cursor"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    match (opaque, items) {
        (Some(cursor), _) => Some(cursor),
        (None, Some(items)) if items.is_empty() && value.get("kind").is_some() => Some("0".into()),
        _ => None,
    }
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
        let Some(text) = entry_text(entry) else {
            // Tool-call-only or otherwise non-text entries carry nothing to capture; skipping
            // them lets the cursor advance instead of re-reading the same page forever.
            continue;
        };
        let id = entry
            .get("id")
            .or_else(|| entry.get("itemId"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                entry.get("seq").and_then(|s| {
                    s.as_i64()
                        .map(|seq| seq.to_string())
                        .or_else(|| s.as_str().map(str::to_owned))
                })
            })
            .ok_or_else(|| invalid("assistant message has no stable id"))?;
        items.push((id, text));
    }
    Ok(items)
}

fn extract_blocks_text(blocks: &[Value]) -> Option<String> {
    let parts: Vec<&str> = blocks
        .iter()
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect();
    (!parts.is_empty()).then(|| parts.join("\n"))
}

fn entry_text(entry: &Value) -> Option<String> {
    let content = entry
        .get("text")
        .filter(|value| !value.is_null())
        .or_else(|| entry.get("content"))?;
    match content {
        Value::String(text) => {
            if let Ok(Value::Array(blocks)) = serde_json::from_str::<Value>(text) {
                if let Some(extracted) = extract_blocks_text(&blocks) {
                    return Some(extracted);
                }
            }
            Some(text.clone())
        }
        Value::Array(blocks) => extract_blocks_text(blocks),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn error_streak_needs_both_count_and_elapsed_time() {
        let start = Instant::now();
        let mut streak = ErrorStreak::default();
        // A burst of wake-ups during one outage must not exhaust the budget.
        for offset in 0..5 {
            assert!(!streak.record(start + Duration::from_secs(offset)));
        }
        assert!(streak.record(start + ERROR_WINDOW));
        streak.clear();
        assert!(!streak.record(start + ERROR_WINDOW * 2));
        assert_eq!(streak.count, 1);
    }

    #[test]
    fn non_text_assistant_entries_are_skipped_not_fatal() {
        let page = json!({"messages":[
            {"id":"tool","role":"assistant","text":null},
            {"id":"tool-2","role":"assistant","content":[{"type":"tool_use","name":"x"}]},
            {"id":"blocks","role":"assistant","content":[{"type":"text","text":"a"},{"type":"text","text":"b"}]},
            {"id":"user","role":"user","text":"ignored"},
            {"id":"plain","role":"assistant","text":"done"}
        ]});
        assert_eq!(
            assistant_items(&page).unwrap(),
            vec![
                ("blocks".to_owned(), "a\nb".to_owned()),
                ("plain".to_owned(), "done".to_owned())
            ]
        );
    }

    #[test]
    fn next_cursor_prefers_highest_seq_over_paging_cursor() {
        let page_with_next_cursor = json!({
            "items": [
                {"seq": 10, "role": "assistant", "content": "hello"},
                {"seq": 15, "role": "assistant", "content": "world"}
            ],
            "next_cursor": "cursor-abc"
        });
        assert_eq!(next_cursor(&page_with_next_cursor).as_deref(), Some("15"));

        let page_with_camel_case = json!({
            "messages": [
                {"id": "m1", "role": "assistant", "content": "hello"}
            ],
            "nextCursor": "cursor-xyz"
        });
        assert_eq!(
            next_cursor(&page_with_camel_case).as_deref(),
            Some("cursor-xyz")
        );

        let page_seq_only = json!({
            "kind": "thread_read",
            "thread_id": "t1",
            "items": [
                {"seq": 12, "role": "user", "content": "q"},
                {"seq": 18, "role": "assistant", "content": "a"},
                {"seq": 15, "role": "assistant", "content": "b"}
            ],
            "truncated": false
        });
        assert_eq!(next_cursor(&page_seq_only).as_deref(), Some("18"));

        let empty_page = json!({
            "items": []
        });
        assert_eq!(next_cursor(&empty_page), None);

        let empty_thread =
            json!({"kind": "ok", "thread_id": "t1", "items": [], "truncated": false});
        assert_eq!(next_cursor(&empty_thread).as_deref(), Some("0"));
    }

    #[test]
    fn assistant_items_accepts_seq_and_json_encoded_blocks() {
        let json_blocks_str = serde_json::to_string(&json!([
            {"type": "text", "text": "first line"},
            {"type": "tool_use", "id": "t1"},
            {"type": "text", "text": "second line"}
        ]))
        .unwrap();

        let page = json!({
            "items": [
                {"seq": 42, "role": "assistant", "content": json_blocks_str},
                {"seq": "43", "role": "assistant", "content": "simple string"},
                {"seq": 44, "role": "assistant", "content": [{"type": "text", "text": "array block"}]},
                {"seq": 45, "role": "user", "content": "user query"}
            ]
        });

        assert_eq!(
            assistant_items(&page).unwrap(),
            vec![
                ("42".to_owned(), "first line\nsecond line".to_owned()),
                ("43".to_owned(), "simple string".to_owned()),
                ("44".to_owned(), "array block".to_owned())
            ]
        );
    }
}
