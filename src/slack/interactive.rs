use serde_json::Value;
use uuid::Uuid;

use super::{SlackApi, SlackConfig, SlackEgress};
use crate::{parse_custom_id, ApprovalDecision, Result, SmartApprovalGuard};

const APPROVAL_PREFIX: &str = "omon:approval:";
const NOT_AUTHORIZED: &str = "You are not authorized to answer this approval request.";
const STALE_APPROVAL: &str =
    "이 승인 요청은 이미 처리됐거나 만료됐어요. 다시 누르지 않으셔도 돼요.";

pub async fn handle_block_actions(
    payload: &Value,
    config: &SlackConfig,
    approvals: &SmartApprovalGuard,
    egress: &SlackEgress,
    api: &SlackApi,
) -> Result<()> {
    for (request_id, label) in resolve_approvals(payload, config, approvals, api).await? {
        egress.mark_approval_decided(request_id, &label).await?;
    }
    super::buttons::handle_button_actions(payload, config, api).await
}

async fn resolve_approvals(
    payload: &Value,
    config: &SlackConfig,
    approvals: &SmartApprovalGuard,
    api: &SlackApi,
) -> Result<Vec<(Uuid, String)>> {
    if payload.get("type").and_then(Value::as_str) != Some("block_actions") {
        return Ok(Vec::new());
    }
    let values = approval_values(payload);
    if values.is_empty() {
        return Ok(Vec::new());
    }
    let user = payload
        .pointer("/user/id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if user.is_empty() || user != config.owner_id() || !config.is_allowed(user) {
        tracing::warn!(user, "rejected Slack approval click from unauthorized user");
        if let Some(channel) = payload_channel(payload) {
            api.post_ephemeral(channel, user, NOT_AUTHORIZED).await?;
        }
        return Ok(Vec::new());
    }
    let mut decided = Vec::new();
    for value in values {
        let Some((request_id, decision)) = parse_custom_id(value) else {
            tracing::warn!(value, "ignoring malformed Slack approval value");
            continue;
        };
        if approvals.resolve_custom_id(value).await {
            decided.push((request_id, decision_label(&decision, user)));
        } else {
            tracing::info!(%request_id, "Slack approval click for an unknown or expired request");
            // The card still shows buttons that can no longer do anything (decided elsewhere,
            // expired, or lost to a restart): replace them so the owner is not left clicking.
            if let (Some(channel), Some(ts)) =
                (payload_channel(payload), payload_message_ts(payload))
            {
                if let Err(error) = api
                    .update_message(channel, ts, STALE_APPROVAL, Some(stale_blocks()))
                    .await
                {
                    tracing::warn!(%request_id, %error, "failed to clear a stale Slack approval card");
                }
            }
        }
    }
    Ok(decided)
}

fn approval_values(payload: &Value) -> Vec<&str> {
    payload
        .get("actions")
        .and_then(Value::as_array)
        .map(|actions| {
            actions
                .iter()
                .filter_map(|action| action.get("value").and_then(Value::as_str))
                .filter(|value| value.starts_with(APPROVAL_PREFIX))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn payload_channel(payload: &Value) -> Option<&str> {
    payload
        .pointer("/channel/id")
        .or_else(|| payload.pointer("/container/channel_id"))
        .and_then(Value::as_str)
}

fn payload_message_ts(payload: &Value) -> Option<&str> {
    payload
        .pointer("/container/message_ts")
        .or_else(|| payload.pointer("/message/ts"))
        .and_then(Value::as_str)
}

fn stale_blocks() -> Value {
    serde_json::json!([{ "type": "section", "text": { "type": "mrkdwn", "text": STALE_APPROVAL } }])
}

fn decision_label(decision: &ApprovalDecision, user: &str) -> String {
    let label = match decision {
        ApprovalDecision::Once => "✅ Approved once",
        ApprovalDecision::Session => "✅ Approved for this session",
        ApprovalDecision::Always => "✅ Always approved",
        ApprovalDecision::Deny { .. } => "❌ Denied",
    };
    format!("{label} by <@{user}>")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::extract::Request;
    use axum::response::Response;
    use parking_lot::Mutex;
    use serde_json::json;

    use super::*;

    const OWNER: &str = "UOWNER";

    fn config() -> SlackConfig {
        SlackConfig {
            allowed_users: vec![OWNER.into()],
            ..SlackConfig::test_default()
        }
    }

    fn click(user: &str, values: &[String]) -> Value {
        json!({
            "type": "block_actions",
            "user": { "id": user },
            "channel": { "id": "C1" },
            "container": { "channel_id": "C1", "message_ts": "1.2" },
            "actions": values
                .iter()
                .map(|value| json!({ "action_id": "approval", "value": value }))
                .collect::<Vec<_>>(),
        })
    }

    async fn fake_slack() -> (SlackApi, Arc<Mutex<Vec<(String, Value)>>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let recorded = calls.clone();
        let app = axum::Router::new().fallback(move |req: Request| {
            let recorded = recorded.clone();
            async move {
                let path = req.uri().path().to_owned();
                let body = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap();
                recorded
                    .lock()
                    .push((path, serde_json::from_slice(&body).unwrap_or(Value::Null)));
                Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "ok": true }).to_string()))
                    .unwrap()
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        (
            SlackApi::with_base_url("xoxb-test".into(), format!("{base}/api")),
            calls,
        )
    }

    #[test]
    fn approval_values_keep_only_approval_buttons() {
        let id = Uuid::new_v4();
        let payload = json!({
            "actions": [
                { "value": format!("omon:approval:{id}:once") },
                { "value": "something:else" },
                { "action_id": "no-value" },
            ],
        });
        assert_eq!(
            approval_values(&payload),
            vec![format!("omon:approval:{id}:once")]
        );
        assert!(approval_values(&json!({})).is_empty());
    }

    #[test]
    fn decision_labels_cover_every_choice() {
        assert_eq!(
            decision_label(&ApprovalDecision::Once, "U1"),
            "✅ Approved once by <@U1>"
        );
        assert_eq!(
            decision_label(&ApprovalDecision::Session, "U1"),
            "✅ Approved for this session by <@U1>"
        );
        assert_eq!(
            decision_label(&ApprovalDecision::Always, "U1"),
            "✅ Always approved by <@U1>"
        );
        assert_eq!(
            decision_label(&ApprovalDecision::deny(None), "U2"),
            "❌ Denied by <@U2>"
        );
    }

    #[test]
    fn payload_channel_falls_back_to_container() {
        assert_eq!(
            payload_channel(&json!({ "channel": { "id": "C9" } })),
            Some("C9")
        );
        assert_eq!(
            payload_channel(&json!({ "container": { "channel_id": "D3" } })),
            Some("D3")
        );
        assert_eq!(payload_channel(&json!({})), None);
    }

    #[tokio::test]
    async fn owner_click_resolves_pending_approval() {
        let guard = SmartApprovalGuard::new();
        let prompt = guard.request().await;
        let request_id = prompt.request_id;
        let (api, calls) = fake_slack().await;

        let payload = click(OWNER, &[format!("omon:approval:{request_id}:session")]);
        let decided = resolve_approvals(&payload, &config(), &guard, &api)
            .await
            .unwrap();

        assert_eq!(
            decided,
            vec![(
                request_id,
                format!("✅ Approved for this session by <@{OWNER}>")
            )]
        );
        assert_eq!(guard.pending_count().await, 0);
        assert!(calls.lock().is_empty());

        let again = resolve_approvals(&payload, &config(), &guard, &api)
            .await
            .unwrap();
        assert!(again.is_empty());
        drop(prompt);
    }

    #[tokio::test]
    async fn unauthorized_click_posts_ephemeral_and_keeps_request_pending() {
        let guard = SmartApprovalGuard::new();
        let prompt = guard.request().await;
        let request_id = prompt.request_id;
        let (api, calls) = fake_slack().await;

        let payload = click("USTRANGER", &[format!("omon:approval:{request_id}:once")]);
        let decided = resolve_approvals(&payload, &config(), &guard, &api)
            .await
            .unwrap();

        assert!(decided.is_empty());
        assert_eq!(guard.pending_count().await, 1);
        let calls = calls.lock().clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "/api/chat.postEphemeral");
        assert_eq!(calls[0].1["user"], "USTRANGER");
        assert_eq!(calls[0].1["channel"], "C1");
        assert_eq!(calls[0].1["text"], NOT_AUTHORIZED);
        drop(prompt);
    }

    #[tokio::test]
    async fn malformed_unknown_and_non_approval_actions_are_ignored() {
        let guard = SmartApprovalGuard::new();
        let (api, calls) = fake_slack().await;
        let unknown = Uuid::new_v4();
        let payload = click(
            OWNER,
            &[
                "omon:approval:not-a-uuid:once".to_owned(),
                format!("omon:approval:{unknown}:bogus"),
                format!("omon:approval:{unknown}:deny"),
            ],
        );
        assert!(resolve_approvals(&payload, &config(), &guard, &api)
            .await
            .unwrap()
            .is_empty());
        // The well-formed but unknown click gets its dead buttons replaced; nothing else is sent.
        {
            let calls = calls.lock();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].0, "/api/chat.update");
            assert_eq!(calls[0].1["ts"], "1.2");
            assert_eq!(calls[0].1["text"], STALE_APPROVAL);
        }
        calls.lock().clear();

        let mut other = click("USTRANGER", &["other:button".to_owned()]);
        assert!(resolve_approvals(&other, &config(), &guard, &api)
            .await
            .unwrap()
            .is_empty());
        other["type"] = json!("view_submission");
        assert!(resolve_approvals(&other, &config(), &guard, &api)
            .await
            .unwrap()
            .is_empty());
        assert!(calls.lock().is_empty());
    }
}
