use chrono::Utc;
use poise::CreateReply;
use serde_json::Value;

use super::commands::{chunk_slash_reply, CommandError, PoiseContext};
use crate::sessions::control::{SendMode, SessionControl, Target};
use crate::sessions::overview::{self, format_detail, format_overview, DISCORD_LIMIT};
use crate::sessions::watch;

pub fn owner_allowed(author_id: u64) -> bool {
    crate::sessions::status_reply::owner_id() == author_id.to_string()
}

pub fn reply_ephemeral(guild: bool) -> bool {
    guild
}

async fn check_owner_or_reject(ctx: PoiseContext<'_>) -> Result<bool, CommandError> {
    if !owner_allowed(ctx.author().id.get()) {
        ctx.send(
            CreateReply::default()
                .content("이 명령은 소유자만 사용할 수 있어요.")
                .ephemeral(true),
        )
        .await?;
        return Ok(false);
    }
    Ok(true)
}

async fn send_reply_chunked(
    ctx: PoiseContext<'_>,
    ephemeral: bool,
    text: &str,
) -> Result<(), CommandError> {
    let chunks = chunk_slash_reply(text, DISCORD_LIMIT);
    if chunks.is_empty() {
        ctx.send(
            CreateReply::default()
                .content("(내용 없음)")
                .ephemeral(ephemeral),
        )
        .await?;
        return Ok(());
    }

    for chunk in chunks {
        ctx.send(CreateReply::default().content(chunk).ephemeral(ephemeral))
            .await?;
    }
    Ok(())
}

async fn send_error(
    ctx: PoiseContext<'_>,
    ephemeral: bool,
    error: impl std::fmt::Display,
) -> Result<(), CommandError> {
    ctx.send(
        CreateReply::default()
            .content(format!("⚠️ {error}"))
            .ephemeral(ephemeral),
    )
    .await?;
    Ok(())
}

#[poise::command(
    slash_command,
    subcommands("overview", "read", "send", "stop", "answer", "watch", "unwatch")
)]
pub async fn sessions(_ctx: PoiseContext<'_>) -> Result<(), CommandError> {
    Ok(())
}

#[poise::command(slash_command)]
pub async fn overview(ctx: PoiseContext<'_>) -> Result<(), CommandError> {
    if !check_owner_or_reject(ctx).await? {
        return Ok(());
    }
    let ephemeral = reply_ephemeral(ctx.guild_id().is_some());
    if ephemeral {
        ctx.defer_ephemeral().await?;
    } else {
        ctx.defer().await?;
    }

    let control = match SessionControl::from_environment() {
        Ok(c) => c,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let overview_data = overview::collect(&control, 100).await;
    let formatted = format_overview(&overview_data, Utc::now(), DISCORD_LIMIT);
    send_reply_chunked(ctx, ephemeral, &formatted).await
}

#[poise::command(slash_command)]
pub async fn read(
    ctx: PoiseContext<'_>,
    #[description = "Target session (<machine>:<session_id or prefix>)"] target: String,
    #[description = "Max lines or items to fetch (default: 20)"] limit: Option<usize>,
) -> Result<(), CommandError> {
    if !check_owner_or_reject(ctx).await? {
        return Ok(());
    }
    let ephemeral = reply_ephemeral(ctx.guild_id().is_some());
    if ephemeral {
        ctx.defer_ephemeral().await?;
    } else {
        ctx.defer().await?;
    }

    let parsed_target: Target = match target.parse() {
        Ok(t) => t,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let control = match SessionControl::from_environment() {
        Ok(c) => c,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let (resolved_target, session) = match control.resolve(&parsed_target).await {
        Ok(res) => res,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let limit_val = limit.unwrap_or(20);
    let transcript_value = match control.read(&resolved_target, limit_val, None).await {
        Ok(v) => v,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let mut transcript_tail = Vec::new();
    if let Some(items) = transcript_value.get("items").and_then(Value::as_array) {
        for item in items {
            let role = item
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let content = item.get("content").and_then(Value::as_str).unwrap_or("");
            transcript_tail.push(format!("{role}: {content}"));
        }
    }

    let probe = crate::sessions::probe::Probe::new(control.runner());
    let snapshot = probe.snapshot(resolved_target.machine(), 100).await.ok();
    let live_session = snapshot
        .as_ref()
        .and_then(|snap| {
            snap.sessions
                .iter()
                .find(|s| s.session_id == session.session_id || s.routing_id == session.session_id)
        })
        .cloned()
        .unwrap_or_else(|| crate::sessions::probe::LiveSession {
            machine: resolved_target.machine().to_string(),
            session_id: session.session_id.clone(),
            routing_id: session.session_id.clone(),
            endpoint_kind: "unknown".to_string(),
            name: session.name.clone(),
            cwd: None,
            streaming: false,
            compacting: false,
            blocking_question: false,
            pending_messages: 0,
            message_count: 0,
            last_user: String::new(),
            last_assistant: String::new(),
            questions: Vec::new(),
            messages_error: None,
            updated_at: None,
        });

    let detail = format_detail(
        &resolved_target.to_string(),
        &live_session,
        &transcript_tail,
        DISCORD_LIMIT,
    );
    send_reply_chunked(ctx, ephemeral, &detail).await
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum SlashSendMode {
    #[name = "auto"]
    Auto,
    #[name = "steer"]
    Steer,
    #[name = "follow_up"]
    FollowUp,
}

impl From<SlashSendMode> for SendMode {
    fn from(mode: SlashSendMode) -> Self {
        match mode {
            SlashSendMode::Auto => SendMode::Auto,
            SlashSendMode::Steer => SendMode::Steer,
            SlashSendMode::FollowUp => SendMode::FollowUp,
        }
    }
}

#[poise::command(slash_command)]
pub async fn send(
    ctx: PoiseContext<'_>,
    #[description = "Target session (<machine>:<session_id or prefix>)"] target: String,
    #[description = "Instruction text to send"] text: String,
    #[description = "Send mode (auto, steer, follow_up)"] mode: Option<SlashSendMode>,
) -> Result<(), CommandError> {
    if !check_owner_or_reject(ctx).await? {
        return Ok(());
    }
    let ephemeral = reply_ephemeral(ctx.guild_id().is_some());
    if ephemeral {
        ctx.defer_ephemeral().await?;
    } else {
        ctx.defer().await?;
    }

    let parsed_target: Target = match target.parse() {
        Ok(t) => t,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let control = match SessionControl::from_environment() {
        Ok(c) => c,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let (resolved_target, _) = match control.resolve(&parsed_target).await {
        Ok(res) => res,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let send_mode = mode.map(SendMode::from).unwrap_or(SendMode::Auto);
    let key = uuid::Uuid::new_v4();

    if let Err(e) = control.send(&resolved_target, &text, send_mode, key).await {
        return send_error(ctx, ephemeral, e).await;
    }

    let pool = &ctx.data().pool;
    let target_str = resolved_target.to_string();
    let watched = match watch::watch(pool, &target_str).await {
        Ok(()) => "감시 목록에 등록했어요".to_owned(),
        Err(error) => {
            tracing::warn!(%error, target = %target_str, "auto-watch after send failed");
            format!("감시 등록은 실패했어요: {error}")
        }
    };

    ctx.send(
        CreateReply::default()
            .content(format!("✅ `{target_str}`로 지시를 전송했어요. {watched}."))
            .ephemeral(ephemeral),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn stop(
    ctx: PoiseContext<'_>,
    #[description = "Target session (<machine>:<session_id or prefix>)"] target: String,
) -> Result<(), CommandError> {
    if !check_owner_or_reject(ctx).await? {
        return Ok(());
    }
    let ephemeral = reply_ephemeral(ctx.guild_id().is_some());
    if ephemeral {
        ctx.defer_ephemeral().await?;
    } else {
        ctx.defer().await?;
    }

    let parsed_target: Target = match target.parse() {
        Ok(t) => t,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let control = match SessionControl::from_environment() {
        Ok(c) => c,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let (resolved_target, _) = match control.resolve(&parsed_target).await {
        Ok(res) => res,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    if let Err(e) = control.interrupt(&resolved_target).await {
        return send_error(ctx, ephemeral, e).await;
    }

    ctx.send(
        CreateReply::default()
            .content(format!(
                "🛑 `{}` 실행을 중단(인터럽트)했어요.",
                resolved_target
            ))
            .ephemeral(ephemeral),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn answer(
    ctx: PoiseContext<'_>,
    #[description = "Target session (<machine>:<session_id or prefix>)"] target: String,
    #[description = "Answer text or selected option label"] text: String,
    #[description = "Request ID of the question (optional if only 1 pending)"] request_id: Option<
        String,
    >,
) -> Result<(), CommandError> {
    if !check_owner_or_reject(ctx).await? {
        return Ok(());
    }
    let ephemeral = reply_ephemeral(ctx.guild_id().is_some());
    if ephemeral {
        ctx.defer_ephemeral().await?;
    } else {
        ctx.defer().await?;
    }

    let parsed_target: Target = match target.parse() {
        Ok(t) => t,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let control = match SessionControl::from_environment() {
        Ok(c) => c,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let (resolved_target, _) = match control.resolve(&parsed_target).await {
        Ok(res) => res,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    if let Err(e) = control
        .answer(&resolved_target, &text, request_id.as_deref())
        .await
    {
        return send_error(ctx, ephemeral, e).await;
    }

    ctx.send(
        CreateReply::default()
            .content(format!(
                "💬 `{}` 질문에 답변을 전송했어요.",
                resolved_target
            ))
            .ephemeral(ephemeral),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn watch(
    ctx: PoiseContext<'_>,
    #[description = "Target session (<machine>:<session_id or prefix>)"] target: String,
) -> Result<(), CommandError> {
    if !check_owner_or_reject(ctx).await? {
        return Ok(());
    }
    let ephemeral = reply_ephemeral(ctx.guild_id().is_some());
    if ephemeral {
        ctx.defer_ephemeral().await?;
    } else {
        ctx.defer().await?;
    }

    let parsed_target: Target = match target.parse() {
        Ok(t) => t,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let control = match SessionControl::from_environment() {
        Ok(c) => c,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let (resolved_target, _) = match control.resolve(&parsed_target).await {
        Ok(res) => res,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let pool = &ctx.data().pool;
    let target_str = resolved_target.to_string();
    if let Err(e) = watch::watch(pool, &target_str).await {
        return send_error(ctx, ephemeral, e).await;
    }

    ctx.send(
        CreateReply::default()
            .content(format!(
                "👀 `{}` 세션을 감시 목록에 등록했어요.",
                target_str
            ))
            .ephemeral(ephemeral),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn unwatch(
    ctx: PoiseContext<'_>,
    #[description = "Target session (<machine>:<session_id or prefix>)"] target: String,
) -> Result<(), CommandError> {
    if !check_owner_or_reject(ctx).await? {
        return Ok(());
    }
    let ephemeral = reply_ephemeral(ctx.guild_id().is_some());
    if ephemeral {
        ctx.defer_ephemeral().await?;
    } else {
        ctx.defer().await?;
    }

    let parsed_target: Target = match target.parse() {
        Ok(t) => t,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let control = match SessionControl::from_environment() {
        Ok(c) => c,
        Err(e) => return send_error(ctx, ephemeral, e).await,
    };

    let target_str = match control.resolve(&parsed_target).await {
        Ok((resolved, _)) => resolved.to_string(),
        Err(_) => parsed_target.to_string(),
    };

    let pool = &ctx.data().pool;
    match watch::unwatch(pool, &target_str).await {
        Ok(true) => {
            ctx.send(
                CreateReply::default()
                    .content(format!("제외: `{}` 세션 감시를 해제했어요.", target_str))
                    .ephemeral(ephemeral),
            )
            .await?;
        }
        Ok(false) => {
            ctx.send(
                CreateReply::default()
                    .content(format!("`{}` 세션은 감시 목록에 없었어요.", target_str))
                    .ephemeral(ephemeral),
            )
            .await?;
        }
        Err(e) => return send_error(ctx, ephemeral, e).await,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owner_allowed() {
        let owner: u64 = crate::sessions::status_reply::owner_id()
            .parse()
            .expect("owner id is numeric");
        assert!(owner_allowed(owner));
        assert!(!owner_allowed(owner.wrapping_add(1)));
        assert!(!owner_allowed(0));
    }

    #[test]
    fn test_reply_ephemeral() {
        assert!(reply_ephemeral(true));
        assert!(!reply_ephemeral(false));
    }
}
