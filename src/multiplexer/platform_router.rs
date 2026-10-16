use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use super::OutboundDispatcher;
use crate::{OmonError, OutboundAction, Result, SessionKey};

/// Sends each action to the dispatcher registered for its session's platform. Actions without
/// a session (approval expiry) go to every platform, since only the owner of the request id
/// knows the message to update.
#[derive(Clone, Default)]
pub struct PlatformRouter {
    routes: HashMap<String, Arc<dyn OutboundDispatcher>>,
    fallback: Option<Arc<dyn OutboundDispatcher>>,
}

impl PlatformRouter {
    pub fn new(fallback: Arc<dyn OutboundDispatcher>) -> Self {
        Self {
            routes: HashMap::new(),
            fallback: Some(fallback),
        }
    }

    pub fn with_platform(mut self, platform: &str, dispatcher: Arc<dyn OutboundDispatcher>) -> Self {
        self.routes.insert(platform.to_ascii_lowercase(), dispatcher);
        self
    }

    fn for_session(&self, session: &SessionKey) -> Result<&Arc<dyn OutboundDispatcher>> {
        self.routes
            .get(&session.platform.to_ascii_lowercase())
            .or(self.fallback.as_ref())
            .ok_or_else(|| {
                OmonError::Config(format!(
                    "no outbound dispatcher for platform {}",
                    session.platform
                ))
            })
    }
}

pub fn action_session(action: &OutboundAction) -> Option<&SessionKey> {
    match action {
        OutboundAction::SendMessage { session, .. }
        | OutboundAction::EditMessage { session, .. }
        | OutboundAction::DeleteMessage { session, .. }
        | OutboundAction::UploadFile { session, .. }
        | OutboundAction::Stream { session, .. }
        | OutboundAction::Typing { session, .. }
        | OutboundAction::React { session, .. }
        | OutboundAction::ApprovalRequest { session, .. } => Some(session),
        OutboundAction::ExpireApproval { .. } => None,
    }
}

#[async_trait]
impl OutboundDispatcher for PlatformRouter {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        match action_session(&action) {
            Some(session) => self.for_session(session)?.dispatch(action).await,
            None => {
                let mut targets: Vec<&Arc<dyn OutboundDispatcher>> = self.routes.values().collect();
                if let Some(fallback) = &self.fallback {
                    if !targets.iter().any(|target| Arc::ptr_eq(target, fallback)) {
                        targets.push(fallback);
                    }
                }
                let mut first_error = None;
                let mut delivered = false;
                for target in targets {
                    match target.dispatch(action.clone()).await {
                        Ok(()) => delivered = true,
                        Err(error) => {
                            first_error.get_or_insert(error);
                        }
                    }
                }
                match (delivered, first_error) {
                    (false, Some(error)) => Err(error),
                    _ => Ok(()),
                }
            }
        }
    }

    async fn owner_thread_is_private(&self, session: &SessionKey) -> bool {
        match self.for_session(session) {
            Ok(dispatcher) => dispatcher.owner_thread_is_private(session).await,
            Err(_) => false,
        }
    }

    async fn dispatch_direct_reply(&self, session: SessionKey, content: String) -> Result<()> {
        self.for_session(&session)?
            .dispatch_direct_reply(session, content)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>, bool);

    #[async_trait]
    impl OutboundDispatcher for Recorder {
        async fn dispatch(&self, action: OutboundAction) -> Result<()> {
            if self.1 {
                return Err(OmonError::Multiplexer("down".into()));
            }
            let label = action_session(&action)
                .map_or_else(|| "expire".to_owned(), |s| s.platform.clone());
            self.0.lock().push(label);
            Ok(())
        }
    }

    fn send(platform: &str) -> OutboundAction {
        OutboundAction::SendMessage {
            session: SessionKey::new(platform, None::<String>, "c", None::<String>, "u"),
            content: "hi".into(),
            reply_to: None,
        }
    }

    #[tokio::test]
    async fn routes_by_platform_and_falls_back() {
        let discord = Arc::new(Recorder::default());
        let slack = Arc::new(Recorder::default());
        let router = PlatformRouter::new(discord.clone()).with_platform("slack", slack.clone());
        router.dispatch(send("slack")).await.unwrap();
        router.dispatch(send("discord")).await.unwrap();
        router.dispatch(send("cron")).await.unwrap();
        assert_eq!(*slack.0.lock(), ["slack"]);
        assert_eq!(*discord.0.lock(), ["discord", "cron"]);
    }

    #[tokio::test]
    async fn expiry_reaches_every_platform_and_tolerates_one_failure() {
        let discord = Arc::new(Recorder::default());
        let slack = Arc::new(Recorder(Mutex::new(Vec::new()), true));
        let router = PlatformRouter::new(discord.clone()).with_platform("slack", slack);
        router
            .dispatch(OutboundAction::ExpireApproval {
                request_id: uuid::Uuid::new_v4(),
            })
            .await
            .unwrap();
        assert_eq!(*discord.0.lock(), ["expire"]);
    }
}
