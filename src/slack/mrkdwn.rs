//! Markdown to Slack mrkdwn conversion and Slack-sized message splitting.

use std::sync::LazyLock;

use regex::{Captures, Regex};

const PLACEHOLDER_OPEN: char = '\u{E000}';
const PLACEHOLDER_CLOSE: char = '\u{E001}';
const BOLD_MARK: char = '\u{E002}';
const FENCE: &str = "```";
const FENCE_CLOSE: &str = "\n```";
/// Most messages one reply may post; the rest is cut with [`TRUNCATION_NOTICE`].
pub const MAX_MESSAGES: usize = 8;
pub const TRUNCATION_NOTICE: &str =
    "\n\n_… [Response truncated: output exceeded the Slack 8-message limit]_";

static BROADCAST_MENTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<!(?:here|channel|everyone|subteam\^[^<>|]*)(?:\|[^<>]*)?>").unwrap()
});

static SLACK_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<(?:[@#!][^<>\s|]+(?:\|[^<>]*)?|(?:https?|mailto):[^<>\s|]+(?:\|[^<>]*)?)>")
        .unwrap()
});
static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"!?\[([^\]\n]*)\]\(<?([^()\s<>]+)>?(?:\s+"[^"\n]*")?\)"#).unwrap()
});
static STAR_BOLD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\*\*(\S(?:[^\n]*?\S)?)\*\*").unwrap());
static UNDERSCORE_BOLD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"__(\S(?:[^\n]*?\S)?)__").unwrap());
static ITALIC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\*([^\s*](?:[^*\n]*?[^\s*])?)\*").unwrap());
static STRIKE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"~~(\S(?:[^\n]*?\S)?)~~").unwrap());
static PLACEHOLDER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x{E000}(\d+)\x{E001}").unwrap());
static HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s{0,3}#{1,6}\s+(.*?)(?:\s+#+)?\s*$").unwrap());
static QUOTE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*(?:>\s?)+)(.*)$").unwrap());
static BULLET_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)[-*+]\s+(.*)$").unwrap());
static TABLE_SEPARATOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*\|?\s*:?-+:?\s*(?:\|\s*:?-+:?\s*)*\|?\s*$").unwrap()
});

/// Converts common Markdown to Slack mrkdwn. Code (fenced and inline) is left verbatim,
/// fence language tags are dropped and Markdown tables become code blocks.
pub fn to_mrkdwn(markdown: &str) -> String {
    let lines: Vec<&str> = markdown
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut in_fence = false;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if is_fence(line) {
            out.push(FENCE.to_owned());
            in_fence = !in_fence;
            index += 1;
        } else if in_fence {
            out.push(line.to_owned());
            index += 1;
        } else if is_table_start(&lines, index) {
            out.push(FENCE.to_owned());
            while index < lines.len() && is_table_row(lines[index]) {
                out.push(lines[index].trim_end().to_owned());
                index += 1;
            }
            out.push(FENCE.to_owned());
        } else {
            out.push(convert_line(line));
            index += 1;
        }
    }
    if in_fence {
        out.push(FENCE.to_owned());
    }
    neutralize_broadcast_mentions(&out.join("\n"))
}

/// Escapes `<!here>`, `<!channel>`, `<!everyone>` and `<!subteam^…>` so a reply can never ping
/// a whole channel or user group; user mentions (`<@U…>`) are left intact.
pub fn neutralize_broadcast_mentions(text: &str) -> String {
    BROADCAST_MENTION_RE
        .replace_all(text, |c: &Captures| {
            c[0].replace('<', "&lt;").replace('>', "&gt;")
        })
        .into_owned()
}

/// Keeps at most `max_messages` chunks; when some are dropped the last kept chunk is trimmed to
/// fit `limit`, its open code block closed, and [`TRUNCATION_NOTICE`] appended.
pub fn bound_chunks(mut chunks: Vec<String>, max_messages: usize, limit: usize) -> Vec<String> {
    if max_messages == 0 || chunks.len() <= max_messages {
        return chunks;
    }
    chunks.truncate(max_messages);
    if let Some(last) = chunks.last_mut() {
        let budget = limit.saturating_sub(TRUNCATION_NOTICE.len() + FENCE_CLOSE.len());
        if last.len() > budget {
            let mut end = budget;
            while end > 0 && !last.is_char_boundary(end) {
                end -= 1;
            }
            last.truncate(end);
        }
        if last.split('\n').filter(|line| is_fence(line)).count() % 2 == 1 {
            last.push_str(FENCE_CLOSE);
        }
        last.push_str(TRUNCATION_NOTICE);
    }
    chunks
}

/// Splits `text` into chunks of at most `limit` bytes on line boundaries. A chunk that ends
/// inside a code block is closed and the block is reopened at the start of the next chunk;
/// lines longer than the available room are hard-split.
pub fn split_message(text: &str, limit: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut splitter = Splitter {
        limit,
        chunks: Vec::new(),
        current: String::new(),
        lines: 0,
        open: None,
        open_at: 0,
        open_has_content: false,
    };
    for line in text.split('\n') {
        let fence_line = is_fence(line);
        let budget = match splitter.open {
            Some(open) => limit.saturating_sub(open.len() + 1 + FENCE_CLOSE.len()),
            None => limit,
        }
        .max(1);
        if fence_line || line.len() <= budget {
            splitter.push(line, fence_line);
        } else {
            for piece in hard_split(line, budget) {
                splitter.push(piece, false);
            }
        }
    }
    splitter.finish()
}

struct Splitter<'a> {
    limit: usize,
    chunks: Vec<String>,
    current: String,
    lines: usize,
    open: Option<&'a str>,
    open_at: usize,
    open_has_content: bool,
}

impl<'a> Splitter<'a> {
    fn push(&mut self, mut piece: &'a str, fence_line: bool) {
        let next_open = match (fence_line, self.open) {
            (true, Some(_)) => None,
            (true, None) => Some(piece),
            (false, open) => open,
        };
        let separator = usize::from(self.lines > 0);
        let reserve = if next_open.is_some() { FENCE_CLOSE.len() } else { 0 };
        if self.lines > 0 && self.current.len() + separator + piece.len() + reserve > self.limit {
            if fence_line && self.open.is_some() {
                piece = FENCE;
            } else {
                self.flush();
            }
        }
        if self.lines > 0 {
            self.current.push('\n');
        }
        if fence_line && self.open.is_none() {
            self.open_at = self.current.len();
            self.open_has_content = false;
        } else if self.open.is_some() {
            self.open_has_content = true;
        }
        self.current.push_str(piece);
        self.lines += 1;
        self.open = next_open;
    }

    fn flush(&mut self) {
        let mut keep = true;
        if self.open.is_some() {
            if self.open_has_content {
                self.current.push_str(FENCE_CLOSE);
            } else {
                keep = self.open_at > 0;
                self.current.truncate(self.open_at.saturating_sub(1));
            }
        }
        let chunk = std::mem::take(&mut self.current);
        if keep {
            self.chunks.push(chunk);
        }
        self.lines = 0;
        if let Some(open) = self.open {
            self.current.push_str(open);
            self.lines = 1;
            self.open_at = 0;
            self.open_has_content = false;
        }
    }

    fn finish(mut self) -> Vec<String> {
        if self.lines > 0 {
            if self.open.is_some() {
                self.current.push_str(FENCE_CLOSE);
            }
            self.chunks.push(self.current);
        }
        self.chunks
    }
}

fn hard_split(line: &str, budget: usize) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut rest = line;
    while rest.len() > budget {
        let mut cut = budget;
        while !rest.is_char_boundary(cut) {
            cut -= 1;
        }
        if cut == 0 {
            cut = rest.chars().next().map_or(rest.len(), char::len_utf8);
        } else if let Some(space) = rest[..cut].rfind(' ').filter(|&at| at >= cut / 2) {
            cut = space + 1;
        }
        pieces.push(&rest[..cut]);
        rest = &rest[cut..];
    }
    pieces.push(rest);
    pieces
}

fn is_fence(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with(FENCE) && !trimmed[FENCE.len()..].contains(FENCE)
}

fn is_table_row(line: &str) -> bool {
    !line.trim().is_empty() && line.contains('|')
}

fn is_table_start(lines: &[&str], index: usize) -> bool {
    is_table_row(lines[index])
        && lines
            .get(index + 1)
            .is_some_and(|next| next.contains('|') && TABLE_SEPARATOR_RE.is_match(next))
}

fn convert_line(line: &str) -> String {
    if let Some(captures) = HEADING_RE.captures(line) {
        let inner = convert_inline(&captures[1].replace("**", "").replace("__", ""));
        return if inner.is_empty() {
            String::new()
        } else {
            format!("*{inner}*")
        };
    }
    if let Some(captures) = QUOTE_RE.captures(line) {
        return format!("{}{}", &captures[1], convert_line(&captures[2]));
    }
    if let Some(captures) = BULLET_RE.captures(line) {
        return format!("{}• {}", &captures[1], convert_inline(&captures[2]));
    }
    convert_inline(line)
}

fn convert_inline(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .filter(|c| !matches!(*c, PLACEHOLDER_OPEN | PLACEHOLDER_CLOSE | BOLD_MARK))
        .collect();
    let mut store = Vec::new();
    let text = stash_code_spans(&cleaned, &mut store);
    let text = SLACK_TOKEN_RE
        .replace_all(&text, |c: &Captures| stash(&mut store, c[0].to_owned()))
        .into_owned();
    let text = LINK_RE
        .replace_all(&text, |c: &Captures| {
            let url = c[2].replace('&', "&amp;");
            let label = escape(c[1].trim());
            let link = if label.is_empty() || label == url {
                format!("<{url}>")
            } else {
                format!("<{url}|{label}>")
            };
            stash(&mut store, link)
        })
        .into_owned();
    let text = STAR_BOLD_RE.replace_all(&text, format!("{BOLD_MARK}$1{BOLD_MARK}").as_str());
    let text = UNDERSCORE_BOLD_RE.replace_all(&text, format!("{BOLD_MARK}$1{BOLD_MARK}").as_str());
    let text = ITALIC_RE.replace_all(&text, "_${1}_");
    let text = STRIKE_RE.replace_all(&text, "~$1~");
    let mut text = escape(&text).replace(BOLD_MARK, "*");
    for _ in 0..=store.len() {
        if !text.contains(PLACEHOLDER_OPEN) {
            break;
        }
        text = PLACEHOLDER_RE
            .replace_all(&text, |c: &Captures| {
                c[1].parse::<usize>()
                    .ok()
                    .and_then(|index| store.get(index))
                    .cloned()
                    .unwrap_or_default()
            })
            .into_owned();
    }
    text
}

fn stash(store: &mut Vec<String>, value: String) -> String {
    store.push(value);
    format!("{PLACEHOLDER_OPEN}{}{PLACEHOLDER_CLOSE}", store.len() - 1)
}

fn stash_code_spans(text: &str, store: &mut Vec<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let run = backtick_run(&rest[start..]);
        let after = &rest[start + run..];
        if let Some(end) = find_backtick_run(after, run) {
            out.push_str(&rest[..start]);
            let ticks = &rest[start..start + run];
            out.push_str(&stash(store, format!("{ticks}{}{ticks}", &after[..end])));
            rest = &after[end + run..];
        } else {
            out.push_str(&rest[..start + run]);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn backtick_run(text: &str) -> usize {
    text.bytes().take_while(|b| *b == b'`').count()
}

fn find_backtick_run(text: &str, run: usize) -> Option<usize> {
    let mut index = 0;
    while index < text.len() {
        if text.as_bytes()[index] == b'`' {
            let len = backtick_run(&text[index..]);
            if len == run {
                return Some(index);
            }
            index += len;
        } else {
            index += 1;
        }
    }
    None
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fences_balanced(chunk: &str) -> bool {
        chunk.split('\n').filter(|line| is_fence(line)).count() % 2 == 0
    }

    fn body(text: &str) -> String {
        text.split('\n')
            .filter(|line| !is_fence(line))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn assert_split_invariants(chunks: &[String], limit: usize) {
        assert!(!chunks.is_empty());
        for chunk in chunks {
            assert!(chunk.len() <= limit, "chunk of {} bytes over {limit}: {chunk:?}", chunk.len());
            assert!(fences_balanced(chunk), "unbalanced fences: {chunk:?}");
        }
    }

    #[test]
    fn converts_bold_italic_and_strike() {
        assert_eq!(to_mrkdwn("**bold** and __also__"), "*bold* and *also*");
        assert_eq!(to_mrkdwn("an *italic* word"), "an _italic_ word");
        assert_eq!(to_mrkdwn("**b** then *i*"), "*b* then _i_");
        assert_eq!(to_mrkdwn("~~gone~~"), "~gone~");
        assert_eq!(to_mrkdwn("2 * 3 * 4"), "2 * 3 * 4");
        assert_eq!(to_mrkdwn("_already_ slack"), "_already_ slack");
    }

    #[test]
    fn converts_links_and_escapes_url_ampersand() {
        assert_eq!(
            to_mrkdwn("see [docs](https://e.com/a?x=1&y=2) now"),
            "see <https://e.com/a?x=1&amp;y=2|docs> now"
        );
        assert_eq!(to_mrkdwn("[](https://e.com)"), "<https://e.com>");
        assert_eq!(
            to_mrkdwn("**see [a <b>](https://e.com)**"),
            "*see <https://e.com|a &lt;b&gt;>*"
        );
    }

    #[test]
    fn converts_headings_and_bullets() {
        assert_eq!(to_mrkdwn("# Title"), "*Title*");
        assert_eq!(to_mrkdwn("###### Deep **bold** ##"), "*Deep bold*");
        assert_eq!(to_mrkdwn("#hashtag"), "#hashtag");
        assert_eq!(
            to_mrkdwn("- one\n* two **b**\n  - three"),
            "• one\n• two *b*\n  • three"
        );
        assert_eq!(to_mrkdwn("> quoted **x** & y"), "> quoted *x* &amp; y");
    }

    #[test]
    fn escapes_control_characters_but_keeps_slack_tokens() {
        assert_eq!(to_mrkdwn("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(to_mrkdwn("<div>"), "&lt;div&gt;");
        let tokens = "hi <@U123> in <#C1|general> <!date^1|x> <https://x.com|site> <mailto:a@b.c>";
        assert_eq!(to_mrkdwn(tokens), tokens);
    }

    #[test]
    fn neutralizes_broadcast_mentions_everywhere_but_keeps_user_mentions() {
        assert_eq!(
            to_mrkdwn("hey <!here> <!channel|@channel> <!everyone> <!subteam^S12|@devs> <@U1>"),
            "hey &lt;!here&gt; &lt;!channel|@channel&gt; &lt;!everyone&gt; &lt;!subteam^S12|@devs&gt; <@U1>"
        );
        assert_eq!(to_mrkdwn("`<!here>`"), "`&lt;!here&gt;`");
        assert_eq!(
            to_mrkdwn("```\n<!channel>\n```"),
            "```\n&lt;!channel&gt;\n```"
        );
        assert_eq!(neutralize_broadcast_mentions("<!heresy> <@U2>"), "<!heresy> <@U2>");
    }

    #[test]
    fn bound_chunks_caps_count_and_appends_notice() {
        let chunks: Vec<String> = (0..12).map(|n| format!("chunk {n}")).collect();
        let bounded = bound_chunks(chunks.clone(), MAX_MESSAGES, 100);
        assert_eq!(bounded.len(), MAX_MESSAGES);
        assert_eq!(bounded[..7], chunks[..7]);
        assert_eq!(bounded[7], format!("chunk 7{TRUNCATION_NOTICE}"));
        assert_eq!(bound_chunks(chunks[..3].to_vec(), MAX_MESSAGES, 100), chunks[..3]);
    }

    #[test]
    fn bound_chunks_trims_last_chunk_and_closes_open_fence() {
        let full = format!("```\n{}\n```", "z".repeat(200));
        let chunks = vec!["a".to_owned(), full];
        let bounded = bound_chunks(chunks, 1, 200);
        assert_eq!(bounded.len(), 1);
        assert_eq!(bounded[0], format!("a{TRUNCATION_NOTICE}"));

        let chunks = vec![format!("```\n{}\n```", "z".repeat(200)), "b".to_owned()];
        let limit = 150;
        let bounded = bound_chunks(chunks, 1, limit);
        assert!(bounded[0].len() <= limit, "{}", bounded[0].len());
        assert!(bounded[0].ends_with(TRUNCATION_NOTICE));
        let body = bounded[0].trim_end_matches(TRUNCATION_NOTICE);
        assert!(fences_balanced(body), "{body:?}");
    }

    #[test]
    fn keeps_inline_code_verbatim() {
        assert_eq!(
            to_mrkdwn("use `a **b** <c> & d` then **ok**"),
            "use `a **b** <c> & d` then *ok*"
        );
        assert_eq!(to_mrkdwn("``a `tick` b`` **x**"), "``a `tick` b`` *x*");
        assert_eq!(to_mrkdwn("```inline```\n**b**"), "```inline```\n*b*");
        assert_eq!(to_mrkdwn("a ` lone *i*"), "a ` lone _i_");
    }

    #[test]
    fn keeps_fenced_code_verbatim_and_drops_language() {
        assert_eq!(
            to_mrkdwn("before **b**\n```rust\nlet x = **y** < 1 && `z`;\n# not heading\n```\nafter"),
            "before *b*\n```\nlet x = **y** < 1 && `z`;\n# not heading\n```\nafter"
        );
        assert_eq!(to_mrkdwn("```\nopen <x>"), "```\nopen <x>\n```");
    }

    #[test]
    fn keeps_code_nested_in_list_verbatim() {
        assert_eq!(
            to_mrkdwn("- step **one**\n  ```sh\n  echo *hi* > out\n  ```\n- `x*y*z`"),
            "• step *one*\n```\n  echo *hi* > out\n```\n• `x*y*z`"
        );
    }

    #[test]
    fn wraps_tables_in_code_block() {
        assert_eq!(
            to_mrkdwn("Results:\n| a | **b** |\n|---|:-:|\n| 1 | <2> |\nafter"),
            "Results:\n```\n| a | **b** |\n|---|:-:|\n| 1 | <2> |\n```\nafter"
        );
        assert_eq!(to_mrkdwn("a | b\nno separator"), "a | b\nno separator");
    }

    #[test]
    fn split_returns_single_chunk_when_short() {
        assert_eq!(split_message("hello\nworld", 100), vec!["hello\nworld"]);
        assert!(split_message("", 100).is_empty());
    }

    #[test]
    fn split_breaks_on_line_boundaries() {
        let input = (0..20)
            .map(|n| format!("line {n:04}."))
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = split_message(&input, 35);
        assert!(chunks.len() > 1);
        assert_split_invariants(&chunks, 35);
        assert_eq!(chunks.join("\n"), input);
        for chunk in &chunks {
            assert!(chunk.split('\n').all(|line| line.len() == 10));
        }
    }

    #[test]
    fn split_closes_and_reopens_code_fence() {
        let code = (0..12)
            .map(|n| format!("code line {n:02}"))
            .collect::<Vec<_>>()
            .join("\n");
        let input = format!("intro text\n```rust\n{code}\n```\noutro");
        let chunks = split_message(&input, 60);
        assert!(chunks.len() > 2);
        assert_split_invariants(&chunks, 60);
        assert_eq!(body(&chunks.join("\n")), body(&input));
        assert!(chunks[1..].iter().any(|chunk| chunk.starts_with("```rust\n")));
        assert!(chunks.iter().all(|chunk| !chunk.contains("```rust\n```")));
    }

    #[test]
    fn split_does_not_leave_empty_fence_at_chunk_end() {
        let input = format!("{}\n```\n{}\n```", "a".repeat(20), "b".repeat(20));
        let chunks = split_message(&input, 30);
        assert_split_invariants(&chunks, 30);
        assert_eq!(chunks[0], "a".repeat(20));
        assert_eq!(chunks[1], format!("```\n{}\n```", "b".repeat(20)));
    }

    #[test]
    fn split_hard_splits_overlong_lines() {
        let input = "x".repeat(250);
        let chunks = split_message(&input, 100);
        assert_eq!(chunks.len(), 3);
        assert_split_invariants(&chunks, 100);
        assert_eq!(chunks.concat(), input);

        let words = "word ".repeat(40);
        let chunks = split_message(&words, 32);
        assert_split_invariants(&chunks, 32);
        assert!(chunks[..chunks.len() - 1].iter().all(|chunk| chunk.ends_with(' ')));
        assert_eq!(chunks.concat(), words);
    }

    #[test]
    fn split_hard_splits_inside_fence_and_multibyte_text() {
        let input = format!("```\n{}\n```", "y".repeat(300));
        let chunks = split_message(&input, 50);
        assert_split_invariants(&chunks, 50);
        assert_eq!(
            body(&chunks.join("\n")).replace('\n', ""),
            body(&input).replace('\n', "")
        );

        let korean = "가".repeat(100);
        let chunks = split_message(&korean, 50);
        assert_split_invariants(&chunks, 50);
        assert_eq!(chunks.concat(), korean);
    }

    #[test]
    fn converted_output_splits_at_message_limit() {
        let markdown = format!(
            "# Report\n{}\n```python\n{}\n```",
            "- item **bold** & more\n".repeat(200),
            "print('x')\n".repeat(400)
        );
        let converted = to_mrkdwn(&markdown);
        let chunks = split_message(&converted, crate::slack::MESSAGE_LIMIT);
        assert!(chunks.len() > 1);
        assert_split_invariants(&chunks, crate::slack::MESSAGE_LIMIT);
        assert_eq!(body(&chunks.join("\n")), body(&converted));
    }
}
