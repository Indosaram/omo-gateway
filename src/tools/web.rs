use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use serde_json::{json, Value};

use super::Tool;
use crate::OmonError;

#[derive(Clone, Default)]
pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the live web for current information, documentation, news, or facts using DuckDuckGo search API."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query to execute."
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of search results to return (default 5)."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, OmonError> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| OmonError::ToolExecution("missing 'query'".into()))?;
        let max_results = args
            .get("max_results")
            .and_then(Value::as_u64)
            .unwrap_or(5)
            .clamp(1, 100) as usize;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .redirect(bounded_redirect_policy())
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .build()
            .map_err(|e| OmonError::ToolExecution(e.to_string()))?;

        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| OmonError::ToolExecution(format!("search request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(OmonError::ToolExecution(format!(
                "search provider returned {}",
                resp.status()
            )));
        }
        let html = read_bounded_text(resp.bytes_stream(), MAX_SEARCH_CHARS).await?;

        let results = parse_duckduckgo_results(&html, max_results);

        if results.is_empty() {
            return Err(OmonError::ToolExecution(
                "search provider response contained no parseable results".into(),
            ));
        }

        Ok(json!({
            "query": query,
            "count": results.len(),
            "results": results
        }))
    }
}

#[derive(Clone, Default)]
pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch URL content and extract clean readable text/markdown from any webpage."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch."
                },
                "max_chars": {
                    "type": "integer",
                    "description": "Maximum characters to return (default 8000)."
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, OmonError> {
        let url = args
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| OmonError::ToolExecution("missing 'url'".into()))?;
        let max_chars = args
            .get("max_chars")
            .and_then(Value::as_u64)
            .unwrap_or(100_000)
            .clamp(1, 1_000_000) as usize;
        let parsed = reqwest::Url::parse(url)
            .map_err(|error| OmonError::ToolExecution(format!("invalid URL: {error}")))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(OmonError::ToolExecution(
                "web fetch only supports http and https URLs".into(),
            ));
        }
        let reader_prefix = std::env::var(READER_PREFIX_ENV).ok();
        let fetch_url = fetch_target_url(&parsed, reader_prefix.as_deref());
        ensure_public_destination(&parsed).await?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .redirect(bounded_redirect_policy())
            .dns_resolver(std::sync::Arc::new(PublicOnlyResolver))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .build()
            .map_err(|e| OmonError::ToolExecution(e.to_string()))?;

        let resp = client
            .get(&fetch_url)
            .send()
            .await
            .map_err(|e| OmonError::ToolExecution(format!("fetch request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(OmonError::ToolExecution(format!(
                "fetch provider returned {}",
                resp.status()
            )));
        }
        let truncated = read_bounded_text(resp.bytes_stream(), max_chars).await?;

        Ok(json!({
            "url": url,
            "length": truncated.chars().count(),
            "content": truncated
        }))
    }
}

fn extract_attribute(html: &str, attr_prefix: &str, end_char: &str) -> Option<String> {
    let start = html.find(attr_prefix)? + attr_prefix.len();
    let rest = &html[start..];
    let end = rest.find(end_char)?;
    Some(rest[..end].to_string())
}

/// Finds the first element carrying CSS class `class` (alongside any other
/// classes or attributes) and returns its opening-tag text plus inner HTML.
fn element_with_class<'a>(html: &'a str, class: &str) -> Option<(&'a str, &'a str)> {
    let mut search_from = 0usize;
    while let Some(relative) = html[search_from..].find(class) {
        let pos = search_from + relative;
        search_from = pos + class.len();
        let before_ok = html[..pos].ends_with('"') || html[..pos].ends_with(' ');
        let after_ok = html[pos + class.len()..].starts_with(['"', ' ']);
        if !before_ok || !after_ok {
            continue;
        }
        let tag_start = html[..pos].rfind('<')?;
        let rest = &html[tag_start..];
        let open_end = rest.find('>')?;
        let open_tag = &rest[..open_end];
        if !open_tag.contains("class=") {
            continue;
        }
        let tag_name: String = open_tag[1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        if tag_name.is_empty() {
            continue;
        }
        let inner_start = open_end + 1;
        let end_tag = format!("</{tag_name}>");
        let inner_end = rest[inner_start..].find(&end_tag)?;
        return Some((open_tag, &rest[inner_start..inner_start + inner_end]));
    }
    None
}

/// Resolves a DuckDuckGo result href. Organic results are wrapped in a
/// `//duckduckgo.com/l/?uddg=<percent-encoded target>` redirect; ad and other
/// DuckDuckGo-internal links resolve to `None`.
fn resolve_duckduckgo_href(raw: &str) -> Option<String> {
    let unescaped = raw.replace("&amp;", "&");
    let absolute = if let Some(rest) = unescaped.strip_prefix("//") {
        format!("https://{rest}")
    } else {
        unescaped
    };
    let parsed = reqwest::Url::parse(&absolute).ok()?;
    let host = parsed.host_str().unwrap_or_default();
    if host == "duckduckgo.com" || host.ends_with(".duckduckgo.com") {
        let target = parsed
            .query_pairs()
            .find(|(key, _)| key == "uddg")
            .map(|(_, value)| value.into_owned())?;
        let target_url = reqwest::Url::parse(&target).ok()?;
        return matches!(target_url.scheme(), "http" | "https").then_some(target);
    }
    matches!(parsed.scheme(), "http" | "https").then(|| parsed.to_string())
}

/// Parses DuckDuckGo's HTML endpoint. Each result's container carries
/// `result__body` among several classes; the title and link live on the
/// `result__a` anchor.
fn parse_duckduckgo_results(html: &str, max_results: usize) -> Vec<Value> {
    let mut results = Vec::new();
    for block in html.split("result__body").skip(1) {
        if results.len() >= max_results {
            break;
        }
        let Some((open_tag, title_html)) = element_with_class(block, "result__a") else {
            continue;
        };
        let Some(url) = extract_attribute(open_tag, "href=\"", "\"")
            .as_deref()
            .and_then(resolve_duckduckgo_href)
        else {
            continue;
        };
        let snippet_html = element_with_class(block, "result__snippet")
            .map(|(_, inner)| inner)
            .unwrap_or_default();
        let title = clean_html(title_html);
        let snippet = clean_html(snippet_html);
        if title.is_empty() && snippet.is_empty() {
            continue;
        }
        results.push(json!({
            "title": title,
            "snippet": snippet,
            "url": url
        }));
    }
    results
}

const MAX_REDIRECTS: usize = 5;
/// The raw page is fetched directly by default so that no third party learns
/// which URLs (including internal ones) the agent reads. A reader/proxy service
/// is opt-in: set `OMON_WEB_READER_PREFIX` (e.g. `https://r.jina.ai/`) to route
/// fetches through it, accepting that the operator discloses every fetched URL
/// to that service and lets it control the text injected into model context.
const READER_PREFIX_ENV: &str = "OMON_WEB_READER_PREFIX";
const MAX_SEARCH_CHARS: usize = 500_000;

fn fetch_target_url(parsed: &reqwest::Url, reader_prefix: Option<&str>) -> String {
    match reader_prefix {
        Some(prefix) if !prefix.trim().is_empty() => format!("{}{parsed}", prefix.trim()),
        _ => parsed.to_string(),
    }
}

fn redirect_is_allowed(next: &reqwest::Url, hops: usize) -> bool {
    hops <= MAX_REDIRECTS
        && matches!(next.scheme(), "http" | "https")
        && next
            .host_str()
            .is_some_and(|host| !is_internal_host_literal(host))
}

fn is_internal_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
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
        || (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfe80
}

fn is_internal_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_internal_ipv4(v4),
        IpAddr::V6(v6) => is_internal_ipv6(v6),
    }
}

fn strip_brackets(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(host)
}

/// IP literals and `localhost` names; resolved names are checked by
/// `PublicOnlyResolver` on every connection, redirect hops included.
fn is_internal_host_literal(host: &str) -> bool {
    let lowered = host.trim_end_matches('.').to_ascii_lowercase();
    let bare = strip_brackets(&lowered);
    if let Ok(ip) = bare.parse::<IpAddr>() {
        return is_internal_ip(ip);
    }
    bare == "localhost" || bare.ends_with(".localhost")
}

fn internal_address_error(host: &str) -> OmonError {
    OmonError::ToolExecution(format!("web fetch to internal address `{host}` is refused"))
}

/// Blocks SSRF on the requested URL before any request is sent.
async fn ensure_public_destination(url: &reqwest::Url) -> Result<(), OmonError> {
    let host = url
        .host_str()
        .ok_or_else(|| OmonError::ToolExecution("web fetch requires a URL with a host".into()))?;
    if is_internal_host_literal(host) {
        return Err(internal_address_error(host));
    }
    let bare = strip_brackets(host);
    if bare.parse::<IpAddr>().is_ok() {
        return Ok(());
    }
    let port = url.port_or_known_default().unwrap_or(80);
    let resolved = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::net::lookup_host((bare, port)),
    )
    .await
    .map_err(|_| OmonError::ToolExecution(format!("timed out resolving `{host}`")))?
    .map_err(|error| OmonError::ToolExecution(format!("could not resolve `{host}`: {error}")))?;
    for addr in resolved {
        if is_internal_ip(addr.ip()) {
            return Err(internal_address_error(host));
        }
    }
    Ok(())
}

/// DNS resolver that refuses names resolving to internal addresses, so a
/// redirect hop (or DNS rebinding) cannot reach loopback/private/link-local.
struct PublicOnlyResolver;

impl reqwest::dns::Resolve for PublicOnlyResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        Box::pin(async move {
            let host = name.as_str().to_owned();
            let addrs: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            if addrs.iter().any(|addr| is_internal_ip(addr.ip())) {
                return Err(Box::new(internal_address_error(&host))
                    as Box<dyn std::error::Error + Send + Sync>);
            }
            Ok(Box::new(addrs.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

/// Every redirect hop is re-validated: the scheme check on the original URL
/// only ever covers hop zero.
fn bounded_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if redirect_is_allowed(attempt.url(), attempt.previous().len()) {
            attempt.follow()
        } else {
            let message = format!(
                "blocked redirect to {} after {} hops",
                attempt.url(),
                attempt.previous().len()
            );
            attempt.error(message)
        }
    })
}

/// Streams the body and stops as soon as `max_chars` characters have been
/// decoded, so an oversized response is never fully buffered. Bytes are decoded
/// incrementally at character boundaries; a multi-byte sequence split across two
/// network chunks is held until it completes instead of being corrupted.
async fn read_bounded_text<S, B, E>(stream: S, max_chars: usize) -> Result<String, OmonError>
where
    S: Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    E: std::fmt::Display,
{
    let mut stream = std::pin::pin!(stream);
    let mut pending: Vec<u8> = Vec::new();
    let mut text = String::new();
    let mut remaining = max_chars;
    while remaining > 0 {
        let Some(chunk) = stream.next().await else {
            break;
        };
        let chunk = chunk.map_err(|error| {
            OmonError::ToolExecution(format!("failed to read response: {error}"))
        })?;
        pending.extend_from_slice(chunk.as_ref());
        push_decoded(&mut pending, &mut text, &mut remaining);
    }
    if remaining > 0 && !pending.is_empty() {
        // Trailing truncated sequence at end of stream.
        text.push(char::REPLACEMENT_CHARACTER);
    }
    Ok(text)
}

fn push_decoded(pending: &mut Vec<u8>, out: &mut String, remaining: &mut usize) {
    let mut consumed = 0usize;
    while consumed < pending.len() && *remaining > 0 {
        match std::str::from_utf8(&pending[consumed..]) {
            Ok(valid) => {
                consumed += push_chars(valid, out, remaining);
                break;
            }
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                let valid =
                    std::str::from_utf8(&pending[consumed..consumed + valid_up_to]).unwrap_or("");
                let taken = push_chars(valid, out, remaining);
                consumed += taken;
                if taken < valid_up_to || *remaining == 0 {
                    break;
                }
                match error.error_len() {
                    Some(len) => {
                        out.push(char::REPLACEMENT_CHARACTER);
                        *remaining = remaining.saturating_sub(1);
                        consumed += len;
                    }
                    // Incomplete sequence at the chunk boundary: wait for more bytes.
                    None => break,
                }
            }
        }
    }
    pending.drain(..consumed);
}

fn push_chars(input: &str, out: &mut String, remaining: &mut usize) -> usize {
    let mut used = 0usize;
    for character in input.chars() {
        if *remaining == 0 {
            break;
        }
        out.push(character);
        *remaining -= 1;
        used += character.len_utf8();
    }
    used
}

fn clean_html(input: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in input.chars() {
        if c == '<' {
            inside = true;
        } else if c == '>' {
            inside = false;
        } else if !inside {
            out.push(c);
        }
    }
    out.replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    const DDG_FIXTURE: &str = r#"
<div class="result results_links results_links_deep web-result ">
<div class="links_main links_deep result__body">
<h2 class="result__title"><a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fa%3Fx%3D1%26y%3D2&amp;rut=abc">Example <b>Title</b> &#x27;one&#x27;</a></h2>
<div class="result__extras"><a class="result__url" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fa&amp;rut=abc">example.com/a</a></div>
<a class="result__snippet" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fa&amp;rut=abc">First <b>snippet</b> &amp; more</a>
</div></div>
<div class="result results_links results_links_deep result--ad">
<div class="links_main links_deep result__body">
<h2 class="result__title"><a rel="nofollow" class="result__a" href="https://duckduckgo.com/y.js?ad_provider=x">Sponsored</a></h2>
<a class="result__snippet" href="https://duckduckgo.com/y.js?ad_provider=x">buy now</a>
</div></div>
<div class="result results_links results_links_deep web-result ">
<div class="links_main links_deep result__body">
<h2 class="result__title"><a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.org%2Fb&amp;rut=def">Second</a></h2>
<a class="result__snippet" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.org%2Fb&amp;rut=def">Second snippet</a>
</div></div>
"#;

    #[test]
    fn duckduckgo_results_parse_live_markup_with_decoded_urls() {
        let results = parse_duckduckgo_results(DDG_FIXTURE, 10);

        assert_eq!(results.len(), 2, "ad redirect must be skipped: {results:?}");
        assert_eq!(results[0]["title"], "Example Title 'one'");
        assert_eq!(results[0]["url"], "https://example.com/a?x=1&y=2");
        assert_eq!(results[0]["snippet"], "First snippet & more");
        assert_eq!(results[1]["title"], "Second");
        assert_eq!(results[1]["url"], "https://example.org/b");
    }

    #[test]
    fn duckduckgo_results_honor_max_results() {
        let results = parse_duckduckgo_results(DDG_FIXTURE, 1);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["url"], "https://example.com/a?x=1&y=2");
    }

    #[test]
    fn duckduckgo_results_reject_non_http_targets() {
        let html = r#"<div class="links_main result__body"><a class="result__a" href="//duckduckgo.com/l/?uddg=javascript%3Aalert(1)">x</a></div>"#;

        assert!(parse_duckduckgo_results(html, 5).is_empty());
    }

    #[test]
    fn web_fetch_targets_url_directly_unless_reader_configured() {
        let parsed = reqwest::Url::parse("https://internal.corp.example/secret").unwrap();

        assert_eq!(
            fetch_target_url(&parsed, None),
            "https://internal.corp.example/secret",
            "default fetch must not disclose the URL to a third-party reader service"
        );
        assert_eq!(
            fetch_target_url(&parsed, Some("https://r.jina.ai/")),
            "https://r.jina.ai/https://internal.corp.example/secret",
            "an explicitly configured reader service is still honoured"
        );
    }

    #[test]
    fn redirect_hops_revalidate_scheme_and_are_capped() {
        let https = reqwest::Url::parse("https://example.com/a").unwrap();
        let http = reqwest::Url::parse("http://example.com/a").unwrap();
        let file = reqwest::Url::parse("file:///etc/passwd").unwrap();

        assert!(redirect_is_allowed(&https, 1));
        assert!(redirect_is_allowed(&http, 1));
        assert!(
            !redirect_is_allowed(&file, 1),
            "redirect hops must re-validate the scheme"
        );
        assert!(
            !redirect_is_allowed(&https, MAX_REDIRECTS + 1),
            "redirect chains must be capped"
        );
    }

    #[tokio::test]
    async fn read_bounded_text_stops_reading_once_max_chars_reached() {
        let polled = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&polled);
        let chunks: Vec<Result<Vec<u8>, std::io::Error>> =
            (0..100).map(|_| Ok(vec![b'a'; 1024])).collect();
        let stream = futures_util::stream::iter(chunks).map(move |item| {
            counter.fetch_add(1, Ordering::SeqCst);
            item
        });

        let text = read_bounded_text(stream, 2048).await.unwrap();

        assert_eq!(text.chars().count(), 2048);
        assert!(
            polled.load(Ordering::SeqCst) < 100,
            "body must stop streaming at max_chars, polled {} of 100 chunks",
            polled.load(Ordering::SeqCst)
        );
    }

    #[tokio::test]
    async fn read_bounded_text_invalid_byte_after_budget_does_not_underflow() {
        let chunks: Vec<Result<Vec<u8>, std::io::Error>> = vec![Ok(b"a\xffb".to_vec())];

        let text = read_bounded_text(futures_util::stream::iter(chunks), 1)
            .await
            .unwrap();

        assert_eq!(text, "a");
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

    #[test]
    fn redirect_hops_to_internal_addresses_are_blocked() {
        for target in [
            "http://127.0.0.1/",
            "http://10.0.0.5/",
            "http://169.254.169.254/latest/meta-data/",
            "http://[fe80::1]/",
            "http://localhost/",
        ] {
            let url = reqwest::Url::parse(target).unwrap();
            assert!(
                !redirect_is_allowed(&url, 1),
                "redirect to {target} must be blocked"
            );
        }
    }

    #[tokio::test]
    async fn read_bounded_text_decodes_multibyte_split_across_chunks() {
        let encoded = "한글".as_bytes().to_vec();
        let (head, tail) = encoded.split_at(4);
        let chunks: Vec<Result<Vec<u8>, std::io::Error>> =
            vec![Ok(head.to_vec()), Ok(tail.to_vec())];

        let text = read_bounded_text(futures_util::stream::iter(chunks), 100)
            .await
            .unwrap();

        assert_eq!(text, "한글");
    }
}
