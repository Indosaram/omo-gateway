use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::judge::{Judge, Purpose};
use crate::storage::{take_undelivered_events, LaneEvent};
use crate::{EventOrigin, InboundEvent, Result, SessionKey, SessionMultiplexer};

/// Local queue scan only; judgment is requested only for an eligible new event.
pub struct EventDispatcher {
    pool: SqlitePool,
    multiplexer: SessionMultiplexer,
    judge: Arc<dyn Judge>,
}

impl EventDispatcher {
    pub fn new(pool: SqlitePool, multiplexer: SessionMultiplexer, judge: Arc<dyn Judge>) -> Self {
        Self {
            pool,
            multiplexer,
            judge,
        }
    }

    pub async fn serve(self, shutdown: CancellationToken) -> Result<()> {
        let mut ticks = tokio::time::interval(Duration::from_secs(5));
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => return Ok(()),
                _ = ticks.tick() => {}
            }
            // Finish an admission/receipt pair before observing cancellation.
            if let Err(error) = self.tick(Utc::now()).await {
                tracing::warn!(%error, "lane event dispatch tick failed; events remain queued");
            }
        }
    }

    /// The supplied clock makes batching and rate limiting deterministic in tests.
    pub async fn tick(&self, now: DateTime<Utc>) -> Result<()> {
        if self.multiplexer.is_draining() {
            return Ok(());
        }
        let mut groups = BTreeMap::<String, Vec<LaneEvent>>::new();
        for event in take_undelivered_events(&self.pool, None).await? {
            if event.kind == "cron_problem" {
                // One bad cron problem must not block every other lead's events.
                match crate::cron::remediate::handle_event(&event.body, &event.lead_session_key)
                    .await
                {
                    Ok(true) => {
                        self.receipt(&[event.id], now, "cron_remediation").await?;
                        continue;
                    }
                    Ok(false) => {}
                    Err(error) => {
                        tracing::warn!(event_id = event.id, %error, "cron problem handling failed; event remains queued");
                        continue;
                    }
                }
            }
            let target = if owner_dm_worthy(&event) {
                event.lead_session_key.clone()
            } else {
                self.report_target(&event.lead_session_key).await
            };
            groups.entry(target).or_default().push(event);
        }
        for (lead, events) in groups {
            if let Err(error) = self.deliver_group(&lead, events, now).await {
                tracing::warn!(lead_session_key = %lead, %error, "lane events remain queued for retry");
            }
        }
        Ok(())
    }

    async fn deliver_group(
        &self,
        lead: &str,
        events: Vec<LaneEvent>,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let Some(oldest) = events.iter().map(|event| event.created_at).min() else {
            return Ok(());
        };
        if now.signed_duration_since(oldest).num_seconds() < 30
            && !events.iter().any(|event| {
                matches!(
                    event.kind.as_str(),
                    "done_claimed" | "dead" | "kibitzer" | "cron_report"
                )
            })
        {
            return Ok(());
        }
        let last: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT MAX(delivered_at) FROM lane_events WHERE lead_session_key = ? AND delivered_at IS NOT NULL AND (delivery_reason IS NULL OR delivery_reason <> 'jev_skip')",
        ).bind(lead).fetch_one(&self.pool).await?;
        if last.is_some_and(|last| now.signed_duration_since(last).num_seconds() < 30) {
            return Ok(());
        }
        let session = SessionKey::from_storage_key(lead)?;
        if !self.multiplexer.session_is_idle(&session).await? {
            return Ok(());
        }

        // Keep every source ID for receipts, but judge and render only the newest body.
        let mut collapsed = BTreeMap::<(Option<String>, String), (LaneEvent, Vec<i64>)>::new();
        for event in events {
            let entry = collapsed
                .entry((event.lane_key.clone(), event.kind.clone()))
                .or_insert_with(|| (event.clone(), Vec::new()));
            entry.1.push(event.id);
            if (event.created_at, event.id) >= (entry.0.created_at, entry.0.id) {
                entry.0 = event;
            }
        }
        let mut selected = Vec::new();
        for (event, ids) in collapsed.into_values() {
            if !critical(&event) {
                let state = crate::judge::scrub_for_external(&format!(
                    "{} {}: {}",
                    event.lane_key.as_deref().unwrap_or("-"),
                    event.kind,
                    event.body,
                ));
                let score = self
                    .judge
                    .noul(
                        Purpose::EventWake,
                        &state,
                        "Does this event need the lead agent's judgment now?",
                    )
                    .await;
                match score {
                    Ok(score) if score < 0.5 => {
                        if self.multiplexer.is_draining() {
                            return Ok(());
                        }
                        self.receipt(&ids, now, "jev_skip").await?;
                        continue;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!(%error, event_id = event.id, "event wake judgment unavailable; retaining event");
                        continue;
                    }
                }
            }
            selected.push((event, ids));
        }
        selected.sort_by_key(|(event, _)| event.id);
        let ids: Vec<i64> = selected
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect();
        let (Some(first), Some(last)) = (ids.iter().min(), ids.iter().max()) else {
            return Ok(());
        };
        let lines = selected
            .iter()
            .map(|(event, _)| {
                let body: String = event
                    .body
                    .chars()
                    .take(600)
                    .map(|ch| if ch == '\n' || ch == '\r' { ' ' } else { ch })
                    .collect();
                format!(
                    "{} {}: {}",
                    event.lane_key.as_deref().unwrap_or("-"),
                    event.kind,
                    body
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let event = InboundEvent {
            id: Uuid::new_v4(),
            session,
            platform_message_id: format!("lane-event:{first}-{last}"),
            delivery_id: None,
            referenced_message_id: None,
            author_id: None,
            origin: EventOrigin::System,
            content: format!("[LANE_EVENT]\n{lines}"),
            attachments: vec![],
            received_at: now,
        };
        // System admission is atomic at the actor: a user turn that won the race
        // after the idle probe makes route refuse rather than queue this turn.
        let inbound_id = event.id;
        self.multiplexer.route(event).await?;
        tracing::info!(lead_session_key = %lead, first_event_id = first, last_event_id = last, system_inbound_id = %inbound_id, "lane event admitted to lead turn");
        self.receipt(&ids, now, "lead_turn").await
    }

    /// The home channel session that takes routine events for an owner-DM lead, or the lead
    /// itself when there is none.
    async fn report_target(&self, lead: &str) -> String {
        let Ok(key) = SessionKey::from_storage_key(lead) else {
            return lead.to_owned();
        };
        match crate::slack::home::report_session(&self.pool, &key).await {
            Ok(Some(report)) => report.storage_key(),
            Ok(None) => lead.to_owned(),
            Err(error) => {
                tracing::warn!(%error, lead, "home channel lookup failed; event stays on the lead");
                lead.to_owned()
            }
        }
    }

    async fn receipt(&self, ids: &[i64], now: DateTime<Utc>, reason: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for id in ids {
            sqlx::query("UPDATE lane_events SET delivered_at = ?, delivery_reason = ? WHERE id = ? AND delivered_at IS NULL")
                .bind(now).bind(reason).bind(id).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

/// Events urgent enough to wake a turn in the owner DM itself: something the owner must act
/// on soon, or a failure only he can fix. Everything else (action-loop items, cron reports,
/// finished lanes, load alerts) goes to the Slack home channel (2026-10-21: "DM으로는 급한
/// 용무만").
fn owner_dm_worthy(event: &LaneEvent) -> bool {
    match event.kind.as_str() {
        "inbox_urgent" | "owner_reply_failed" | "dead" | "launch_failed" | "startup_error" => true,
        "cron_problem" => event.body.contains("needs_owner"),
        _ => false,
    }
}

fn critical(event: &LaneEvent) -> bool {
    match event.kind.as_str() {
        // These carry the only record of something the owner must act on: where finished work
        // went, or why a lane failed to launch, so they must reach the owner without passing
        // the wake judgment that can skip an event.
        "done_claimed" | "dead" | "not_done" | "inbox_urgent" | "owner_reply_failed"
        | "cron_report" | "unmerged_branch" | "launch_failed" | "startup_error" => true,
        // Triage already chose these items for action; a wake skip ends them as failed before
        // the lead sees them (2026-10-20 11:18Z and 11:30Z: 11 items, no lead turn).
        "action_loop" => true,
        // A lane that stopped making progress is parked until someone writes to it; skipped by
        // the wake judgment, it would sit idle with no one told.
        "no_progress" => true,
        "cron_problem" => event.body.contains("needs_owner"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: &str) -> LaneEvent {
        LaneEvent {
            id: 1,
            lane_key: Some("lane".into()),
            lead_session_key: "lead".into(),
            kind: kind.into(),
            body: "body".into(),
            created_at: Utc::now(),
            delivered_at: None,
        }
    }

    #[test]
    fn only_urgent_events_stay_in_the_owner_dm() {
        for kind in [
            "inbox_urgent",
            "owner_reply_failed",
            "dead",
            "launch_failed",
        ] {
            assert!(owner_dm_worthy(&event(kind)), "{kind}");
        }
        for kind in [
            "action_loop",
            "cron_report",
            "done_claimed",
            "guard",
            "close_failed",
            "question",
        ] {
            assert!(!owner_dm_worthy(&event(kind)), "{kind}");
        }
        let mut problem = event("cron_problem");
        assert!(!owner_dm_worthy(&problem));
        problem.body = "{\"kind\":\"needs_owner\"}".into();
        assert!(owner_dm_worthy(&problem));
    }

    #[test]
    fn owner_visible_failure_events_skip_the_wake_judgment() {
        // The branch name is the only pointer to work that left the lane, and a launch
        // failure is the only notice that a lane never ran, so a judge verdict must never
        // be able to drop either event.
        assert!(critical(&event("unmerged_branch")));
        assert!(critical(&event("launch_failed")));
        assert!(critical(&event("startup_error")));
        assert!(
            critical(&event("action_loop")),
            "triaged action items must reach the lead, not end as jev_skip failures"
        );
        assert!(critical(&event("no_progress")));
        assert!(!critical(&event("nudged")));
        assert!(
            !critical(&event("working_while_paused")),
            "output after a confirmed pause is routed like other lane events"
        );
    }
}
