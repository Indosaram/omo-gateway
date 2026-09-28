//! A button with custom id `omon:btn:<namespace>:<action>:<arg>` runs the command configured for
//! `<namespace>` in the button-actions file with `<action> <arg>` appended. Exit 0 marks the
//! button done; any other outcome restores it for a retry.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::Deserialize;
use serenity::all::{
    ActionRow, ActionRowComponent, ButtonKind, ChannelId, ComponentInteraction, Context,
    CreateActionRow, CreateAllowedMentions, CreateButton, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditMessage, Http, MessageId,
};

pub const BUTTON_PREFIX: &str = "omon:btn:";
const DEFAULT_TIMEOUT_SECS: u64 = 600;
const MAX_REPLY_CHARS: usize = 1900;
const PENDING_LABEL: &str = "⏳ 처리 중…";
const DEFAULT_DONE_LABEL: &str = "✓ 완료";
const INHERITED_ENV: [&str; 6] = ["HOME", "USER", "LOGNAME", "TMPDIR", "LANG", "LC_ALL"];

static IN_FLIGHT: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ButtonClick {
    pub namespace: String,
    pub action: String,
    pub arg: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ButtonActionConfig {
    pub command: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub done_labels: HashMap<String, String>,
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

/// The arg becomes an argv element of the configured command, so it may never start with `-`.
pub fn parse_button_custom_id(custom_id: &str) -> Option<ButtonClick> {
    let mut parts = custom_id.strip_prefix(BUTTON_PREFIX)?.split(':');
    let (namespace, action, arg) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || !is_name(namespace) || !is_name(action) || !is_arg(arg) {
        return None;
    }
    Some(ButtonClick {
        namespace: namespace.to_string(),
        action: action.to_string(),
        arg: arg.to_string(),
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
pub fn rebuild_buttons(
    rows: &[ActionRow],
    clicked: &str,
    clicked_label: Option<&str>,
    disabled: bool,
) -> Option<Vec<CreateActionRow>> {
    rows.iter()
        .map(|row| {
            row.components
                .iter()
                .map(|component| match component {
                    ActionRowComponent::Button(button) => Some(match &button.data {
                        ButtonKind::NonLink { custom_id, style } if custom_id == clicked => {
                            match clicked_label {
                                Some(label) => CreateButton::new(custom_id.clone())
                                    .style(*style)
                                    .label(label)
                                    .disabled(disabled),
                                None => CreateButton::from(button.clone()).disabled(disabled),
                            }
                        }
                        ButtonKind::Link { .. } => CreateButton::from(button.clone()),
                        _ => CreateButton::from(button.clone()).disabled(disabled),
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
    if let Err(error) = channel_id
        .edit_message(http, message_id, EditMessage::new().components(rows))
        .await
    {
        tracing::warn!(%error, %message_id, "failed to update button state");
    }
}

pub async fn handle_button_click(
    ctx: &Context,
    component: &ComponentInteraction,
    allowed_users: &[u64],
    click: ButtonClick,
) -> serenity::Result<()> {
    let user_id = component.user.id.get();
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
        return Err(error);
    }

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
            rebuild_buttons(&original, &clicked, Some(label), true)
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
    }

    #[test]
    fn rejects_malformed_custom_ids() {
        for custom_id in [
            "omon:approval:6d1f:once",
            "omon:btn:content-intel:publish",
            "omon:btn:content-intel:publish:86:extra",
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
            r#"{"content-intel":{"command":["bun","run","x.ts"],"cwd":"/tmp","timeout_secs":900,"done_labels":{"publish":"done"}},"empty":{"command":[]}}"#,
        )
        .unwrap();
        let config = load_button_action(&path, "content-intel").unwrap().unwrap();
        assert_eq!(config.command, vec!["bun", "run", "x.ts"]);
        assert_eq!(config.timeout_secs, Some(900));
        assert_eq!(
            config.done_labels.get("publish").map(String::as_str),
            Some("done")
        );
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
}
