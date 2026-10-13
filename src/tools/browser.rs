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
/// How often and how many times `/json/list` is polled to see where a freshly
/// opened tab settled before it is accepted.
const SETTLE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
const SETTLE_MAX_POLLS: usize = 40;

#[derive(Clone)]
pub struct BrowserTool {
    cdp_port: u16,
    /// Tab ids this tool instance opened, keyed by the opening session's storage
    /// key. The CDP endpoint is shared with the operator and every other
    /// session, so snapshots are filtered to the caller's own tabs.
    owned_tabs: Arc<Mutex<HashMap<String, HashSet<String>>>>,
    settle_interval: std::time::Duration,
    settle_max_polls: usize,
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
            settle_interval: SETTLE_INTERVAL,
            settle_max_polls: SETTLE_MAX_POLLS,
        }
    }

    /// Overrides how a new tab is polled for its settled URL.
    pub fn with_settle(mut self, interval: std::time::Duration, max_polls: usize) -> Self {
        self.settle_interval = interval;
        self.settle_max_polls = max_polls;
        self
    }

    /// Polls `/json/list` until the tab's URL is real (not blank) and unchanged
    /// across two consecutive reads, checking every observed URL against the
    /// internal-address filter. Fails closed: a failed or malformed listing, a
    /// vanished tab, or a tab that never settles is an error.
    async fn verify_tab_settled(
        &self,
        client: &reqwest::Client,
        list_url: &str,
        tab_id: &str,
    ) -> Result<(), OmonError> {
        if internal_destinations_allowed() {
            return Ok(());
        }
        let mut previous: Option<String> = None;
        for attempt in 0..self.settle_max_polls {
            if attempt > 0 {
                tokio::time::sleep(self.settle_interval).await;
            }
            let url = fetch_tab_url(client, list_url, tab_id).await?;
            ensure_public_final_url(&url).await?;
            if !url.is_empty() && url != "about:blank" && previous.as_deref() == Some(url.as_str())
            {
                return Ok(());
            }
            previous = Some(url);
        }
        Err(OmonError::ToolExecution(
            "browser tab did not settle on a final URL; navigation refused because the destination could not be verified".into(),
        ))
    }

    async fn close_tab(&self, client: &reqwest::Client, tab_id: &str) {
        let close_url = format!("http://127.0.0.1:{}/json/close/{}", self.cdp_port, tab_id);
        let _ = client.get(&close_url).send().await;
        let mut owned = self.owned_tabs.lock();
        for tabs in owned.values_mut() {
            tabs.remove(tab_id);
        }
        owned.retain(|_, tabs| !tabs.is_empty());
    }
}

async fn fetch_tab_url(
    client: &reqwest::Client,
    list_url: &str,
    tab_id: &str,
) -> Result<String, OmonError> {
    let unverifiable =
        |why: String| OmonError::ToolExecution(format!("could not verify browser tab: {why}"));
    let resp = client
        .get(list_url)
        .send()
        .await
        .map_err(|error| unverifiable(error.to_string()))?;
    if !resp.status().is_success() {
        return Err(unverifiable(format!("CDP returned {}", resp.status())));
    }
    let list: Value = resp
        .json()
        .await
        .map_err(|error| unverifiable(error.to_string()))?;
    let page = list
        .as_array()
        .and_then(|pages| {
            pages
                .iter()
                .find(|page| page.get("id").and_then(Value::as_str) == Some(tab_id))
        })
        .ok_or_else(|| unverifiable("tab is missing from the CDP listing".into()))?;
    Ok(page
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string())
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
    let [a, b, c, _] = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        // 0.0.0.0/8 ("this network") and 100.64.0.0/10 (carrier NAT) are not
        // public destinations either; `Ipv4Addr::is_shared` is still unstable.
        || a == 0
        || (a == 100 && (64..128).contains(&b))
        || ip.is_multicast()
        // 240.0.0.0/4 reserved, 192.0.0.0/24 IETF protocol assignments,
        // 198.18.0.0/15 benchmarking.
        || a >= 240
        || (a == 192 && b == 0 && c == 0)
        || (a == 198 && (b == 18 || b == 19))
}

fn is_internal_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped().or_else(|| ip.to_ipv4()) {
        return is_internal_ipv4(mapped);
    }
    let segments = ip.segments();
    let first = segments[0];
    // NAT64 (64:ff9b::/96) and 6to4 (2002::/16) embed an IPv4 address.
    if first == 0x64 && segments[1] == 0xff9b && segments[2..6].iter().all(|s| *s == 0) {
        return is_internal_ipv4(Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            segments[6] as u8,
            (segments[7] >> 8) as u8,
            segments[7] as u8,
        ));
    }
    if first == 0x2002 {
        return is_internal_ipv4(Ipv4Addr::new(
            (segments[1] >> 8) as u8,
            segments[1] as u8,
            (segments[2] >> 8) as u8,
            segments[2] as u8,
        ));
    }
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
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
                let listed = pages.as_array().cloned().unwrap_or_default();
                let candidates: Vec<Value> = {
                    let mut owned = self.owned_tabs.lock();
                    // Forget tabs that no longer exist so the map cannot grow forever.
                    let live: HashSet<&str> = listed
                        .iter()
                        .filter_map(|page| page.get("id").and_then(Value::as_str))
                        .collect();
                    for tabs in owned.values_mut() {
                        tabs.retain(|id| live.contains(id.as_str()));
                    }
                    owned.retain(|_, tabs| !tabs.is_empty());
                    let mine = owned.get(&owner);
                    let selected: Vec<Value> = listed
                        .iter()
                        .filter(|page| {
                            page.get("id")
                                .and_then(Value::as_str)
                                .zip(mine)
                                .map(|(id, mine)| mine.contains(id))
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect();
                    selected
                };
                // A tab may have navigated onto an internal host after it was accepted.
                let mut mine = Vec::new();
                for page in candidates {
                    let url = page.get("url").and_then(Value::as_str).unwrap_or_default();
                    if ensure_public_final_url(url).await.is_err() {
                        if let Some(id) = page.get("id").and_then(Value::as_str) {
                            self.close_tab(&client, id).await;
                        }
                    } else {
                        mine.push(page);
                    }
                }
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
                let Some(tab_id) = tab_info
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                else {
                    return Err(OmonError::ToolExecution(
                        "CDP did not return a tab id, so the final URL cannot be verified".into(),
                    ));
                };
                // A redirect or DNS rebinding may move the tab onto an internal host
                // after the pre-check, and the tab keeps loading after `/json/new`
                // returns; wait for it to settle, then verify where it landed.
                if let Err(error) = self.verify_tab_settled(&client, &list_url, &tab_id).await {
                    self.close_tab(&client, &tab_id).await;
                    return Err(error);
                }
                self.owned_tabs
                    .lock()
                    .entry(owner)
                    .or_default()
                    .insert(tab_id);

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
        start_fake_cdp_sequence(vec![pages]).await
    }

    /// Like `start_fake_cdp`, but the Nth `/json/list` call returns `pages[N]`
    /// (the last entry repeats), so a tab can be seen moving between reads. Every
    /// tool call lists once up front before the action runs.
    async fn start_fake_cdp_sequence(pages: Vec<Value>) -> FakeCdp {
        let list_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
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
                    let index = list_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let page = pages[index.min(pages.len() - 1)].clone();
                    async move { axum::Json(page) }
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
        let cdp = start_fake_cdp(json!([
            {"id": "tab-1", "url": "https://example.com/docs"}
        ]))
        .await;
        let tool = BrowserTool::new(cdp.port).with_settle(std::time::Duration::ZERO, 5);

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
        let tool = BrowserTool::new(cdp.port).with_settle(std::time::Duration::ZERO, 5);

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
        let tool = BrowserTool::new(cdp.port).with_settle(std::time::Duration::ZERO, 5);

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
    async fn navigate_fails_closed_when_tab_is_missing_from_listing() {
        let cdp = start_fake_cdp(json!([])).await;
        let tool = BrowserTool::new(cdp.port).with_settle(std::time::Duration::ZERO, 5);

        let result = tool
            .execute(json!({"action": "navigate", "url": "https://example.com/docs"}))
            .await;

        assert!(
            matches!(&result, Err(OmonError::ToolExecution(msg)) if msg.contains("could not verify")),
            "unverifiable tab must be refused, got {result:?}"
        );
        assert_eq!(cdp.closed.lock().clone(), ["tab-1"]);
        assert!(tool.owned_tabs.lock().is_empty());
    }

    #[tokio::test]
    async fn navigate_fails_closed_when_listing_is_not_an_array() {
        let cdp = start_fake_cdp_sequence(vec![json!([]), json!({"unexpected": true})]).await;
        let tool = BrowserTool::new(cdp.port).with_settle(std::time::Duration::ZERO, 5);

        let result = tool
            .execute(json!({"action": "navigate", "url": "https://example.com/docs"}))
            .await;

        assert!(
            result.is_err(),
            "malformed listing must fail closed: {result:?}"
        );
        assert_eq!(cdp.closed.lock().clone(), ["tab-1"]);
    }

    #[tokio::test]
    async fn navigate_fails_closed_when_tab_never_leaves_about_blank() {
        let cdp = start_fake_cdp(json!([{"id": "tab-1", "url": "about:blank"}])).await;
        let tool = BrowserTool::new(cdp.port).with_settle(std::time::Duration::ZERO, 4);

        let result = tool
            .execute(json!({"action": "navigate", "url": "https://example.com/docs"}))
            .await;

        assert!(
            matches!(&result, Err(OmonError::ToolExecution(msg)) if msg.contains("did not settle")),
            "unsettled tab must be refused, got {result:?}"
        );
        assert_eq!(cdp.closed.lock().clone(), ["tab-1"]);
    }

    #[tokio::test]
    async fn navigate_catches_redirect_that_lands_after_the_first_read() {
        // First read still shows the public URL; a later read shows the redirect target.
        let cdp = start_fake_cdp_sequence(vec![
            json!([]),
            json!([{"id": "tab-1", "url": "https://example.com/r"}]),
            json!([{"id": "tab-1", "url": "http://192.168.0.1/admin"}]),
        ])
        .await;
        let tool = BrowserTool::new(cdp.port).with_settle(std::time::Duration::ZERO, 5);

        let result = tool
            .execute(json!({"action": "navigate", "url": "https://example.com/r"}))
            .await;

        assert!(
            matches!(&result, Err(OmonError::ToolExecution(msg)) if msg.contains("internal address")),
            "late redirect must be refused, got {result:?}"
        );
        assert_eq!(cdp.closed.lock().clone(), ["tab-1"]);
        assert!(tool.owned_tabs.lock().is_empty());
    }

    #[tokio::test]
    async fn snapshot_closes_owned_tab_that_moved_to_an_internal_host() {
        let ok = json!([{"id": "tab-1", "title": "Docs", "url": "https://example.com/docs"}]);
        let moved = json!([{"id": "tab-1", "title": "Meta", "url": "http://169.254.169.254/"}]);
        let cdp = start_fake_cdp_sequence(vec![json!([]), ok.clone(), ok, moved]).await;
        let tool = BrowserTool::new(cdp.port).with_settle(std::time::Duration::ZERO, 5);
        tool.execute(json!({"action": "navigate", "url": "https://example.com/docs"}))
            .await
            .unwrap();

        let snapshot = tool.execute(json!({"action": "snapshot"})).await.unwrap();

        assert_eq!(snapshot["active_pages_count"], 0, "{snapshot}");
        assert_eq!(cdp.closed.lock().clone(), ["tab-1"]);
    }

    #[tokio::test]
    async fn snapshot_forgets_tabs_that_no_longer_exist() {
        let ok = json!([{"id": "tab-1", "url": "https://example.com/docs"}]);
        let cdp = start_fake_cdp_sequence(vec![json!([]), ok.clone(), ok, json!([])]).await;
        let tool = BrowserTool::new(cdp.port).with_settle(std::time::Duration::ZERO, 5);
        tool.execute(json!({"action": "navigate", "url": "https://example.com/docs"}))
            .await
            .unwrap();
        assert_eq!(tool.owned_tabs.lock().len(), 1);

        tool.execute(json!({"action": "snapshot"})).await.unwrap();

        assert!(tool.owned_tabs.lock().is_empty());
    }

    #[test]
    fn reserved_and_embedded_ipv4_ranges_are_internal() {
        for ip in [
            "192.0.0.8",
            "198.18.0.1",
            "198.19.255.255",
            "224.0.0.1",
            "240.0.0.1",
        ] {
            assert!(is_internal_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "ff02::1",
            "64:ff9b::a00:1",
            "64:ff9b::7f00:1",
            "2002:c0a8:101::1",
        ] {
            assert!(is_internal_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "64:ff9b::808:808", "2002:0808:0808::1"] {
            assert!(!is_internal_ip(ip.parse().unwrap()), "{ip}");
        }
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
        let tool = BrowserTool::new(cdp.port).with_settle(std::time::Duration::ZERO, 5);
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
