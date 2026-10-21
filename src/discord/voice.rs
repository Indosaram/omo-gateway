//! `/voice join` and `/voice leave`.

use poise::serenity_prelude as serenity;

use super::commands::{CommandError, PoiseContext};

#[poise::command(slash_command, subcommands("join", "leave"), guild_only)]
/// Join or leave a Discord voice channel.
pub async fn voice(_: PoiseContext<'_>) -> Result<(), CommandError> {
    Ok(())
}

#[poise::command(slash_command, guild_only)]
/// Join your voice channel, listen, and speak the agent reply.
async fn join(
    ctx: PoiseContext<'_>,
    #[description = "Voice channel to join. Defaults to the channel you are in."]
    #[channel_types("Voice", "Stage")]
    channel: Option<serenity::GuildChannel>,
) -> Result<(), CommandError> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| command_message("Voice channels are only available in a server."))?;
    let voice_channel = match channel {
        Some(channel) => channel.id,
        None => author_voice_channel(ctx.serenity_context(), guild_id, ctx.author().id)
            .ok_or_else(|| {
                command_message("Join a voice channel first, or pass one to `/voice join`.")
            })?,
    };
    let session = super::commands::session_key(ctx).await?;
    let bot_user_id = ctx.serenity_context().cache.current_user().id.get();
    let message = ctx
        .data()
        .voice
        .join(
            ctx.serenity_context(),
            guild_id.get(),
            voice_channel.get(),
            session,
            bot_user_id,
        )
        .await
        .map_err(command_message)?;
    reply(ctx, message).await
}

#[poise::command(slash_command, guild_only)]
/// Leave the voice channel in this server.
async fn leave(ctx: PoiseContext<'_>) -> Result<(), CommandError> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| command_message("Voice channels are only available in a server."))?;
    let message = ctx
        .data()
        .voice
        .leave(ctx.serenity_context(), guild_id.get())
        .await
        .map_err(command_message)?;
    reply(ctx, message).await
}

fn author_voice_channel(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
) -> Option<serenity::ChannelId> {
    ctx.cache.guild(guild_id).and_then(|guild| {
        guild
            .voice_states
            .get(&user_id)
            .and_then(|state| state.channel_id)
    })
}

async fn reply(ctx: PoiseContext<'_>, message: String) -> Result<(), CommandError> {
    ctx.send(
        poise::CreateReply::default()
            .content(message)
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

fn command_message(message: impl Into<String>) -> CommandError {
    Box::new(VoiceCommandError(message.into()))
}

#[derive(Debug)]
struct VoiceCommandError(String);

impl std::fmt::Display for VoiceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for VoiceCommandError {}
