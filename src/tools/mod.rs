mod browser;
mod cron;
mod file;
mod mcp;
mod message_context;
mod message_context_lazy;
pub mod message_context_slack;
mod skills;
mod terminal;
mod web;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use crate::{ApprovalDecision, ApprovalRequester, OmonError, SessionKey};

pub use browser::BrowserTool;
pub use cron::CronTool;
pub use file::FileTool;
pub use mcp::{McpClientTool, McpTool, McpTransport};
pub use message_context::{
    DiscordMessageContextApi, DiscordMessageContextProvider, MessageContextAttachment,
    MessageContextConversationMetadata, MessageContextMessage, MessageContextOperation,
    MessageContextPolicy, MessageContextProvider, MessageContextRequest, MessageContextResult,
    MessageContextTool, SerenityDiscordMessageContextApi,
};
use message_context_lazy::LazyDiscordMessageContextProvider;
pub use skills::SkillsTool;
pub use terminal::{
    augmented_path_from_environment, build_augmented_path, build_session_environment,
    ApprovalPolicy, TerminalTool, DEFAULT_EXTRA_PATH,
};
pub use web::{WebFetchTool, WebSearchTool};

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    async fn execute(&self, args: Value) -> Result<Value, OmonError>;

    fn requires_approval(&self, _args: &Value) -> Option<String> {
        None
    }

    async fn execute_with_context(
        &self,
        args: Value,
        _session: Option<&SessionKey>,
    ) -> Result<Value, OmonError> {
        self.execute(args).await
    }
}

#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    approval_requester: Option<Arc<dyn ApprovalRequester>>,
    approval_timeout: Duration,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self {
            tools: HashMap::new(),
            approval_requester: None,
            approval_timeout: Duration::from_secs(900),
        }
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        if let Some(provider) = LazyDiscordMessageContextProvider::from_environment() {
            registry.register(MessageContextTool::new().with_provider(provider));
        }
        registry
    }

    pub fn with_approval_requester(
        mut self,
        requester: Arc<dyn ApprovalRequester>,
        timeout: Duration,
    ) -> Self {
        self.approval_requester = Some(requester);
        self.approval_timeout = timeout;
        self
    }

    pub fn set_approval_requester(
        &mut self,
        requester: Arc<dyn ApprovalRequester>,
        timeout: Duration,
    ) {
        self.approval_requester = Some(requester);
        self.approval_timeout = timeout;
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) -> Option<Arc<dyn Tool>> {
        self.tools.insert(tool.name().to_owned(), Arc::new(tool))
    }

    pub fn register_arc(&mut self, tool: Arc<dyn Tool>) -> Option<Arc<dyn Tool>> {
        self.tools.insert(tool.name().to_owned(), tool)
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }

    pub async fn execute(&self, name: &str, args: Value) -> Result<Value, OmonError> {
        self.execute_with_context(name, args, None).await
    }

    pub async fn execute_with_context(
        &self,
        name: &str,
        args: Value,
        session: Option<&SessionKey>,
    ) -> Result<Value, OmonError> {
        let tool = self
            .get(name)
            .ok_or_else(|| OmonError::ToolExecution(format!("unknown tool: {name}")))?;

        if let Some(reason) = tool.requires_approval(&args) {
            let session = session.ok_or_else(|| {
                OmonError::Approval(format!(
                    "Tool '{name}' requires interactive approval ({reason}), but no session is available"
                ))
            })?;
            let requester = self.approval_requester.as_ref().ok_or_else(|| {
                OmonError::Approval(format!(
                    "Tool '{name}' requires interactive approval ({reason}), but no approval guard is configured"
                ))
            })?;

            if !requester.is_yolo(session).await {
                use sha2::{Digest, Sha256};

                let display_target = format!("<{name}>");
                // Length-prefix the name to keep the rule identity unambiguous.
                // Policy identity must not be inferred from the display target.
                let mut digest = Sha256::new();
                digest.update((name.len() as u64).to_be_bytes());
                digest.update(name.as_bytes());
                digest.update(reason.as_bytes());
                let pattern_key = format!("tool-rule:v1:{:x}", digest.finalize());
                let decision = tokio::time::timeout(
                    self.approval_timeout,
                    requester.request_approval_scoped(
                        session,
                        &display_target,
                        &reason,
                        &pattern_key,
                    ),
                )
                .await
                .map_err(|_| OmonError::Approval("approval request timed out".into()))?
                .map_err(|error| OmonError::Approval(error.to_string()))?;

                match decision {
                    ApprovalDecision::Once
                    | ApprovalDecision::Session
                    | ApprovalDecision::Always => {}
                    ApprovalDecision::Deny { reason } => {
                        let msg = match reason {
                            Some(r) if !r.trim().is_empty() => {
                                format!("Tool execution denied by user: {r}")
                            }
                            _ => "Tool execution denied by user".to_string(),
                        };
                        return Err(OmonError::Approval(msg));
                    }
                }
            }
        }

        tool.execute_with_context(args, session).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ApprovalError;
    use serde_json::json;
    use tokio::sync::Mutex;

    struct MockApprovalRequester {
        decision: Mutex<ApprovalDecision>,
    }

    #[async_trait]
    impl ApprovalRequester for MockApprovalRequester {
        async fn request_approval(
            &self,
            _session: &SessionKey,
            _command: &str,
            _reason: &str,
        ) -> Result<ApprovalDecision, ApprovalError> {
            Ok(self.decision.lock().await.clone())
        }
    }

    struct GatedDummyTool;

    #[tokio::test]
    async fn tool_approval_scopes_do_not_cross_rules() {
        use crate::{
            Database, DiscordApprovalRequester, OutboundAction, OutboundDispatcher,
            SmartApprovalGuard,
        };

        struct Resolver {
            guard: SmartApprovalGuard,
            prompts: Arc<Mutex<Vec<(String, String)>>>,
        }
        #[async_trait]
        impl OutboundDispatcher for Resolver {
            async fn dispatch(&self, action: OutboundAction) -> crate::Result<()> {
                if let OutboundAction::ApprovalRequest {
                    request_id,
                    command,
                    reason,
                    ..
                } = action
                {
                    let mut prompts = self.prompts.lock().await;
                    prompts.push((command, reason));
                    let decision = if prompts.len() == 1 { "always" } else { "deny" };
                    assert!(
                        self.guard
                            .resolve_custom_id(&format!("omon:approval:{request_id}:{decision}"))
                            .await
                    );
                }
                Ok(())
            }
        }

        let calls = Arc::new(Mutex::new(Vec::<Value>::new()));
        let fixture_calls = calls.clone();
        let app = axum::Router::new().route(
            "/",
            axum::routing::post(move |axum::Json(request): axum::Json<Value>| {
                let calls = fixture_calls.clone();
                async move {
                    assert_eq!(request["method"], "tools/call");
                    assert!(
                        ["rule_a", "rule_b"].contains(&request["params"]["name"].as_str().unwrap())
                    );
                    calls.lock().await.push(request["params"].clone());
                    axum::Json(
                        json!({"jsonrpc":"2.0", "id":request["id"], "result":request["params"]}),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let (shutdown, stopped) = tokio::sync::oneshot::channel::<()>();
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
        let prompts = Arc::new(Mutex::new(Vec::new()));
        let requester = Arc::new(DiscordApprovalRequester::new(
            guard.clone(),
            Duration::from_secs(5),
        ));
        requester
            .set_dispatcher(Arc::new(Resolver {
                guard: guard.clone(),
                prompts: prompts.clone(),
            }))
            .await;
        let mut registry =
            ToolRegistry::default().with_approval_requester(requester, Duration::from_secs(5));
        registry.register(McpTool::new(
            [("client_a", "rule_a"), ("client_b", "rule_b")]
                .into_iter()
                .map(|(name, remote)| {
                    McpClientTool::new(
                        name,
                        remote,
                        "fixture",
                        json!({}),
                        McpTransport::Sse {
                            url: url.clone(),
                            bearer_token: None,
                        },
                    )
                    .with_approval(true)
                })
                .collect(),
        ));
        let session = SessionKey::new("discord", None::<String>, "a", None::<String>, "u");
        let other = SessionKey::new("discord", None::<String>, "b", None::<String>, "u");
        let a = json!({"tool":"client_a", "arguments":{"sentinel":"A"}});
        let b = json!({"tool":"client_b", "arguments":{"sentinel":"B"}});
        let first = registry
            .execute_with_context("mcp", a.clone(), Some(&session))
            .await
            .unwrap();
        assert_eq!(first["arguments"], a["arguments"]);
        // Recreate the requester and guard from SQLite: Always must remain global,
        // but must never become a grant for another rule under the same tool.
        let restored = SmartApprovalGuard::new().with_pool(db.pool().clone());
        assert_eq!(restored.load_persisted_allowlist().await.unwrap(), 1);
        let requester = Arc::new(DiscordApprovalRequester::new(
            restored.clone(),
            Duration::from_secs(5),
        ));
        requester
            .set_dispatcher(Arc::new(Resolver {
                guard: restored.clone(),
                prompts: prompts.clone(),
            }))
            .await;
        registry.set_approval_requester(requester.clone(), Duration::from_secs(5));
        registry
            .execute_with_context("mcp", a, Some(&other))
            .await
            .unwrap();
        assert_eq!(prompts.lock().await.len(), 1);
        let second = registry.execute_with_context("mcp", b, Some(&other)).await;
        let prompt_count = prompts.lock().await.len();
        let call_count = calls.lock().await.len();
        // Terminal category grants deliberately remain broad.
        let terminal_key = crate::security::derive_pattern_key("rm -rf /tmp/one");
        restored.approve_session(&other, &terminal_key).await;
        assert!(requester
            .request_approval(&other, "rm -rf /tmp/two", "recursive delete")
            .await
            .unwrap()
            .is_approved());
        assert_eq!(guard.pending_count().await, 0);
        assert_eq!(restored.pending_count().await, 0);
        shutdown.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        db.pool().close().await;
        println!(
            "AP06 prompts={prompt_count}, MCP calls={call_count}, rule B denied={}",
            second.is_err()
        );
        assert_eq!(
            prompt_count, 2,
            "rule B autoapproved using rule A's Always grant"
        );
        assert!(matches!(second, Err(OmonError::Approval(_))));
        assert_eq!(call_count, 2, "denied rule B must not reach MCP");
    }
    #[async_trait]
    impl Tool for GatedDummyTool {
        fn name(&self) -> &str {
            "gated_dummy"
        }
        fn description(&self) -> &str {
            "gated dummy"
        }
        fn input_schema(&self) -> Value {
            json!({})
        }
        fn requires_approval(&self, args: &Value) -> Option<String> {
            if args.get("dangerous").and_then(Value::as_bool) == Some(true) {
                Some("dangerous parameter passed".to_string())
            } else {
                None
            }
        }
        async fn execute(&self, _args: Value) -> Result<Value, OmonError> {
            Ok(json!({"result": "success"}))
        }
    }

    #[tokio::test]
    async fn test_tool_approval_hook_dispatch_approved() {
        let mut registry = ToolRegistry::new();
        registry.register(GatedDummyTool);
        let approver = Arc::new(MockApprovalRequester {
            decision: Mutex::new(ApprovalDecision::Once),
        });
        registry.set_approval_requester(approver, Duration::from_secs(5));

        let session = SessionKey::new("discord", None::<String>, "c1", None::<String>, "u1");
        let safe_res = registry
            .execute_with_context("gated_dummy", json!({"dangerous": false}), Some(&session))
            .await
            .unwrap();
        assert_eq!(safe_res, json!({"result": "success"}));
        let dangerous_res = registry
            .execute_with_context("gated_dummy", json!({"dangerous": true}), Some(&session))
            .await
            .unwrap();
        assert_eq!(dangerous_res, json!({"result": "success"}));
    }

    #[tokio::test]
    async fn test_tool_approval_hook_dispatch_denied() {
        let mut registry = ToolRegistry::new();
        registry.register(GatedDummyTool);
        let approver = Arc::new(MockApprovalRequester {
            decision: Mutex::new(ApprovalDecision::Deny {
                reason: Some("Operation blocked by admin".to_string()),
            }),
        });
        registry.set_approval_requester(approver, Duration::from_secs(5));

        let session = SessionKey::new("discord", None::<String>, "c1", None::<String>, "u1");
        let err = registry
            .execute_with_context("gated_dummy", json!({"dangerous": true}), Some(&session))
            .await
            .unwrap_err();
        assert!(matches!(err, OmonError::Approval(_)));
        assert!(err.to_string().contains("Operation blocked by admin"));
    }
}
