# runtime-aux review

### [P1] Cron/out-of-band mirror falls back to any thread session in the channel
- Location: src/mirror.rs:147
- Trigger: `crate::cron::scheduler::mirror_cron_delivery_to_session` (src/cron/scheduler.rs:2075) calls `find_session_by_origin` for a channel destination with `thread_id = None`. Discord thread sessions are stored with `channel_id` set to the parent channel and `thread_id` set (src/discord/adapter.rs:1960-1985). In a guild channel where all conversation happens in threads (`auto_thread`), no row matches `thread_id IS NULL OR ''`. The code then re-queries `WHERE platform = ? AND channel_id = ?` with no thread filter (mirror.rs:147-156) and picks the most recently updated thread session. `mirror_to_session` then inserts the cron output there as an `assistant` message.
- Impact: The cron output was posted to the channel but is written into an unrelated thread's transcript. That thread's agent then sees a message it never sent, and it can bleed into the context of a different conversation.
- Fix: Drop the second, thread-unfiltered query for the `thread_id = None` case. Return `None` when no channel-level session exists.

### [P1] OpenAI speech-to-text HTTP client has no timeout, so one stalled request wedges inbound message handling
- Location: src/voice/pipeline.rs:141
- Trigger: `main.rs:1061` builds `AttachmentDownloader::with_stt(OpenAiSpeechToText::new(..))`. `route_claimed_event_with_constituents` (src/discord/adapter.rs:1494) calls `downloader.hydrate(attachment).await` for every attachment. For a voice attachment, `hydrate` calls `stt.transcribe`. `OpenAiSpeechToText` uses `reqwest::Client::new()`, which has no request timeout. If the OpenAI endpoint accepts the connection and never answers, the await never completes.
- Impact: The delivery was already recorded in the ledger (adapter.rs:1514) but is never routed to the multiplexer. For the debounced path that task hangs forever. If the process is restarted, the delivery is then treated as a duplicate (`Ignoring duplicate Discord delivery`) and the message is lost.
- Fix: Build the client with `reqwest::Client::builder().timeout(..)`, for example 60s. Optionally wrap `transcribe` in `tokio::time::timeout`.

### [P2] Readiness backend probe uses the wrong default port and the wrong env var
- Location: src/readiness.rs:240
- Trigger: `collect_runtime_readiness` (called at src/main.rs:1079) reads `OMON_APPSERVER_URL` (readiness.rs:292) and defaults to `http://127.0.0.1:18800`. The real backend config reads `OMON_OMO_APPSERVER_URL` and defaults to `ws://127.0.0.1:19742` (src/agent/omo_config.rs:224-228), which is where the supervised daemon listens. In the zero-config setup the probe hits a port nothing listens on.
- Impact: Startup readiness always logs `degraded` ("backend connection refused or unreachable") for a healthy gateway. If an operator sets `OMON_OMO_APPSERVER_URL` to a non-default value, the probe still checks the wrong host. `dashboard.rs:848` shares the same env-var mismatch.
- Fix: Take the appserver URL from the resolved `OmoBackendConfig.appserver_url` (or `OMON_OMO_APPSERVER_URL`, default `ws://127.0.0.1:19742`). Remove the hard-coded 18800 default.

### [P2] Inbox filter-drop counts use mismatched day boundaries (Seoul vs UTC)
- Location: src/usage.rs:232
- Trigger: `usage::yesterday` (called from src/inbox/digest.rs:93) computes the report window as the previous Asia/Seoul calendar day. It then sums `inbox_filter_stats WHERE day = <Seoul previous date>`. `inbox_filter_stats.day` is written with `Utc::now().date_naive()` (src/inbox/prefilter.rs:181). In `report()`, `since.date_naive()` and `until.date_naive()` (usage.rs:60) are UTC dates compared with `day>=? AND day<?`. For a 24h window this excludes the current UTC day's drops and includes the full starting day.
- Impact: The digest line "인박스 수집 N건 중 필터 통과 M건" reports a collected total whose dropped component covers a different 24h than the passed component. The `usage --json` `inbox_collected` field is wrong the same way.
- Fix: Bucket `inbox_filter_stats.day` in the same timezone as the report (store the Seoul date), or convert the window to UTC dates consistently.

### [P2] `apply_block` does not bump `bot_profiles.updated_at`
- Location: src/profiles/lanes_block.rs:172
- Trigger: `omo-gateway profile apply-block` (src/entry.rs:160) runs `UPDATE bot_profiles SET system_prompt = ? WHERE bot_id = ?`. The dashboard update path sets `updated_at = ?` in its own UPDATE (src/dashboard.rs:1999-2006). The column has no trigger (no migration defines one).
- Impact: `profile export` and the dashboard show a stale `updated_at` after the system prompt has changed, so change tracking is wrong.
- Fix: Use `UPDATE bot_profiles SET system_prompt = ?, updated_at = CURRENT_TIMESTAMP WHERE bot_id = ?`.

### [P2] Local audio transcription needs ffmpeg and whisper-cli, which the shipped image does not install
- Location: src/voice_transcription.rs:22
- Trigger: `route_claimed_event_with_constituents` (src/discord/adapter.rs:1531-1536) calls `transcribe_audio` for every audio attachment. `transcribe_audio` spawns `ffmpeg` and `whisper-cli`. The runtime stage of the Dockerfile installs only `ca-certificates` and `libopus0`. A voice message therefore fails with "ffmpeg spawn failed". This also happens when `hydrate` has already produced a transcript through OpenAI STT.
- Impact: The adapter appends "⚠️ 음성 전사 실패" to the event content for every audio attachment in the containerised deploy. With `OPENAI_API_KEY` set, that note sits next to a successful transcript in `attachment.text_content`, so the agent gets contradictory input.
- Fix: Install ffmpeg and whisper-cli in the image. Alternatively skip the local transcription when `attachment.text_content` is already set, or when the binaries are not on PATH.
