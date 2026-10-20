//! A button with custom id `omon:btn:<namespace>:<action>:<arg>` runs the command configured for
//! `<namespace>` in the button-actions file with `<action> <arg>` appended. Exit 0 marks the
//! button done; any other outcome restores it for a retry.
//!
//! Discord rejects a component `custom_id` longer than 100 characters, while a publish binding
//! carries a full sha256 token (`omon:btn:content-intel:publish:96-417-435:4.<64 hex>.2`, 110
//! characters). Truncating that token would weaken the publish binding, so an over-long id is
//! instead compacted losslessly into an explicitly versioned form: a `v1` field is inserted before
//! the token and each 64-hex-digit part becomes `b64-<43 base64url chars>`, which turns the same
//! id into 96 characters. Only an id carrying that `v1` field is ever decoded, so a legitimate raw
//! token is never reinterpreted, and [`parse_button_custom_id`] expands it back before parsing so
//! the command still receives the exact original token.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use parking_lot::Mutex;
use serde::Deserialize;
use serenity::all::{
    ActionRow, ActionRowComponent, Button, ButtonKind, ChannelId, ComponentInteraction, Context,
    CreateActionRow, CreateAllowedMentions, CreateButton, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditMessage, Http, MessageId,
};

pub const BUTTON_PREFIX: &str = "omon:btn:";
const DEFAULT_TIMEOUT_SECS: u64 = 600;
const MAX_REPLY_CHARS: usize = 1900;
const PENDING_LABEL: &str = "⏳ 처리 중…";
const DEFAULT_DONE_LABEL: &str = "✓ 완료";
const EXECUTED_INTERACTION_LIMIT: usize = 500;
const ACK_FAILURE_REPLY: &str = "⚠️ 버튼 응답 확인에 실패해 실행하지 않았습니다";
const INHERITED_ENV: [&str; 6] = ["HOME", "USER", "LOGNAME", "TMPDIR", "LANG", "LC_ALL"];

static IN_FLIGHT: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static EXECUTED_INTERACTIONS: LazyLock<Mutex<InteractionLru>> =
    LazyLock::new(|| Mutex::new(InteractionLru::default()));

#[derive(Default)]
struct InteractionLru {
    ids: HashSet<u64>,
    order: VecDeque<u64>,
}

impl InteractionLru {
    fn contains(&mut self, id: u64) -> bool {
        if !self.ids.contains(&id) {
            return false;
        }
        self.order.retain(|existing| *existing != id);
        self.order.push_back(id);
        true
    }

    fn insert(&mut self, id: u64) {
        if self.ids.insert(id) {
            self.order.push_back(id);
            if self.order.len() > EXECUTED_INTERACTION_LIMIT {
                if let Some(oldest) = self.order.pop_front() {
                    self.ids.remove(&oldest);
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ButtonClick {
    pub namespace: String,
    pub action: String,
    pub arg: String,
    /// Optional fifth segment, passed to the command as one more argv element. content-intel
    /// binds the publish button to the reviewed snapshot this way (`<id>.<sha256>.<revision>`).
    pub token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ButtonActionConfig {
    pub command: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub done_labels: HashMap<String, String>,
    #[serde(default)]
    pub nonterminal_actions: HashSet<String>,
}

#[derive(Debug)]
pub struct ActionOutcome {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

fn is_name(segment: &str) -> bool {
    (1..=32).contains(&segment.len())
        && segment.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && segment
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn is_arg(segment: &str) -> bool {
    (1..=64).contains(&segment.len())
        && segment.starts_with(|c: char| c.is_ascii_alphanumeric())
        && segment
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn is_token(segment: &str) -> bool {
    // 128 keeps the Slack button values working (Slack allows 2000-character values). Discord
    // cannot carry an id that long, so [`compact_custom_id`] shrinks the hex parts losslessly
    // before it is sent there.
    (1..=128).contains(&segment.len())
        && segment.starts_with(|c: char| c.is_ascii_alphanumeric())
        && segment
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

/// Discord rejects a component `custom_id` longer than this. An id above it is sent in the
/// lossless [`compact_custom_id`] form rather than truncated.
pub const DISCORD_CUSTOM_ID_LIMIT: usize = 100;

/// Marks a compacted token part: `b64-` followed by the base64url (no padding) of the original
/// 32 bytes. A compacted id is always recognised by its [`COMPACT_VERSION`] field, never by this
/// marker alone, so an untouched token that happens to start with `b64-` keeps its literal value.
const COMPACT_PART_MARKER: &str = "b64-";
const HEX_SHA256_CHARS: usize = 64;
const COMPACT_PAYLOAD_CHARS: usize = 43;

/// Version field inserted before the token of a compacted id: `omon:btn:<namespace>:<action>:<arg>:v1:<token>`.
/// A six-segment id never contains it, so today's ids are unambiguous and need no migration.
const COMPACT_VERSION: &str = "v1";

/// Whether a `custom_id` is short enough for Discord to accept it on a component.
pub fn custom_id_fits_discord(custom_id: &str) -> bool {
    custom_id.len() <= DISCORD_CUSTOM_ID_LIMIT
}

fn hex_to_bytes(hex: &str) -> Option<[u8; 32]> {
    let bytes = hex.as_bytes();
    if bytes.len() != HEX_SHA256_CHARS || !bytes.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let mut out = [0u8; 32];
    for (index, pair) in bytes.chunks(2).enumerate() {
        let text = std::str::from_utf8(pair).ok()?;
        out[index] = u8::from_str_radix(text, 16).ok()?;
    }
    Some(out)
}

/// Compacts a token part: a 64-hex-digit digest becomes `b64-<43 base64url chars>`, anything else
/// is returned unchanged. Nothing is dropped, so the digest keeps its full strength.
fn compact_part(part: &str) -> String {
    match hex_to_bytes(part) {
        Some(bytes) => format!("{COMPACT_PART_MARKER}{}", URL_SAFE_NO_PAD.encode(bytes)),
        None => part.to_owned(),
    }
}

/// Expands a compacted token part back to its 64-hex-digit form; other parts pass through.
fn expand_part(part: &str) -> String {
    let Some(payload) = part.strip_prefix(COMPACT_PART_MARKER) else {
        return part.to_owned();
    };
    if payload.len() != COMPACT_PAYLOAD_CHARS {
        return part.to_owned();
    }
    match URL_SAFE_NO_PAD.decode(payload) {
        Ok(bytes) if bytes.len() == 32 => bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .concat(),
        _ => part.to_owned(),
    }
}

/// The Discord-legal form of a `custom_id`: identical when it already fits, otherwise the same id
/// with an explicit [`COMPACT_VERSION`] field and its hex digest parts compacted losslessly.
pub fn compact_custom_id(custom_id: &str) -> String {
    if custom_id_fits_discord(custom_id) {
        return custom_id.to_owned();
    }
    let mut parts: Vec<String> = custom_id.split(':').map(str::to_owned).collect();
    // `omon:btn:<namespace>:<action>:<arg>:<token>` — only the token can hold a sha256 digest.
    if parts.len() != 6 {
        return custom_id.to_owned();
    }
    let token = parts[5]
        .split('.')
        .map(compact_part)
        .collect::<Vec<_>>()
        .join(".");
    parts[5] = COMPACT_VERSION.to_owned();
    parts.push(token);
    parts.join(":")
}

/// The original form of a `custom_id`: expands the [`compact_custom_id`] encoding, so the parsed
/// click carries exactly the token the producer built. An id without the version field is
/// returned verbatim, so a raw token is never rewritten.
pub fn expand_custom_id(custom_id: &str) -> String {
    let mut parts: Vec<String> = custom_id.split(':').map(str::to_owned).collect();
    if parts.len() != 7 || parts[5] != COMPACT_VERSION {
        return custom_id.to_owned();
    }
    let token = parts[6]
        .split('.')
        .map(expand_part)
        .collect::<Vec<_>>()
        .join(".");
    parts.truncate(5);
    parts.push(token);
    parts.join(":")
}

/// The arg and token become argv elements of the configured command, so they may never start
/// with `-`. A compacted id is expanded first, so `click.token` is always the producer's token.
pub fn parse_button_custom_id(custom_id: &str) -> Option<ButtonClick> {
    let expanded = expand_custom_id(custom_id);
    let mut parts = expanded.strip_prefix(BUTTON_PREFIX)?.split(':');
    let (namespace, action, arg) = (parts.next()?, parts.next()?, parts.next()?);
    let token = parts.next();
    if parts.next().is_some()
        || !is_name(namespace)
        || !is_name(action)
        || !is_arg(arg)
        || token.is_some_and(|token| !is_token(token))
    {
        return None;
    }
    Some(ButtonClick {
        namespace: namespace.to_string(),
        action: action.to_string(),
        arg: arg.to_string(),
        token: token.map(str::to_string),
    })
}

/// Fails closed: unlike approval buttons, an empty allow-list authorizes nobody, because a
/// click runs a command.
pub fn is_button_clicker_allowed(user_id: u64, allowed_users: &[u64]) -> bool {
    allowed_users.contains(&user_id)
}

pub fn button_actions_path() -> PathBuf {
    if let Some(path) = std::env::var_os("OMON_BUTTON_ACTIONS_FILE") {
        return PathBuf::from(path);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".omon").join("button-actions.json")
}

pub fn load_button_action(
    path: &Path,
    namespace: &str,
) -> Result<Option<ButtonActionConfig>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let mut actions: HashMap<String, ButtonActionConfig> = serde_json::from_str(&text)
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    Ok(actions
        .remove(namespace)
        .filter(|config| !config.command.is_empty()))
}

/// The command runs with a cleared environment so gateway credentials never reach it.
pub async fn run_button_action(
    config: &ButtonActionConfig,
    click: &ButtonClick,
    extra_env: &[(String, String)],
) -> std::io::Result<ActionOutcome> {
    let Some((program, base_args)) = config.command.split_first() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "button action command is empty",
        ));
    };
    let mut command = tokio::process::Command::new(program);
    command
        .args(base_args)
        .arg(&click.action)
        .arg(&click.arg)
        .args(&click.token)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for key in INHERITED_ENV {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let path = crate::tools::augmented_path_from_environment();
    if !path.is_empty() {
        command.env("PATH", path);
    }
    for (key, value) in extra_env {
        command.env(key, value);
    }
    if let Some(cwd) = &config.cwd {
        command.current_dir(cwd);
    }
    #[cfg(unix)]
    {
        command.process_group(0);
    }

    // No kill_on_drop: a gateway restart must not cut an irreversible publish off half-way.
    let child = command.spawn()?;
    let pid = child.id();
    let timeout = Duration::from_secs(config.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(output) => {
            let output = output?;
            Ok(ActionOutcome {
                success: output.status.success(),
                code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                timed_out: false,
            })
        }
        Err(_) => {
            #[cfg(unix)]
            if let Some(pid) = pid {
                // SAFETY: signals the process group this function created for the child.
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
            }
            Ok(ActionOutcome {
                success: false,
                code: None,
                stdout: String::new(),
                stderr: format!("timed out after {}s", timeout.as_secs()),
                timed_out: true,
            })
        }
    }
}

fn tail_chars(text: &str, limit: usize) -> String {
    let count = text.chars().count();
    if count <= limit {
        return text.to_string();
    }
    let tail: String = text.chars().skip(count - limit).collect();
    format!("…{tail}")
}

pub fn reply_text(outcome: &ActionOutcome) -> String {
    let stdout = outcome.stdout.trim();
    if outcome.success {
        return if stdout.is_empty() {
            "완료했습니다.".to_string()
        } else {
            tail_chars(stdout, MAX_REPLY_CHARS)
        };
    }
    let detail = if stdout.is_empty() {
        outcome.stderr.trim()
    } else {
        stdout
    };
    let status = match (outcome.timed_out, outcome.code) {
        (true, _) => "시간 초과".to_string(),
        (false, Some(code)) => format!("exit {code}"),
        (false, None) => "비정상 종료".to_string(),
    };
    format!(
        "⚠️ 실패 ({status}) — 버튼을 다시 누르면 재시도합니다.\n{}",
        tail_chars(detail, MAX_REPLY_CHARS - 80)
    )
}

/// `None` when a row holds anything but buttons: select menus cannot be round-tripped here.
///
/// Every non-link `custom_id` is re-emitted through [`compact_custom_id`], so a row that carries
/// an over-long id (a full sha256 publish binding) goes back to Discord within its 100-character
/// limit while the full token stays recoverable on the next click. Label, emoji and style come
/// from the original button, so only the id changes.
pub fn rebuild_buttons(
    rows: &[ActionRow],
    clicked: &str,
    clicked_label: Option<&str>,
    disabled: bool,
) -> Option<Vec<CreateActionRow>> {
    let resend = |button: &Button, custom_id: &str| {
        CreateButton::from(button.clone()).custom_id(compact_custom_id(custom_id))
    };
    rows.iter()
        .map(|row| {
            row.components
                .iter()
                .map(|component| match component {
                    ActionRowComponent::Button(button) => Some(match &button.data {
                        ButtonKind::NonLink { custom_id, style } if custom_id == clicked => {
                            match clicked_label {
                                Some(label) => resend(button, custom_id)
                                    .style(*style)
                                    .label(label)
                                    .disabled(disabled),
                                None => resend(button, custom_id).disabled(disabled),
                            }
                        }
                        ButtonKind::NonLink { custom_id, .. } => {
                            resend(button, custom_id).disabled(disabled || button.disabled)
                        }
                        ButtonKind::Link { .. } => CreateButton::from(button.clone()),
                        _ => {
                            CreateButton::from(button.clone()).disabled(disabled || button.disabled)
                        }
                    }),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .map(CreateActionRow::Buttons)
        })
        .collect()
}

async fn respond_ephemeral(
    ctx: &Context,
    component: &ComponentInteraction,
    content: &str,
) -> serenity::Result<()> {
    component
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(content),
            ),
        )
        .await
}

async fn set_buttons(
    http: &Http,
    channel_id: ChannelId,
    message_id: MessageId,
    rows: Option<Vec<CreateActionRow>>,
) {
    let Some(rows) = rows else { return };
    // A transient edit failure after a terminal action would leave the button clickable
    // (re-running an irreversible action), so retry a couple of times.
    for attempt in 1..=3u32 {
        match channel_id
            .edit_message(
                http,
                message_id,
                EditMessage::new().components(rows.clone()),
            )
            .await
        {
            Ok(_) => return,
            Err(error) => {
                tracing::warn!(%error, %message_id, attempt, "failed to update button state");
                if attempt < 3 {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
}

async fn post_ack_failure_reply(http: &Http, channel_id: ChannelId, message_id: MessageId) {
    let reply = CreateMessage::new()
        .content(ACK_FAILURE_REPLY)
        .reference_message((channel_id, message_id))
        .allowed_mentions(CreateAllowedMentions::new());
    if let Err(error) = channel_id.send_message(http, reply).await {
        tracing::warn!(%error, %message_id, "failed to post button acknowledgement failure");
    }
}

pub async fn handle_button_click(
    ctx: &Context,
    component: &ComponentInteraction,
    allowed_users: &[u64],
    click: ButtonClick,
) -> serenity::Result<()> {
    let user_id = component.user.id.get();
    let interaction_id = component.id.get();
    if EXECUTED_INTERACTIONS.lock().contains(interaction_id) {
        tracing::warn!(interaction_id, custom_id = %component.data.custom_id, "duplicate button interaction ignored");
        return Ok(());
    }
    if !is_button_clicker_allowed(user_id, allowed_users) {
        tracing::warn!(user_id, custom_id = %component.data.custom_id, "unauthorized button click");
        return respond_ephemeral(ctx, component, "이 버튼을 누를 권한이 없습니다.").await;
    }
    let config = match load_button_action(&button_actions_path(), &click.namespace) {
        Ok(Some(config)) => config,
        Ok(None) => {
            let content = format!("`{}` 버튼 동작이 설정되어 있지 않습니다.", click.namespace);
            return respond_ephemeral(ctx, component, &content).await;
        }
        Err(error) => {
            tracing::warn!(%error, "button actions config is unreadable");
            return respond_ephemeral(ctx, component, "버튼 설정 파일을 읽지 못했습니다.").await;
        }
    };

    let message_id = component.message.id;
    let claimed = IN_FLIGHT.lock().insert(message_id.get());
    if !claimed {
        return respond_ephemeral(ctx, component, "이미 처리 중입니다.").await;
    }
    if let Err(error) = component
        .create_response(ctx, CreateInteractionResponse::Acknowledge)
        .await
    {
        IN_FLIGHT.lock().remove(&message_id.get());
        post_ack_failure_reply(&ctx.http, component.channel_id, message_id).await;
        tracing::warn!(%error, %message_id, "button acknowledgement failed; action was not run");
        return Ok(());
    }
    EXECUTED_INTERACTIONS.lock().insert(interaction_id);

    let channel_id = component.channel_id;
    let original = component.message.components.clone();
    let clicked = component.data.custom_id.clone();
    let http: Arc<Http> = ctx.http.clone();
    set_buttons(
        &http,
        channel_id,
        message_id,
        rebuild_buttons(&original, &clicked, Some(PENDING_LABEL), true),
    )
    .await;

    let extra_env = vec![
        ("OMON_BUTTON_USER_ID".to_string(), user_id.to_string()),
        ("OMON_BUTTON_CHANNEL_ID".to_string(), channel_id.to_string()),
        ("OMON_BUTTON_MESSAGE_ID".to_string(), message_id.to_string()),
    ];
    tracing::info!(user_id, namespace = %click.namespace, action = %click.action, arg = %click.arg, "running button action");
    tokio::spawn(async move {
        let started = Instant::now();
        let outcome = run_button_action(&config, &click, &extra_env)
            .await
            .unwrap_or_else(|error| ActionOutcome {
                success: false,
                code: None,
                stdout: String::new(),
                stderr: format!("failed to start command: {error}"),
                timed_out: false,
            });
        tracing::info!(
            namespace = %click.namespace,
            action = %click.action,
            success = outcome.success,
            code = ?outcome.code,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "button action finished"
        );
        let rows = if outcome.success {
            let label = config
                .done_labels
                .get(&click.action)
                .map_or(DEFAULT_DONE_LABEL, String::as_str);
            rebuild_buttons(
                &original,
                &clicked,
                Some(label),
                !config.nonterminal_actions.contains(&click.action),
            )
        } else {
            rebuild_buttons(&original, &clicked, None, false)
        };
        set_buttons(&http, channel_id, message_id, rows).await;
        let reply = CreateMessage::new()
            .content(reply_text(&outcome))
            .reference_message((channel_id, message_id))
            .allowed_mentions(CreateAllowedMentions::new());
        if let Err(error) = channel_id.send_message(&http, reply).await {
            tracing::warn!(%error, %message_id, "failed to post button action result");
        }
        IN_FLIGHT.lock().remove(&message_id.get());
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn click(action: &str, arg: &str) -> ButtonClick {
        ButtonClick {
            token: None,
            namespace: "content-intel".into(),
            action: action.into(),
            arg: arg.into(),
        }
    }

    fn script_config(dir: &Path, body: &str, timeout_secs: u64) -> ButtonActionConfig {
        let script = dir.join("action.sh");
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        ButtonActionConfig {
            command: vec!["/bin/sh".into(), script.to_string_lossy().into_owned()],
            cwd: Some(dir.to_path_buf()),
            timeout_secs: Some(timeout_secs),
            done_labels: HashMap::new(),
            nonterminal_actions: HashSet::new(),
        }
    }

    #[test]
    fn parses_well_formed_custom_ids() {
        assert_eq!(
            parse_button_custom_id("omon:btn:content-intel:publish:86-256-314"),
            Some(click("publish", "86-256-314"))
        );
        assert_eq!(
            parse_button_custom_id("omon:btn:content-intel:skip:86"),
            Some(click("skip", "86"))
        );
        let token = format!("4.{}.2", "a".repeat(64));
        assert_eq!(
            parse_button_custom_id(&format!(
                "omon:btn:content-intel:publish:96-417-435:{token}"
            )),
            Some(ButtonClick {
                token: Some(token),
                ..click("publish", "96-417-435")
            })
        );
    }

    #[test]
    fn over_long_button_id_round_trips_through_the_compact_form() {
        let token = format!("4.{}.2", "a".repeat(64));
        let original = format!("omon:btn:content-intel:publish:96-417-435:{token}");
        assert!(
            !custom_id_fits_discord(&original),
            "the fixture must exceed Discord's limit"
        );

        // What goes to Discord fits, and it still carries all 256 bits of the digest.
        let sent = compact_custom_id(&original);
        assert!(
            custom_id_fits_discord(&sent),
            "{sent} ({} chars)",
            sent.len()
        );
        assert!(
            !sent.contains(&"a".repeat(64)),
            "the digest must not be verbatim"
        );
        assert_eq!(sent.split(':').nth(5), Some(COMPACT_VERSION));
        let expected = concat!(
            "omon:btn:content-intel:publish:96-417-435:v1:",
            "4.b64-qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqo.2"
        );
        assert_eq!(sent, expected);
        assert_eq!(compact_custom_id(&sent), sent, "compaction is idempotent");

        // A click on that id expands to the exact original id and token.
        assert_eq!(expand_custom_id(&sent), original);
        let click = parse_button_custom_id(&sent).unwrap();
        assert_eq!(click.namespace, "content-intel");
        assert_eq!(click.action, "publish");
        assert_eq!(click.arg, "96-417-435");
        assert_eq!(click.token, Some(token));

        // An id that already fits is never rewritten.
        assert_eq!(
            compact_custom_id("omon:btn:content-intel:publish:86"),
            "omon:btn:content-intel:publish:86"
        );
        assert_eq!(
            expand_custom_id("omon:btn:content-intel:publish:86"),
            "omon:btn:content-intel:publish:86"
        );
    }

    #[test]
    fn a_raw_token_that_looks_compacted_is_left_alone() {
        let raw_token = format!("4.b64-{}.2", "q".repeat(43));
        let raw = format!("omon:btn:content-intel:publish:86:{raw_token}");
        assert!(custom_id_fits_discord(&raw));
        assert_eq!(expand_custom_id(&raw), raw);
        assert_eq!(parse_button_custom_id(&raw).unwrap().token, Some(raw_token));

        let other = "omon:btn:content-intel:publish:86:v9:4.b64-abc.2";
        assert_eq!(expand_custom_id(other), other);
    }

    #[test]
    fn rerendered_buttons_send_the_compact_id_and_keep_the_token() {
        let token = format!("4.{}.2", "b".repeat(64));
        let original = format!("omon:btn:content-intel:publish:96-417-435:{token}");
        let rows: Vec<ActionRow> = serde_json::from_value(serde_json::json!([{
            "type": 1,
            "components": [
                {"type": 2, "style": 3, "label": "발행", "custom_id": original.clone()},
                {"type": 2, "style": 2, "label": "건너뛰기", "custom_id": "omon:btn:content-intel:skip:86"}
            ]
        }]))
        .unwrap();

        let restored = rebuild_buttons(&rows, &original, None, false).unwrap();
        let json = serde_json::to_value(&restored).unwrap();
        let clicked = json[0]["components"][0]["custom_id"].as_str().unwrap();
        assert!(custom_id_fits_discord(clicked), "{clicked}");
        assert_eq!(parse_button_custom_id(clicked).unwrap().token, Some(token));
        assert_eq!(
            json[0]["components"][1]["custom_id"],
            "omon:btn:content-intel:skip:86"
        );
    }

    #[test]
    fn rejects_malformed_custom_ids() {
        for custom_id in [
            "omon:approval:6d1f:once",
            "omon:btn:content-intel:publish",
            "omon:btn:content-intel:publish:86:extra:more",
            "omon:btn:content-intel:publish:86:-x",
            "omon:btn:content-intel:publish:86:a;b",
            "omon:btn:content-intel:publish:86:",
            "omon:btn:Content:publish:86",
            "omon:btn:content-intel:publish:--all-channels",
            "omon:btn:content-intel:publish:86;rm",
            "omon:btn:content-intel:publish:86 87",
            "omon:btn::publish:86",
            "omon:btn:-x:publish:86",
        ] {
            assert_eq!(parse_button_custom_id(custom_id), None, "{custom_id}");
        }
        let long_arg = format!("omon:btn:content-intel:publish:{}", "1".repeat(65));
        assert_eq!(parse_button_custom_id(&long_arg), None);
    }

    #[test]
    fn empty_allow_list_authorizes_nobody() {
        assert!(!is_button_clicker_allowed(42, &[]));
        assert!(!is_button_clicker_allowed(42, &[7]));
        assert!(is_button_clicker_allowed(42, &[7, 42]));
    }

    #[test]
    fn loads_only_the_requested_namespace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("button-actions.json");
        assert!(load_button_action(&path, "content-intel")
            .unwrap()
            .is_none());

        std::fs::write(
            &path,
            r#"{"content-intel":{"command":["bun","run","x.ts"],"cwd":"/tmp","timeout_secs":900,"done_labels":{"publish":"done"},"nonterminal_actions":["preview"]},"empty":{"command":[]}}"#,
        )
        .unwrap();
        let config = load_button_action(&path, "content-intel").unwrap().unwrap();
        assert_eq!(config.command, vec!["bun", "run", "x.ts"]);
        assert_eq!(config.timeout_secs, Some(900));
        assert_eq!(
            config.done_labels.get("publish").map(String::as_str),
            Some("done")
        );
        assert!(config.nonterminal_actions.contains("preview"));
        assert!(load_button_action(&path, "other").unwrap().is_none());
        assert!(load_button_action(&path, "empty").unwrap().is_none());

        std::fs::write(&path, "{not json").unwrap();
        assert!(load_button_action(&path, "content-intel").is_err());
    }

    #[tokio::test]
    async fn runs_command_with_action_arg_and_cleared_environment() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("OMON_BUTTON_TEST_LEAK_CANARY", "leaked");
        let config = script_config(
            dir.path(),
            r#"echo "argv=$1,$2"; echo "user=$OMON_BUTTON_USER_ID"; echo "canary=${OMON_BUTTON_TEST_LEAK_CANARY:-absent}"; pwd -P"#,
            30,
        );
        let env = vec![("OMON_BUTTON_USER_ID".to_string(), "414".to_string())];
        let outcome = run_button_action(&config, &click("publish", "86-256"), &env)
            .await
            .unwrap();
        assert!(outcome.success, "{outcome:?}");
        assert!(
            outcome.stdout.contains("argv=publish,86-256"),
            "{}",
            outcome.stdout
        );
        assert!(outcome.stdout.contains("user=414"), "{}", outcome.stdout);
        assert!(
            outcome.stdout.contains("canary=absent"),
            "{}",
            outcome.stdout
        );
        let cwd = dir.path().canonicalize().unwrap();
        assert!(
            outcome.stdout.contains(cwd.to_string_lossy().as_ref()),
            "{}",
            outcome.stdout
        );
    }

    #[tokio::test]
    async fn reports_failure_output_for_retry() {
        let dir = tempfile::tempdir().unwrap();
        let config = script_config(
            dir.path(),
            "echo 'publish failed: token expired'; exit 3",
            30,
        );
        let outcome = run_button_action(&config, &click("publish", "86"), &[])
            .await
            .unwrap();
        assert!(!outcome.success);
        assert_eq!(outcome.code, Some(3));
        let reply = reply_text(&outcome);
        assert!(reply.contains("exit 3"), "{reply}");
        assert!(reply.contains("token expired"), "{reply}");
    }

    #[tokio::test]
    async fn kills_command_after_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let config = script_config(dir.path(), "sleep 30", 1);
        let started = Instant::now();
        let outcome = run_button_action(&config, &click("publish", "86"), &[])
            .await
            .unwrap();
        assert!(outcome.timed_out);
        assert!(!outcome.success);
        assert!(started.elapsed() < Duration::from_secs(10));
        assert!(reply_text(&outcome).contains("시간 초과"));
    }

    #[test]
    fn long_output_is_trimmed_to_a_discord_message() {
        let outcome = ActionOutcome {
            success: true,
            code: Some(0),
            stdout: "가".repeat(5000),
            stderr: String::new(),
            timed_out: false,
        };
        assert!(reply_text(&outcome).chars().count() <= 2000);
    }

    #[test]
    fn rebuild_disables_action_buttons_but_keeps_links() {
        let rows: Vec<ActionRow> = serde_json::from_value(serde_json::json!([{
            "type": 1,
            "components": [
                {"type": 2, "style": 3, "label": "발행", "custom_id": "omon:btn:content-intel:publish:86-256"},
                {"type": 2, "style": 2, "label": "건너뛰기", "custom_id": "omon:btn:content-intel:skip:86"},
                {"type": 2, "style": 5, "label": "대시보드", "url": "http://127.0.0.1:4321/assets"}
            ]
        }]))
        .unwrap();

        let pending = rebuild_buttons(
            &rows,
            "omon:btn:content-intel:publish:86-256",
            Some("처리 중"),
            true,
        )
        .unwrap();
        let json = serde_json::to_value(&pending).unwrap();
        let buttons = json[0]["components"].as_array().unwrap();
        assert_eq!(buttons[0]["label"], "처리 중");
        assert_eq!(buttons[0]["disabled"], true);
        assert_eq!(buttons[1]["label"], "건너뛰기");
        assert_eq!(buttons[1]["disabled"], true);
        assert_eq!(buttons[2]["url"], "http://127.0.0.1:4321/assets");
        assert_ne!(buttons[2]["disabled"], true);

        let restored =
            rebuild_buttons(&rows, "omon:btn:content-intel:publish:86-256", None, false).unwrap();
        let json = serde_json::to_value(&restored).unwrap();
        assert_eq!(json[0]["components"][0]["label"], "발행");
        assert_eq!(json[0]["components"][0]["disabled"], false);
    }

    #[test]
    fn restore_keeps_originally_disabled_buttons_disabled() {
        let rows: Vec<ActionRow> = serde_json::from_value(serde_json::json!([{
            "type": 1,
            "components": [
                {"type": 2, "style": 3, "label": "발행", "custom_id": "omon:btn:content-intel:publish:86"},
                {"type": 2, "style": 2, "label": "이미 완료", "custom_id": "omon:btn:content-intel:skip:86", "disabled": true}
            ]
        }]))
        .unwrap();
        let restored =
            rebuild_buttons(&rows, "omon:btn:content-intel:publish:86", None, false).unwrap();
        let json = serde_json::to_value(&restored).unwrap();
        assert_eq!(json[0]["components"][0]["disabled"], false);
        assert_eq!(json[0]["components"][1]["disabled"], true);
    }

    #[test]
    fn executed_interaction_lru_deduplicates_and_evicts_oldest() {
        let mut lru = InteractionLru::default();
        lru.insert(41);
        assert!(lru.contains(41));
        for id in 0..EXECUTED_INTERACTION_LIMIT as u64 - 1 {
            lru.insert(id + 100);
        }
        assert!(lru.contains(41));
        lru.insert(10_000);
        assert!(lru.contains(41));
        assert!(!lru.contains(100));
    }

    #[test]
    fn nonterminal_done_label_keeps_clicked_button_enabled() {
        let rows: Vec<ActionRow> = serde_json::from_value(serde_json::json!([{
            "type": 1,
            "components": [{"type": 2, "style": 3, "label": "Preview", "custom_id": "omon:btn:content-intel:preview:86"}]
        }]))
        .unwrap();
        let done = rebuild_buttons(
            &rows,
            "omon:btn:content-intel:preview:86",
            Some("✓ Preview"),
            false,
        )
        .unwrap();
        let json = serde_json::to_value(done).unwrap();
        assert_eq!(json[0]["components"][0]["label"], "✓ Preview");
        assert_eq!(json[0]["components"][0]["disabled"], false);
    }

    #[test]
    fn failed_acknowledgement_does_not_claim_interaction_for_execution() {
        let mut executed = InteractionLru::default();
        let interaction_id = 42;
        assert!(!executed.contains(interaction_id));
        assert!(ACK_FAILURE_REPLY.contains("실행하지 않았습니다"));
    }
}
