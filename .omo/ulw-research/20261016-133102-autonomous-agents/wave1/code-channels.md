<!-- task=st_01a14511 -->
st_01a14511 [completed] model mahoquot/claude-opus-5-5
Ask: map the channels area of omon-gateway (src/discord, src/voice, src/voice_transcription.rs, src/mirror.rs) - wanted: what exists and what is missing, with file:line evidence. For you: the report is below. Discord is the only two-way channel with full outbound support. The dashboard's `web` platform only receives copies of outbound output. iMessage and Kakao are read-only inbox sources. There is no phone or SMS support, and the realtime voice pipeline is defined but never connected. Now: none. Next: none.

## Capabilities

**Outbound routing**
- Everything outbound goes through the `OutboundDispatcher` trait (`src/multiplexer/actor.rs:85-100`). It defines `dispatch`, `owner_thread_is_private` and `dispatch_direct_reply`.
- The actions are a closed `OutboundAction` enum (`src/models/events.rs:346`): SendMessage, EditMessage, DeleteMessage, UploadFile, Stream, Typing, React, ApprovalRequest and ExpireApproval. `StreamChunk` is defined at `src/models/events.rs:336`.
- The real dispatcher is `DiscordEgress` (`src/discord/adapter.rs:2651`). Its `impl OutboundDispatcher` (`:3514`) handles every action at `:3565`, `:3779`, `:3847`, `:3887`, `:3938`, `:3941`, `:3961`, `:4046` and `:4111`.
- It is wrapped by `CompositeDispatcher` (`src/dashboard.rs:537-565`), which copies each action to the web dashboard and sends the session to Discord unless `platform == "web"`. This is wired up in `src/main.rs:334-338`, and a late-bound `SharedDispatcher` sits at `src/main.rs:294-323`.

**Multiple bots**
- `DiscordEgress::with_bot_clients` (`src/discord/adapter.rs:3076`, built in `src/main.rs:1171`) creates the egress.
- `identity`/`http_for` (`:3346-3357`) pick the bot from `SessionKey.bot_id`.
- `target` (`:3336`) uses the thread ID, or the channel ID if there is no thread.

**Streaming**
- `stream()` (`src/discord/adapter.rs:3388`) posts a zero-width placeholder message (`:3466`), then edits it through `LiveEditThrottler` (`src/discord/throttler.rs:133-190`), which debounces and serializes edits.
- In practice, `completed_stream_content` (`src/discord/adapter.rs:3509`) returns only the final content. Partial chunks just refresh the typing indicator (`:3389-3391`), so users never see token-by-token text. A test pins this behavior: `discord_stream_delivers_only_completed_content` (`:4723`).
- On the final chunk it delivers media directives (`:3187`), renders Markdown tables as images (`:3125`, `src/discord/table_render/mod.rs`), and filters out reasoning text and explicit silence responses (`:3392`, `:3402`).

**Typing indicator**
- The Typing action keeps a guard from `start_typing` for each session (`src/discord/adapter.rs:3941-3959`).
- `keep_typing` refreshes it at most every 5 s per channel (`TYPING_REFRESH_INTERVAL`, `:2670`, `:3359`).

**Reliability**
- `DeadTargetRegistry` (`src/discord/adapter.rs:2697-2990`) short-circuits sends to channels that returned 403/404, with a TTL and probing. It is persisted to the `dead_targets` table (`src/storage/db.rs:921`).
- Other pieces:
  - Receive watchdog with a 1200 s default (`src/discord/adapter.rs:2541-2648`).
  - Replay of failed transport obligations (`:3247`).
  - Missed-message backfill using per-channel and per-bot cursors (`:4443-4690`, `:4236-4376`).
  - Dedup through `DeliveryLedgerService` (`:1664-1675`).

**Inbound handling**
- Inbound filtering (`InboundFilterConfig`, `:459`) covers allowed users, roles and channels, free-response channels, thread mentions and an allow-bots mode.
- Other inbound features:
  - Split messages are coalesced with a 600 ms debounce (`:543`, `:552`, `:639`).
  - Steering into an active turn (`:1594`).
  - Auto-threading and forum post titles (`:259`, `:341`, `:501`).
  - Reply and forward context (`:398`, `:513`).
  - Thread create, update and delete lifecycle (`:1483-1570`).

**Owner access, commands, buttons, approvals**
- Pairing codes with lockout: `PairingStore` (`src/discord/pairing.rs:38-237`).
- Handling of DMs from unpaired users (`src/discord/adapter.rs:1794-1852`).
- Lane owner-reply interception (`:1852`).
- Poise slash commands (`src/discord/commands.rs`), plus session commands: overview, read, send, stop, answer, watch and unwatch (`src/discord/session_commands.rs:72-411`).
- Button actions loaded from `~/.omon/button-actions.json` (`src/discord/buttons.rs:123-349`).
- Approval embeds with buttons (`src/discord/adapter.rs:4046`, `:4138-4213`).
- `SmartApprovalGuard` and `DiscordApprovalRequester`, with heartbeat, timeout and a persisted allowlist (`src/discord/approval.rs:129-445`, `:521`).

**Files and voice notes out**
- `DiscordUploadTransport` (`src/discord/adapter.rs:2300-2435`) sends voice notes using flag 8192 with a synthesized waveform (`:2209-2256`), plus ordinary, forum and batch attachments.
- Uploads go through a media-path allowlist and a deny-list for secret files (`:35-131`).

**Voice in, async (works)**
- Attachments are downloaded through `AttachmentDownloader::hydrate` (`src/discord/attachments.rs:255-345`).
- Voice notes are transcribed by OpenAI Whisper using `OpenAiSpeechToText` (`src/voice/pipeline.rs:154-264`, wired in `src/main.rs:1339`).
- There is a second path that runs local `ffmpeg` and `whisper-cli` (`src/voice_transcription.rs:68`, with prerequisite checks at `:22`), called from `src/discord/adapter.rs:1680-1711`. It adds a Korean "음성 전사" (voice transcript) prefix to the message. Both paths can run on the same audio attachment.

**Voice, realtime (scaffold only)**
- `SongbirdAudioEventListener` (`src/voice/mod.rs:16-85`) turns VoiceTick PCM and RTP Opus into `AudioFrame`s.
- `SpeechPipeline` (STT, then LLM, then TTS) is at `src/voice/pipeline.rs:276-308`, with a bounded mpsc channel `VoiceAudioPipeline` at `:311`.
- `songbird 0.5` with the receive feature is a dependency (`Cargo.toml:58`).
- A repo-wide grep found nothing that constructs the listener or the pipeline, and no `impl TextToSpeech` or `impl VoiceLanguageModel` anywhere.

**Mirror and cross-channel memory**
- `mirror_to_session` (`src/mirror.rs:10-63`) writes out-of-band deliveries into `messages` with `metadata_json.mirror=true` and a `mirror_source`.
- `find_session_by_origin` (`:77-185`) resolves a session from platform, chat, thread, user and bot. It is platform-generic and declines ambiguous matches.
- Cron briefings (`:204-315`) are injected into the next turn: capped at 6000 characters, newest first, consumed only after the turn is accepted. Inbox-digest tokens are never injected (`:225-228`).

**Other channels**
- iMessage is read-only through the `imsg chats/history --json` CLI (`src/inbox/sources/imessage.rs:13-32`).
- Kakao is read-only, fetched over HTTP from a PirateTalk archive (`src/inbox/sources/kakao.rs:12-40`, `:122-231`).
- Mail is read-only (`src/inbox/sources/mail.rs`).
- None of these send anything.

## Data model

| Table / struct | Where |
|---|---|
| `sessions` (session_key, platform, channel_id, thread_id, user_id, state_json, updated_at) | `src/mirror.rs:23,51,83`; thread metadata at `src/discord/adapter.rs:1532,1560` |
| `messages` (id, session_key, role, content, metadata_json, platform_message_id, sequence, created_at) | `src/mirror.rs:41`, `src/discord/adapter.rs:1770` |
| `discord_channel_cursors`, `discord_bot_cursors` | `src/discord/adapter.rs:4247-4367,4465` |
| `dead_targets` | `src/storage/db.rs:921` |
| `paired_users`, `pairing_codes`, `pairing_platform_lockout` | `src/discord/pairing.rs:53,135,179,309,507` |
| `approval_allowlist` (+ yolo) | `src/discord/approval.rs:434,445,521` |
| Delivery ledger | `DeliveryLedgerService` (`src/discord/adapter.rs:1664`) |
| Structs | `SessionKey` (platform is a free-form `String`, `src/models/session.rs:31`), `OutboundAction`, `StreamChunk`, `AudioFrame`/`AudioPayload`, `VoiceMetadata`, `DeadTargetEntry`, `InboundFilterConfig` |

## Limits and gaps for an always-on, multi-channel personal agent

1. **No channel abstraction.** There is no `enum Platform`. Platform is a string, and the code branches on `"discord"` and `"web"` (`src/main.rs:408`, `src/dashboard.rs:550,558`). `CompositeDispatcher` sends every session that is not `web` to Discord. Hook: change `CompositeDispatcher::dispatch` (`src/dashboard.rs:557`) into a dispatcher that routes by `session.platform`, and register one egress per platform beside `DiscordEgress::with_bot_clients` (`src/main.rs:1171`).
2. **iMessage send is missing.** Only reading exists (`src/inbox/sources/imessage.rs:13-32`). Hook: a new `IMessageEgress: OutboundDispatcher`, for example using `imsg send` or AppleScript, routed by platform `"imessage"`. Inbound needs a live watcher that turns messages into `InboundEvent` for `multiplexer.route`, the same way `src/discord/adapter.rs:1714` does, rather than only digest items.
3. **Kakao send is missing.** It is fetch-only (`src/inbox/sources/kakao.rs:122-231`). Hook: a `KakaoEgress` that posts to the PirateTalk base URL (`:22`), if that service supports sending. That is not verified.
4. **No phone, SMS, Telegram or Slack.** A grep found no Twilio, Telegram, Slack or SMS code. The `songbird` voice scaffold applies only to Discord voice channels.
5. **Realtime voice is not connected.** Nothing joins a voice channel, and there is no TTS or voice-LLM implementation. Hook: implement `TextToSpeech` and `VoiceLanguageModel` (`src/voice/pipeline.rs:258-266`) with the LLM backed by the multiplexer session. Register songbird on the serenity client (around `src/main.rs:1171`) and attach `SongbirdAudioEventListener::new` (`src/voice/mod.rs:23`). Two more problems:
   - Opus is sent to Whisper one RTP packet at a time with no Ogg container, and multiple Opus frames are rejected (`src/voice/pipeline.rs:189-193`). Realtime needs voice activity detection and buffering through `AudioFrameBuffer` (`:66`).
   - Transcription runs twice for one attachment (`src/discord/attachments.rs:259` and `src/discord/adapter.rs:1689`). This should be merged into one path.
6. **Streaming shows only final text.** See `src/discord/adapter.rs:3509`. For live progress like a hosted agent, emit partial content through the throttler that already exists (`:3389`).
7. **Proactive delivery is Discord-shaped.** Cron output reaches the agent only through `mirror_to_session` with `cron:` labels (`src/mirror.rs:204`). Other proactive sources such as inbox, heartbeat or connectors need their own `mirror_source` labels and briefing policy (`src/mirror.rs:212-229`).
8. **Approvals are Discord-only.** Buttons and embeds are built in `src/discord/adapter.rs:4046-4213`. A non-Discord channel would need its own `ApprovalRequest` rendering, for example a text reply token. The extension point is `DiscordApprovalRequester::set_dispatcher` (`src/discord/approval.rs:229`).
9. **Identity and pairing are Discord-only.** `PairingStore` uses `u64` Discord user IDs (`src/discord/pairing.rs:70-90`). Recognizing the same owner across channels needs an identity table that maps owner to platform handles. A natural place is next to `find_session_by_origin` (`src/mirror.rs:77`).
10. **Dedup, backfill and the watchdog are tied to Discord.** Cursors (`src/discord/adapter.rs:4236`) and the watchdog (`:2576`) would need generic per-channel versions for always-on use across several channels.

## EXPAND

- `src/inbox/watchers.rs` and `src/inbox/prefilter.rs` mention iMessage and Kakao many times. Check whether they already push live events (not just digests), which would be the inbound side of item 2.
- `src/lanes/discord_threads.rs` and `src/sessions/status_reply.rs` handle owner-thread privacy and status replies, which matter for multi-channel owner routing.
- `src/dashboard.rs:475` (`WebDashboardDispatcher`) is the only other first-class platform (`"web"`) and is a useful template for a new egress.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/discord/adapter.rs - DiscordEgress (all outbound actions), streaming, typing, dead targets, inbound filter, backfill, voice upload, transcription call
- /Users/indo/code/project/omon-gateway/src/discord/throttler.rs - LiveEditThrottler and the message transport trait
- /Users/indo/code/project/omon-gateway/src/discord/approval.rs - approval guard and requester, persisted allowlist
- /Users/indo/code/project/omon-gateway/src/discord/attachments.rs - attachment hydration and OpenAI STT for voice notes
- /Users/indo/code/project/omon-gateway/src/discord/pairing.rs - owner pairing (Discord u64 IDs)
- /Users/indo/code/project/omon-gateway/src/discord/session_commands.rs - owner slash commands for sessions
- /Users/indo/code/project/omon-gateway/src/voice/mod.rs - songbird receive listener (unwired)
- /Users/indo/code/project/omon-gateway/src/voice/pipeline.rs - AudioFrame, STT/LLM/TTS pipeline traits, OpenAI Whisper
- /Users/indo/code/project/omon-gateway/src/voice_transcription.rs - local ffmpeg + whisper-cli transcription
- /Users/indo/code/project/omon-gateway/src/mirror.rs - cross-channel transcript mirror, origin lookup, cron briefings
- /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs - OutboundDispatcher trait
- /Users/indo/code/project/omon-gateway/src/models/events.rs - OutboundAction and StreamChunk
- /Users/indo/code/project/omon-gateway/src/dashboard.rs - CompositeDispatcher (web vs everything-else-to-Discord)
- /Users/indo/code/project/omon-gateway/src/main.rs - dispatcher wiring (294-338, 1171, 1339)
- /Users/indo/code/project/omon-gateway/src/inbox/sources/imessage.rs - read-only iMessage
- /Users/indo/code/project/omon-gateway/src/inbox/sources/kakao.rs - read-only Kakao
</files>
<answer>The only full channel is Discord, which supports multiple bots, threads and forums, edits, reactions, uploads, voice-note sending, approvals, pairing and backfill. The web dashboard only receives copies of outbound output. iMessage, Kakao and mail are read-only inbox sources with no send path. There is no phone, SMS, Telegram or Slack support. Realtime voice exists as a scaffold but is not connected, with no TTS or voice-LLM implementation. Voice-note transcription works, but it runs twice for the same attachment. Streaming shows only the final text. Adding a channel means writing an OutboundDispatcher and replacing CompositeDispatcher's "everything else goes to Discord" rule with routing by platform.</answer>
<next_steps>Design a dispatcher that routes by platform at src/dashboard.rs:557 and src/main.rs:1171. Then add IMessageEgress and KakaoEgress, and a cross-platform owner identity table. Check src/inbox/watchers.rs for any existing live inbound path.</next_steps>
</results>