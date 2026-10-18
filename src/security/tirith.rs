use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const DEFAULT_TIRITH_TIMEOUT_SECS: u64 = 5;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScannerVerdict {
    Allow,
    Warn { reason: String },
    Block { reason: String },
    Deny { reason: String },
}

impl ScannerVerdict {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }
}

#[derive(Clone, Debug)]
pub struct TirithScanner {
    pub url: String,
    pub fail_open: bool,
    pub timeout: Duration,
    http: reqwest::Client,
}

impl TirithScanner {
    pub fn new(url: impl Into<String>, fail_open: bool, timeout: Duration) -> Self {
        Self {
            url: url.into(),
            fail_open,
            timeout,
            http: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Evaluates scanner response JSON or error into an allow/deny verdict.
    pub fn evaluate_response(
        response_result: std::result::Result<&Value, &str>,
        fail_open: bool,
    ) -> ScannerVerdict {
        match response_result {
            Ok(json) => {
                let action = json
                    .get("action")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase();

                if matches!(action.as_str(), "warn" | "deny" | "block" | "reject") {
                    let summary = json
                        .get("summary")
                        .and_then(Value::as_str)
                        .filter(|s| !s.trim().is_empty())
                        .map(str::to_string)
                        .unwrap_or_else(|| {
                            if let Some(findings) = json.get("findings").and_then(Value::as_array) {
                                let parts: Vec<String> = findings
                                    .iter()
                                    .filter_map(|f| {
                                        let title = f.get("title").and_then(Value::as_str)?;
                                        let desc = f
                                            .get("description")
                                            .and_then(Value::as_str)
                                            .unwrap_or("");
                                        if desc.is_empty() {
                                            Some(title.to_string())
                                        } else {
                                            Some(format!("{title}: {desc}"))
                                        }
                                    })
                                    .collect();
                                if !parts.is_empty() {
                                    return parts.join("; ");
                                }
                            }
                            "semantic risk analysis flagged this command".to_string()
                        });
                    if action == "warn" {
                        ScannerVerdict::Warn { reason: summary }
                    } else {
                        ScannerVerdict::Block { reason: summary }
                    }
                } else if action == "allow" {
                    ScannerVerdict::Allow
                } else {
                    Self::evaluate_response(Err("missing or unknown scanner action"), fail_open)
                }
            }
            Err(err) => {
                if fail_open {
                    tracing::warn!(%err, "Tirith scanner call failed; failing open (command permitted)");
                    ScannerVerdict::Allow
                } else {
                    tracing::error!(%err, "Tirith scanner call failed; failing closed (command blocked)");
                    ScannerVerdict::Deny {
                        reason: format!("External security scanner unavailable: {err}"),
                    }
                }
            }
        }
    }

    /// POSTs the candidate command to the Tirith scanner and returns the verdict.
    pub async fn scan_command(&self, command: &str) -> ScannerVerdict {
        let payload = json!({
            "command": command,
        });

        let send_res = self.http.post(&self.url).json(&payload).send().await;

        match send_res {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    return Self::evaluate_response(
                        Err(&format!("HTTP status {status}")),
                        self.fail_open,
                    );
                }
                match resp.json::<Value>().await {
                    Ok(json_body) => Self::evaluate_response(Ok(&json_body), self.fail_open),
                    Err(err) => {
                        let err_msg = format!("failed to decode scanner response JSON: {err}");
                        Self::evaluate_response(Err(&err_msg), self.fail_open)
                    }
                }
            }
            Err(err) => {
                let err_msg = format!("network error: {err}");
                Self::evaluate_response(Err(&err_msg), self.fail_open)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn scanner_warning_and_invalid_payload_never_silently_pass() {
        use crate::tools::{ApprovalPolicy, TerminalTool};
        use crate::{ApprovalDecision, ApprovalError, ApprovalRequester, SessionKey, Tool};
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        struct Reject(AtomicUsize);
        #[async_trait::async_trait]
        impl ApprovalRequester for Reject {
            async fn request_approval(
                &self,
                _: &SessionKey,
                _: &str,
                _: &str,
            ) -> Result<ApprovalDecision, ApprovalError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(ApprovalDecision::Deny { reason: None })
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let app = axum::Router::new().route(
            "/",
            axum::routing::post(move |axum::Json(body): axum::Json<Value>| {
                let counter = counter.clone();
                async move {
                    assert!(body["command"].is_string());
                    let n = counter.fetch_add(1, Ordering::SeqCst);
                    axum::Json(if n == 0 {
                        json!({"action":"warn","findings":[{"rule_id":"x","title":"risk"}]})
                    } else {
                        json!({})
                    })
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    stopped.await.unwrap();
                })
                .await
                .unwrap();
        });
        let scanner = TirithScanner::new(url, false, Duration::from_secs(5));
        let dir = tempfile::tempdir().unwrap();
        let reject = Arc::new(Reject(AtomicUsize::new(0)));
        let session = SessionKey::new("discord", None::<String>, "u05", None::<String>, "user");
        let tool = TerminalTool::new(dir.path())
            .with_external_scanner(scanner.clone())
            .with_approval(
                ApprovalPolicy::Smart,
                reject.clone(),
                Duration::from_secs(5),
            );
        let args = json!({"program":"echo","args":["u05"]});
        let warning = tool
            .execute_with_context(args.clone(), Some(&session))
            .await;
        let invalid = tool
            .execute_with_context(args.clone(), Some(&session))
            .await;
        let prompts = reject.0.load(Ordering::SeqCst);
        println!(
            "warning={warning:?}; invalid={invalid:?}; prompts={prompts}; executions={}",
            usize::from(warning.is_ok()) + usize::from(invalid.is_ok())
        );
        let never = TerminalTool::new(dir.path())
            .with_external_scanner(scanner)
            .with_approval_policy(ApprovalPolicy::Never)
            .with_deny_globs(vec!["npm publish *".into()]);
        let bypass = never.execute(args).await;
        let hardline = never
            .execute(json!({"program":"sh","args":["-c","reboot"]}))
            .await;
        let denied = never
            .execute(json!({"program":"npm","args":["publish","--access","public"]}))
            .await;
        let requests = calls.load(Ordering::SeqCst);
        println!("Never={bypass:?}; hardline={hardline:?}; user_deny={denied:?}; scanner_requests={requests}");
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        dir.close().unwrap();
        assert!(
            warning.is_err()
                && invalid.is_err()
                && prompts == 1
                && bypass.is_ok()
                && hardline.is_err()
                && denied.is_err()
                && requests == 2
        );
    }

    #[tokio::test]
    async fn scanner_caps_precede_persistence() {
        use crate::tools::{ApprovalPolicy, TerminalTool};
        use crate::{
            Database, DiscordApprovalRequester, OutboundAction, OutboundDispatcher, SessionKey,
            SmartApprovalGuard, Tool,
        };
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        struct Resolver(SmartApprovalGuard, Arc<AtomicUsize>);
        #[async_trait::async_trait]
        impl OutboundDispatcher for Resolver {
            async fn dispatch(&self, action: OutboundAction) -> crate::Result<()> {
                if let OutboundAction::ApprovalRequest { request_id, .. } = action {
                    self.1.fetch_add(1, Ordering::SeqCst);
                    assert!(
                        self.0
                            .resolve_custom_id(&format!("omon:approval:{request_id}:always"))
                            .await
                    );
                }
                Ok(())
            }
        }
        let mut failures = Vec::new();
        for action in ["warn", "block"] {
            for combined in [false, true] {
                let calls = Arc::new(AtomicUsize::new(0));
                let count = calls.clone();
                let app = axum::Router::new().route("/", axum::routing::post(move |axum::Json(body): axum::Json<Value>| {
                    let count = count.clone();
                    async move {
                        assert!(body["command"].is_string());
                        count.fetch_add(1, Ordering::SeqCst);
                        axum::Json(json!({"action":action,"findings":[{"rule_id":"x","title":"risk"}]}))
                    }
                }));
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let url = format!("http://{}/", listener.local_addr().unwrap());
                let (stop, stopped) = tokio::sync::oneshot::channel();
                let server = tokio::spawn(async move {
                    axum::serve(listener, app)
                        .with_graceful_shutdown(async {
                            stopped.await.unwrap();
                        })
                        .await
                        .unwrap();
                });
                let db = Database::connect("sqlite::memory:").await.unwrap();
                let guard = SmartApprovalGuard::new().with_pool(db.pool().clone());
                let prompts = Arc::new(AtomicUsize::new(0));
                let requester = Arc::new(DiscordApprovalRequester::new(
                    guard.clone(),
                    Duration::from_secs(5),
                ));
                requester
                    .set_dispatcher(Arc::new(Resolver(guard.clone(), prompts.clone())))
                    .await;
                let dir = tempfile::tempdir().unwrap();
                let tool = TerminalTool::new(dir.path())
                    .with_external_scanner(TirithScanner::new(url, false, Duration::from_secs(5)))
                    .with_approval(ApprovalPolicy::Smart, requester, Duration::from_secs(5));
                let a = SessionKey::new("discord", None::<String>, "a", None::<String>, "u");
                let b = SessionKey::new("discord", None::<String>, "b", None::<String>, "u");
                let args = if combined {
                    json!({"program":"sh","args":["-c","echo u05"]})
                } else {
                    json!({"program":"echo","args":["u05"]})
                };
                let ordinary_key = crate::security::derive_pattern_key(if combined {
                    "sh -c 'echo u05'"
                } else {
                    "echo u05"
                });
                guard.load_permanent([ordinary_key]).await;
                let mut executions = 0;
                for session in [&a, &a, &b] {
                    if let Ok(output) = tool.execute_with_context(args.clone(), Some(session)).await
                    {
                        assert_eq!(output["stdout"], "u05\n");
                        executions += 1;
                    }
                }
                let persisted: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM action_precedents WHERE origin != 'legacy_allowlist'",
                )
                .fetch_one(db.pool())
                .await
                .unwrap();
                let prompt_count = prompts.load(Ordering::SeqCst);
                println!("action={action} combined={combined} prompts={prompt_count} executions={executions} persisted={persisted} HTTP={}", calls.load(Ordering::SeqCst));
                if executions != 3
                    || persisted != 0
                    || prompt_count != if action == "warn" { 2 } else { 3 }
                {
                    failures.push((action, combined));
                }
                assert_eq!(calls.load(Ordering::SeqCst), 3);
                assert_eq!(guard.pending_count().await, 0);
                stop.send(()).unwrap();
                tokio::time::timeout(Duration::from_secs(5), server)
                    .await
                    .unwrap()
                    .unwrap();
                db.pool().close().await;
                dir.close().unwrap();
            }
        }
        assert!(failures.is_empty(), "scope failures: {failures:?}");
    }

    #[test]
    fn test_scanner_verdict_allow() {
        let payload = json!({
            "action": "allow",
            "summary": "safe command"
        });
        let verdict = TirithScanner::evaluate_response(Ok(&payload), true);
        assert_eq!(verdict, ScannerVerdict::Allow);
        assert!(verdict.is_allowed());

        let verdict_fail_closed = TirithScanner::evaluate_response(Ok(&payload), false);
        assert_eq!(verdict_fail_closed, ScannerVerdict::Allow);
    }

    #[test]
    fn test_scanner_verdict_deny_and_block() {
        let payload_deny = json!({
            "action": "deny",
            "summary": "reverse shell payload detected"
        });
        let verdict_deny = TirithScanner::evaluate_response(Ok(&payload_deny), true);
        assert_eq!(
            verdict_deny,
            ScannerVerdict::Block {
                reason: "reverse shell payload detected".to_string()
            }
        );
        assert!(!verdict_deny.is_allowed());

        let payload_findings = json!({
            "action": "block",
            "findings": [
                {"title": "Exfiltration", "description": "curls sensitive tokens"}
            ]
        });
        let verdict_block = TirithScanner::evaluate_response(Ok(&payload_findings), false);
        assert!(matches!(verdict_block, ScannerVerdict::Block { .. }));
        if let ScannerVerdict::Block { reason } = verdict_block {
            assert!(reason.contains("Exfiltration: curls sensitive tokens"));
        }
    }

    #[test]
    fn scanner_schema_actions_follow_failure_policy() {
        for payload in [
            json!({}),
            json!({"action":"unknown"}),
            json!({"action":null}),
            json!([]),
        ] {
            assert_eq!(
                TirithScanner::evaluate_response(Ok(&payload), true),
                ScannerVerdict::Allow
            );
            assert!(matches!(
                TirithScanner::evaluate_response(Ok(&payload), false),
                ScannerVerdict::Deny { .. }
            ));
        }
        for action in ["deny", "block", "reject"] {
            assert!(matches!(
                TirithScanner::evaluate_response(Ok(&json!({"action":action})), false),
                ScannerVerdict::Block { .. }
            ));
        }
        assert!(matches!(
            TirithScanner::evaluate_response(Ok(&json!({"action":"warn"})), false),
            ScannerVerdict::Warn { .. }
        ));
    }

    #[test]
    fn test_scanner_fail_open_vs_fail_closed_on_error() {
        // 1. Fail open: network/timeout error -> Allow
        let verdict_open = TirithScanner::evaluate_response(Err("connection timed out"), true);
        assert_eq!(verdict_open, ScannerVerdict::Allow);

        // 2. Fail closed: network/timeout error -> Deny
        let verdict_closed = TirithScanner::evaluate_response(Err("connection timed out"), false);
        assert!(matches!(verdict_closed, ScannerVerdict::Deny { .. }));
        if let ScannerVerdict::Deny { reason } = verdict_closed {
            assert!(reason.contains("connection timed out"));
        }
    }
}
