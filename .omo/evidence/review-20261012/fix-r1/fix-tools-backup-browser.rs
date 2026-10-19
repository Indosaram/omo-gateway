use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::{json, Value};

use super::Tool;
use crate::{OmonError, SessionKey};

/// Opt-in escape hatch for operators who really do want the agent to drive a
/// browser against loopback/private services (local dev servers, for example).
const ALLOW_INTERNAL_ENV: &str = "OMON_BROWSER_ALLOW_INTERNAL_HOSTS";
const DNS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Clone)]
pub struct BrowserTool {
    cdp_port: u16,
    /// Tab ids this tool instance opened, keyed by the opening session's storage
    /// key. The CDP endpoint is shared with the operator and every other
    /// session, so snapshots are filtered to the caller's own tabs.
    owned_tabs: Arc<Mutex<HashMap<String, HashSet<String>>>>,
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new(9333)
    }
}

impl BrowserTool {
    pub fn new(cdp_port: u16) -> Self {
        Self {
            cdp_port,
            owned_tabs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

fn internal_destinations_allowed() -> bool {
    std::env::var(ALLOW_INTERNAL_ENV)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn is_internal_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        // 0.0.0.0/8 ("this network") and 100.64.0.0/10 (carrier NAT) are not
        // public destinations either; `Ipv4Addr::is_shared` is still unstable.
        || a == 0
        || (a == 100 && (64..128).contains(&b))
}

fn is_internal_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped().or_else(|| ip.to_ipv4()) {
        return is_internal_ipv4(mapped);
    }
    let first = ip.segments()[0];
    ip.is_loopback()
        || ip.is_unspecified()
        // fc00::/7 unique-local and fe80::/10 link-local; both accessors are
        // still unstable, so the prefixes are matched directly.
        || (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfe80
}

fn is_internal_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_internal_ipv4(v4),
        IpAddr::V6(v6) => is_internal_ipv6(v6),
    }
}

/// Rejects hosts by literal, before any DNS work: IP literals in every notation
/// the URL parser normalises, plus the `localhost` names.
fn is_internal_host_literal(host: &str) -> bool {
    let lowered = host.trim_end_matches('.').to_ascii_lowercase();
    let bare = strip_brackets(&lowered);
    if let Ok(ip) = bare.parse::<IpAddr>() {
        return is_internal_ip(ip);
    }
    bare == "localhost" || bare.ends_with(".localhost")
}

fn strip_brackets(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(host)
}

/// Blocks SSRF: the CDP endpoint would happily fetch loopback, link-local
/// (cloud metadata) and private-range URLs on behalf of the model, so the
/// destination is checked by host literal and, for names, by resolved address.
async fn ensure_public_destination(url: &reqwest::Url) -> Result<(), OmonError> {
    if internal_destinations_allowed() {
        return Ok(());
    }
    let host = url.host_str().ok_or_else(|| {
        OmonError::ToolExecution("browser navigation requires a URL with a host".into())
    })?;
    let blocked = || {
        OmonError::ToolExecution(format!(
            "browser navigation to internal address `{host}` is refused; set {ALLOW_INTERNAL_ENV}=1 to allow loopback/private destinations"
        ))
    };
    if is_internal_host_literal(host) {
        return Err(blocked());
    }
    let bare = strip_brackets(host);
    if bare.parse::<IpAddr>().is_ok() {
        // Public IP literal: nothing left to resolve.
        return Ok(());
    }

    let port = url.port_or_known_default().unwrap_or(80);
    let resolved = tokio::time::timeout(DNS_TIMEOUT, tokio::net::lookup_host((bare, port)))
        .await
        .map_err(|_| {
            OmonError::ToolExecution(format!("timed out resolving `{host}` before navigation"))
        })?
        .map_err(|error| {
            OmonError::ToolExecution(format!("could not resolve `{host}`: {error}"))
        })?;

    let mut any = false;
    for addr in resolved {
        any = true;
        if is_internal_ip(addr.ip()) {
            return Err(blocked());
        }
    }
    if !any {
        return Err(OmonError::ToolExecution(format!(
            "could not resolve `{host}` to any address"
        )));
    }
    Ok(())
}

/// Re-checks where a tab actually ended up: a redirect or DNS rebinding can
/// land a tab on an internal host even though the requested URL passed
/// `ensure_public_destination`. Non-http(s) URLs (e.g. `about:blank`) cannot
/// reach a network service and are accepted.
async fn ensure_public_final_url(url: &str) -> Result<(), OmonError> {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return Ok(());
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return Ok(());
    }
    ensure_public_destination(&parsed).await
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        "Control live web browser via CDP on port 9333. Actions: 'navigate' (open URL), 'snapshot' (get page title, URL, content preview), 'eval' (evaluate JavaScript in page), 'screenshot' (capture page state)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["navigate", "snapshot", "eval", "screenshot"],
                    "description": "The browser action to perform."
                },
                "url": {
                    "type": "string",
                    "description": "URL to navigate to."
                },
                "script": {
                    "type": "string",
                    "description": "JavaScript code to evaluate on the page."
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, OmonError> {
        self.execute_inner(args, None).await
    }

    async fn execute_with_context(
        &self,
        args: Value,
        session: Option<&SessionKey>,
    ) -> Result<Value, OmonError> {
        self.execute_inner(args, session).await
    }
}

impl BrowserTool {
    async fn execute_inner(
        &self,
        args: Value,
        session: Option<&SessionKey>,
    ) -> Result<Value, OmonError> {
        // Tabs are owned per session; context-free calls share one anonymous bucket.
        let owner = session.map(SessionKey::storage_key).unwrap_or_default();
        let action = args
            .get("action")
            .and_then(Value::as_str)
            .ok_or_else(|| OmonError::ToolExecution("missing 'action'".into()))?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| OmonError::ToolExecution(e.to_string()))?;

        let list_url = format!("http://127.0.0.1:{}/json/list", self.cdp_port);
        let resp = client.get(&list_url).send().await.map_err(|e| {
            OmonError::ToolExecution(format!("CDP not reachable on port {}: {e}", self.cdp_port))
        })?;
        if !resp.status().is_success() {
            return Err(OmonError::ToolExecution(format!(
                "CDP returned {} for page listing",
                resp.status()
            )));
        }

        let pages: Value = resp
            .json()
            .await
            .map_err(|e| OmonError::ToolExecution(format!("invalid CDP response: {e}")))?;

        match action {
            "snapshot" => {
                // The CDP endpoint is shared with the operator and other
                // sessions; only report pages this tool opened.
                let owned = self.owned_tabs.lock();
                let owned = owned.get(&owner);
                let mine: Vec<Value> = pages
                    .as_array()
                    .map(|all| {
                        all.iter()
                            .filter(|page| {
                                page.get("id")
                                    .and_then(Value::as_str)
                                    .zip(owned)
                                    .map(|(id, owned)| owned.contains(id))
                                    .unwrap_or(false)
                            })
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(json!({
                    "cdp_port": self.cdp_port,
                    "active_pages_count": mine.len(),
                    "pages": mine
                }))
            }
            "navigate" => {
                let url = args
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| OmonError::ToolExecution("missing 'url'".into()))?;

                let parsed = reqwest::Url::parse(url)
                    .map_err(|error| OmonError::ToolExecution(format!("invalid URL: {error}")))?;
                if !matches!(parsed.scheme(), "http" | "https") {
                    return Err(OmonError::ToolExecution(
                        "browser navigation only supports http and https URLs".into(),
                    ));
                }
                ensure_public_destination(&parsed).await?;
                let new_tab_url = format!(
                    "http://127.0.0.1:{}/json/new?{}",
                    self.cdp_port,
                    urlencoding::encode(parsed.as_str())
                );
                let new_resp = client.put(&new_tab_url).send().await.map_err(|e| {
                    OmonError::ToolExecution(format!("failed to open new tab: {e}"))
                })?;

                if !new_resp.status().is_success() {
                    return Err(OmonError::ToolExecution(format!(
                        "CDP returned {} while opening a tab",
                        new_resp.status()
                    )));
                }
                let tab_info: Value = new_resp.json().await.map_err(|error| {
                    OmonError::ToolExecution(format!("invalid CDP tab response: {error}"))
                })?;
                let tab_id = tab_info
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if let Some(tab_id) = &tab_id {
                    // A redirect or DNS rebinding may have moved the tab onto an
                    // internal host after the pre-check; verify where it landed.
                    let final_url = client
                        .get(&list_url)
                        .send()
                        .await
                        .ok()
                        .and_then(|resp| resp.status().is_success().then_some(resp));
                    let final_url = match final_url {
                        Some(resp) => resp.json::<Value>().await.ok(),
                        None => None,
                    }
                    .and_then(|list| {
                        list.as_array()?
                            .iter()
                            .find(|page| page.get("id").and_then(Value::as_str) == Some(tab_id))?
                            .get("url")?
                            .as_str()
                            .map(str::to_string)
                    });
                    if let Some(final_url) = final_url {
                        if let Err(error) = ensure_public_final_url(&final_url).await {
                            let close_url =
                                format!("http://127.0.0.1:{}/json/close/{}", self.cdp_port, tab_id);
                            let _ = client.get(&close_url).send().await;
                            return Err(error);
                        }
                    }
                    self.owned_tabs
                        .lock()
                        .entry(owner)
                        .or_default()
                        .insert(tab_id.clone());
                }

                Ok(json!({
                    "status": "navigated",
                    "url": url,
                    "tab": tab_info
                }))
            }
            "eval" | "screenshot" => Err(OmonError::ToolExecution(format!(
                "browser action `{action}` requires a CDP WebSocket session and is not configured"
            ))),
            _ => Err(OmonError::ToolExecution(format!(
                "unknown browser action: {action}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    struct FakeCdp {
        port: u16,
        navigations: Arc<Mutex<Vec<String>>>,
        closed: Arc<Mutex<Vec<String>>>,
    }

    /// Stands in for the shared Chrome DevTools endpoint: it lists `pages` and
    /// records every URL a `/json/new` call tried to open.
    async fn start_fake_cdp(pages: Value) -> FakeCdp {
        let navigations = Arc::new(Mutex::new(Vec::new()));
        let opened = navigations.clone();
        let closed = Arc::new(Mutex::new(Vec::new()));
        let closer = closed.clone();
        let app = axum::Router::new()
            .route(
                "/json/close/{id}",
                axum::routing::get(
                    move |axum::extract::Path(id): axum::extract::Path<String>| {
                        let closer = closer.clone();
                        async move {
                            closer.lock().push(id);
                            "Target is closing"
                        }
                    },
                ),
            )
            .route(
                "/json/list",
                axum::routing::get(move || {
                    let pages = pages.clone();
                    async move { axum::Json(pages) }
                }),
            )
            .route(
                "/json/new",
                axum::routing::put(
                    move |axum::extract::RawQuery(query): axum::extract::RawQuery| {
                        let opened = opened.clone();
                        async move {
                            let target = query.unwrap_or_default();
                            let mut opened = opened.lock();
                            opened.push(target.clone());
                            axum::Json(serde_json::json!({
                                "id": format!("tab-{}", opened.len()),
                                "url": target,
                            }))
                        }
                    },
                ),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        FakeCdp {
            port,
            navigations,
            closed,
        }
    }

    #[tokio::test]
    async fn navigate_refuses_internal_destinations_but_allows_public_ones() {
        let cdp = start_fake_cdp(json!([])).await;
        let tool = BrowserTool::new(cdp.port);

        let cases = [
            ("http://127.0.0.1:9222/json/list", false),
            ("http://127.9.9.9/", false),
            ("http://localhost:8080/admin", false),
            ("http://LOCALHOST./admin", false),
            ("http://[::1]:9222/", false),
            ("http://[::ffff:127.0.0.1]/", false),
            ("http://2130706433/", false),
            ("http://169.254.169.254/latest/meta-data/", false),
            ("http://[fe80::1]/", false),
            ("http://10.1.2.3/internal", false),
            ("http://172.16.4.5/internal", false),
            ("http://192.168.0.1/router", false),
            ("http://[fd00::1]/", false),
            ("http://0.0.0.0:9222/", false),
            ("https://example.com/docs", true),
        ];

        for (url, should_be_allowed) in cases {
            let result = tool
                .execute(json!({"action": "navigate", "url": url}))
                .await;
            assert_eq!(
                result.is_ok(),
                should_be_allowed,
                "navigate to {url} returned {result:?}"
            );
        }

        let reached = cdp.navigations.lock().clone();
        assert_eq!(
            reached.len(),
            1,
            "only the public destination may reach the CDP endpoint, got {reached:?}"
        );
    }

    #[tokio::test]
    async fn snapshot_only_returns_pages_this_tool_opened() {
        let cdp = start_fake_cdp(json!([
            {
                "id": "operator-tab",
                "title": "Operator banking",
                "url": "https://bank.example/account"
            },
            {
                "id": "tab-1",
                "title": "Docs",
                "url": "https://example.com/docs"
            }
        ]))
        .await;
        let tool = BrowserTool::new(cdp.port);

        tool.execute(json!({"action": "navigate", "url": "https://example.com/docs"}))
            .await
            .unwrap();

        let snapshot = tool.execute(json!({"action": "snapshot"})).await.unwrap();
        let returned: Vec<&str> = snapshot["pages"]
            .as_array()
            .expect("pages array")
            .iter()
            .filter_map(|page| page["id"].as_str())
            .collect();

        assert_eq!(
            returned,
            ["tab-1"],
            "snapshot leaked pages this tool never opened: {snapshot}"
        );
        assert_eq!(snapshot["active_pages_count"], 1);
    }

    #[tokio::test]
    async fn navigate_closes_tab_whose_final_url_is_internal() {
        // The public URL passed the pre-check, but the tab ended up on the cloud
        // metadata address (redirect or DNS rebinding).
        let cdp = start_fake_cdp(json!([
            {"id": "tab-1", "url": "http://169.254.169.254/latest/meta-data/"}
        ]))
        .await;
        let tool = BrowserTool::new(cdp.port);

        let result = tool
            .execute(json!({"action": "navigate", "url": "https://example.com/docs"}))
            .await;

        assert!(
            matches!(&result, Err(OmonError::ToolExecution(msg)) if msg.contains("internal address")),
            "final internal URL must be refused, got {result:?}"
        );
        assert_eq!(cdp.closed.lock().clone(), ["tab-1"]);
        let snapshot = tool.execute(json!({"action": "snapshot"})).await.unwrap();
        assert_eq!(snapshot["active_pages_count"], 0, "{snapshot}");
    }

    #[tokio::test]
    async fn final_url_check_rejects_internal_and_ignores_non_http() {
        assert!(ensure_public_final_url("http://127.0.0.1:9222/json")
            .await
            .is_err());
        assert!(ensure_public_final_url("http://[fe80::1]/").await.is_err());
        assert!(ensure_public_final_url("http://10.0.0.5/").await.is_err());
        assert!(ensure_public_final_url("https://93.184.215.14/")
            .await
            .is_ok());
        assert!(ensure_public_final_url("about:blank").await.is_ok());
    }

    #[tokio::test]
    async fn snapshot_is_scoped_to_the_calling_session() {
        let cdp = start_fake_cdp(json!([
            {"id": "tab-1", "title": "Docs", "url": "https://example.com/docs"}
        ]))
        .await;
        let tool = BrowserTool::new(cdp.port);
        let alice = crate::SessionKey::new("discord", None::<String>, "1", None::<String>, "a");
        let bob = crate::SessionKey::new("discord", None::<String>, "2", None::<String>, "b");

        tool.execute_with_context(
            json!({"action": "navigate", "url": "https://example.com/docs"}),
            Some(&alice),
        )
        .await
        .unwrap();

        let theirs = tool
            .execute_with_context(json!({"action": "snapshot"}), Some(&bob))
            .await
            .unwrap();
        assert_eq!(theirs["active_pages_count"], 0, "leaked: {theirs}");
        let mine = tool
            .execute_with_context(json!({"action": "snapshot"}), Some(&alice))
            .await
            .unwrap();
        assert_eq!(mine["active_pages_count"], 1, "{mine}");
    }
}
