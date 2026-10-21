<div align="center">

```
 ██████╗ ███╗   ███╗ ██████╗ 
██╔═══██╗████╗ ████║██╔═══██╗
██║   ██║██╔████╔██║██║   ██║
██║   ██║██║╚██╔╝██║██║   ██║
╚██████╔╝██║ ╚═╝ ██║╚██████╔╝
 ╚═════╝ ╚═╝     ╚═╝ ╚═════╝ 
   G  A  T  E  W  A  Y
```

**High-performance, Zero-GC Discord Multiplexer Gateway for OMO (oh-my-openagent) in 100% Rust.**

[![Rust](https://img.shields.io/badge/Rust-1.78+-f74c00?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-Async_Runtime-232f3e?style=flat-square&logo=rust&logoColor=white)](https://tokio.rs/)
[![Discord](https://img.shields.io/badge/Discord-Gateway_v10-5865F2?style=flat-square&logo=discord&logoColor=white)](https://discord.com/developers/docs)
[![License](https://img.shields.io/badge/License-Apache_2.0-22c55e?style=flat-square)](LICENSE)
[![Zero GC](https://img.shields.io/badge/GC-Zero_Pause-a855f7?style=flat-square)]()
[![Platform](https://img.shields.io/badge/Platform-Linux_|_macOS_|_Docker-0ea5e9?style=flat-square)]()

---

### The Native Discord Bridge for OMO

**OMO Gateway** is a dedicated, ultra-fast Rust gateway that bridges **OMO (oh-my-openagent)** directly to Discord.<br/>
It multiplexes thousands of concurrent channels, threads, and DMs with sub-millisecond routing, multi-bot sharding, scale-to-zero memory reclamation, and sandboxed tool execution.

</div>

---

## ⚡ Why OMO Gateway?

**OMO (oh-my-openagent)** provides autonomous agent intelligence, deep planning, and subagent orchestration. However, running AI agents directly against Discord's WebSocket APIs creates operational bottlenecks:

- **Heavy Idle Memory**: Running individual Discord connections per agent consumes hundreds of megabytes.
- **Concurrency & Rate Limits**: Managing token streaming across many channels triggers Discord rate-limit bans without centralized debouncing.
- **Multi-Bot Management**: Running multiple bot identities requires running multiple redundant runtime instances.

**OMO Gateway solves this** by acting as a high-throughput, pure-Rust I/O multiplexer sitting between Discord and OMO.

---

## 🏗️ Architecture

```
[ Discord Ingress: DMs / Server Channels / Threads / Voice ]
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                    OMO GATEWAY (Pure Rust)                  │
│                                                             │
│  ┌─────────────────────────┐   ┌─────────────────────────┐  │
│  │   Session Multiplexer   │   │     Delivery Ledger     │  │
│  │   (Lock-Free DashMap)   │   │  (SQLite WAL Idempotent)│  │
│  └────────────┬────────────┘   └─────────────────────────┘  │
│               │                                             │
│               ▼                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │       Bounded Actor Worker Pool (tokio::task)         │  │
│  │     - Scale-to-Zero GC (Idle Session Eviction)        │  │
│  │     - Multi-Bot Sharding (N Bots in 1 Binary)         │  │
│  └────────────────────────────┬──────────────────────────┘  │
└───────────────────────────────┼─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                   OMO AGENT EXECUTION ENGINE                │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  LLM Streaming & Tool-Call Loop (OpenAI/Anthropic/...)│  │
│  └────────────────────────────┬──────────────────────────┘  │
│                               │                             │
│  ┌────────────────────────────┴──────────────────────────┐  │
│  │ Native Tools: PTY Terminal / File CRUD / MCP / Web    │  │
│  │ Dedicated Workspace Isolation (~/.omon/workspace)     │  │
│  └───────────────────────────────────────────────────────┘  │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    DISCORD RESPONSE EGRESS                  │
│  - Live-Edit Debounced Streaming (800ms sliding window)     │
│  - Interactive Smart Approvals ([Approve] / [Reject] UI)    │
│  - Discord /voice join: STT, session reply, TTS playback   │
│  - Scheduled Cron Event Push Dispatch                       │
└─────────────────────────────────────────────────────────────┘
```

---

## ✨ Key Features

- ⚡ **Lock-Free Session Multiplexing**: Routes messages across servers, threads, and DMs using composite session keys `(platform, guild_id, channel_id, thread_id, user_id)` with zero lock contention.
- 🤖 **Multi-Bot Parallel Sharding**: Run and control multiple Discord bot identities simultaneously from a single compiled ~20MB binary.
- 📉 **Scale-to-Zero GC**: Inactive sessions automatically flush their state to SQLite and evict worker tasks, reducing idle memory footprint to near zero.
- 🎙️ **Discord voice channels**: `/voice join` listens with Songbird, transcribes speech, runs the existing session, and plays the reply back. `/voice leave` disconnects. The text reply lands in the channel where the command was run.
- ⏰ **Autonomous Cron Engine**: Persistent SQLite scheduler that triggers background agent prompt runs and command executions, pushing results directly to designated Discord channels.
- 🛠️ **Native Tool Suite**:
  - **PTY Terminal**: Execute commands in an isolated workspace (`~/.omon/workspace`).
  - **File Tools**: Sandboxed file read, write, and directory inspection.
  - **Browser & Web**: Chrome CDP control (port 9333), live web search, and text extraction.
  - **Model Context Protocol (MCP)**: Connect to external tools via stdio and SSE.
- 🛡️ **Smart Approval Guard**: Interactively request user confirmation in Discord via button components before running dangerous shell commands.

---

## 🔄 Hermes to OMO Gateway Migration Guide (for LLMs & Automations)

`omon-gateway migrate` performs the shipped one-click migration from Hermes Agent to OMO Gateway. Run it from the gateway directory so the imported configuration is written to that directory's `.env`:

```bash
# Preview the complete migration without writing files, changing the database,
# signaling processes, or invoking launchctl.
cargo run -- migrate --dry-run

# Import configuration and cron jobs, then retire the Hermes cron stores and gateway.
cargo run -- migrate
```

The compiled binary accepts the same subcommand (`omo-gateway migrate`). The default flow runs in this order:

1. Import Hermes configuration from `$HERMES_HOME` (default: `~/.hermes`), including the root `.env`, `config.yaml`, and profile `.env` files.
2. Authoritatively rewrite the gateway `.env`. If `.env` already exists, its complete previous contents are first copied to `.env.bak-<timestamp>`.
3. Synchronize Hermes cron jobs into the gateway SQLite `cron_jobs` table.
4. Verify each Hermes job exists in the gateway database, back up each non-empty `jobs.json` as `jobs.json.bak-omon-migration-<timestamp>`, and atomically replace its job list with an empty list.
5. Stop live Hermes gateway processes recorded by the root and profile `gateway.lock` files. On macOS, matching `~/Library/LaunchAgents/ai.hermes.gateway*.plist` services are booted out with `launchctl` and the plist files are renamed to `.plist.disabled` so launchd cannot restart them.

The backups and `.disabled` LaunchAgent files make the file cutover reversible; the original files are retained rather than deleted.

### Migration modes

| Command | Behavior |
|---|---|
| `omo-gateway migrate` | Full import and cutover. Writes the authoritative `.env`, imports cron jobs, empties backed-up Hermes cron stores after verification, and stops/disables the Hermes gateway. |
| `omo-gateway migrate --dry-run` | Read-only projection of configuration, cron, process, and LaunchAgent changes. It performs zero writes and does not run cron synchronization or destructive side effects. |
| `omo-gateway migrate --no-cutover` | Import only. Writes the gateway `.env` and synchronizes cron jobs, but does not empty Hermes cron stores or stop/disable Hermes services. |

Use `--no-cutover` when Hermes must remain available during a staged migration. Do not run both gateways against the same bot tokens and cron schedules after the final cutover.

### What the importer maps

The command performs these mappings automatically; this table is a reference, not a manual copy-and-paste procedure.

| Hermes Location | Hermes Key | OMO Gateway `.env` Key | Mapping behavior |
|---|---|---|---|
| `$HERMES_HOME/.env` | `DISCORD_BOT_TOKEN` | `DISCORD_BOT_TOKEN` | Preserves the primary token. |
| `$HERMES_HOME/profiles/*/.env` | `DISCORD_BOT_TOKEN` | `DISCORD_BOT_TOKENS` | Collects non-empty profile tokens in stable order and removes duplicates, including the primary token. |
| `$HERMES_HOME/.env` or profile `.env` | `DISCORD_ALLOWED_USERS` | `DISCORD_ALLOWED_USERS` | The root value wins; otherwise the first profile value is used. |
| `$HERMES_HOME/.env` or profile `.env` | `DISCORD_FREE_RESPONSE_CHANNELS` | `DISCORD_FREE_RESPONSE_CHANNELS` | The root value wins; otherwise the first profile value is used. |
| `$HERMES_HOME/.env` or profile `.env` | `DISCORD_HOME_CHANNEL` | `DISCORD_HOME_CHANNEL` | Preserved; the gateway also treats these as free-response channels at runtime. |
| `$HERMES_HOME/.env` or `config.yaml` | `DISCORD_ALLOWED_CHANNELS` or `discord.allowed_channels` | `DISCORD_ALLOWED_CHANNELS` | Preserves allowed channel filters. |
| `$HERMES_HOME/.env` or `config.yaml` | `DISCORD_IGNORED_CHANNELS` or `discord.ignored_channels` | `DISCORD_IGNORED_CHANNELS` | Preserves ignored channel filters. |
| `$HERMES_HOME/.env` or `config.yaml` | `DISCORD_ALLOWED_ROLES` or `discord.allowed_roles` | `DISCORD_ALLOWED_ROLES` | Preserves authorized Discord role IDs. |
| `$HERMES_HOME/config.yaml` | `model.default` / `model.name` / `model.model` | `DEFAULT_MODEL` | Preserves the configured model identifier. |
| `$HERMES_HOME/config.yaml` | `model.base_url`, `model.api_key` | `ANTHROPIC_BASE_URL`, `ANTHROPIC_API_KEY` | Used when the default model name starts with `claude`. |
| `$HERMES_HOME/config.yaml` | `model.base_url`, `model.api_key` | `OPENAI_API_BASE`, `OPENAI_API_KEY` | Used for other model names. |
| `$HERMES_HOME/.env` or `config.yaml` | `APPROVAL_MODE` or `approvals.mode` | `APPROVAL_MODE` | The root `.env` value wins; otherwise the YAML approval mode is used. |

Unmapped target keys from an existing gateway `.env` are preserved during merge; backups are recorded before any modifications.

### Cron stores, workspace, and skills

- **Cron stores**: Hermes jobs are read from `$HERMES_HOME/cron/jobs.json` and `$HERMES_HOME/profiles/*/cron/jobs.json`. `OMON_HERMES_PROFILES` can restrict the profiles synchronized by the runtime; when unset, the default store and profile directories are discovered automatically. Cron pre-run scripts default to an 1800s (30-minute) timeout, configurable globally via `OMON_CRON_SCRIPT_TIMEOUT_SECS` or per job via `timeout_seconds`/`timeout`.
- **Workspace & Authorized Roots**: When `OMON_WORKSPACE_ROOT` is unset, OMO Gateway isolates terminal and file tools under `$HOME/.omon/workspace`. Additional authorized directory paths can be configured via `OMON_TOOL_ROOTS` (colon-separated absolute paths; defaults to `$HOME` when unset; workspace is always allowed; approval policies and hardline command guards still apply).
- **Skills**: OMO Gateway scans `$HERMES_HOME/skills` (default: `~/.hermes/skills`) and `~/.omon/skills` for `SKILL.md` bundles.

---

## 🚀 Quickstart

### 1. Build and Run from Source

```bash
# Clone the repository
git clone https://github.com/Indosaram/omon-gateway.git
cd omon-gateway

# Configure environment
cp .env.example .env
# Edit .env with your Discord bot tokens and LLM endpoint

# Build and run optimized release binary
cargo run --release
```

### 2. Run with Docker Compose

```bash
docker compose up -d
```

The image does not include the `omo` CLI, so the container connects to an `omo app-server` running on the host (`OMON_OMO_APPSERVER_URL`, default `ws://host.docker.internal:19742`; set `OMON_OMO_APPSERVER_AUTH_TOKEN` if the server uses `--ws-auth`). Start the host daemon listening on an address the container can reach before `docker compose up`.

SQLite data and the agent workspace persist in the named volumes `omon-data` and `omon-workspace`, which are writable by the image's non-root user (uid 10001). Bind mounts of `./data` and `./workspace` would be root-owned on Linux and break the database.

### 3. Run as macOS Background Service (LaunchAgent)

```bash
# Build release binary
cargo build --release --bin omon-gateway

# Install LaunchAgent
cp ai.omon.gateway.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/ai.omon.gateway.plist
```

---

## 🎙️ Voice

Discord voice turns on when an OpenAI-compatible speech key is set (`OPENAI_API_KEY`, or both `OMON_STT_API_KEY` and `OMON_TTS_API_KEY`). `OMON_VOICE_ENABLED=false` turns it off. The bot needs the **Connect** and **Speak** permissions, and the `GUILD_VOICE_STATES` gateway intent (registered automatically when voice is ready).

`/voice join` joins the caller's current voice or stage channel, or the channel passed to the command. Incoming audio is decoded to 16 kHz mono PCM, transcribed, and sent to the same session multiplexer as a typed message (prefixed `[voice]`). The assistant reply is posted in the text channel or thread where the command ran, and synthesized speech is played in the voice channel. A new utterance stops the previous reply. The bot does not join a voice channel by itself.

Slack audio **file** attachments are still transcribed locally (`ffmpeg` + `whisper-cli`). Set `SLACK_VOICE_REPLY=true` to also upload a WAV of the final assistant reply in that thread. The spoken file is that final posted text, not interim status messages.

Slack's own API exposes huddle presence only. It cannot join a huddle or exchange audio. Recall.ai can record a huddle, and its published matrix marks Slack audio output unavailable, so it cannot play a reply back. Two-way huddle voice therefore uses a dedicated Slack **user** signed into the Slack web client, the same approach as the OpenClaw [`slack-huddles` plugin](https://github.com/openclaw/openclaw/tree/main/extensions/slack-huddles) (MIT, OpenClaw Foundation). `sidecar/slack-huddle/index.mjs` is an independent implementation of that approach. It does not copy OpenClaw's sources, does not read Slack session tokens or cookies, does not call Slack's internal huddle endpoints, and does not connect to AWS Chime. Login happens in that Chrome profile.

The feature is **off** until `OMON_SLACK_HUDDLE_ENABLED=true`. It also needs the same STT and TTS keys as Discord voice, Node.js 22 or newer, and Chrome or Chromium on a machine with a display. `/omo-huddle` (register it from `docs/slack-app-manifest.json`) accepts `join`, `leave`, `status`, `speak`, and `transcript`.

```bash
# Linux virtual microphone. The device label must match OMON_SLACK_HUDDLE_MIC_LABEL.
pactl load-module module-null-sink sink_name=omo_meeting \
  sink_properties=device.description="Omo Gateway Meeting Audio"
pactl load-module module-remap-source master=omo_meeting.monitor source_name=omo_meeting_mic \
  source_properties=device.description="Omo Gateway Meeting Audio"

# macOS: brew install blackhole-2ch sox
# Select "BlackHole 2ch" as the huddle microphone. Playback uses sox to that device.
```

Create a Chrome profile used only by this account, start the gateway with `OMON_SLACK_HUDDLE_PROFILE` pointing at it, and sign into Slack in the window the sidecar opens. The account must already be a member of the channel. Someone else starts the huddle. `/omo-huddle join` takes a Copy huddle link (`https://app.slack.com/huddle/T…/C…`) or a channel id (`C…`, `G…`, or `D…`, optionally `channel:` or `team:T…:channel:C…`). The sidecar clicks Join on the web page, stays muted until Slack shows the virtual microphone, then unmutes. Remote audio from the page is transcribed, sent through the existing session multiplexer with a `[voice]` prefix, and the spoken reply is played into that microphone. A new utterance stops the previous reply. `/omo-huddle speak` says a sentence without asking the model. `/omo-huddle leave` clicks Leave and does not end the huddle for anyone else. `/omo-huddle transcript` shows the speech-to-text lines and any captions already visible on the page. Turn on Slack's "captions by default" preference on the dedicated account if you want captions; the sidecar does not open that menu.

Limits: one huddle per dedicated account; only an already-active huddle; English Slack UI; the tab has to stay on that channel; the adapter can break when Slack changes the web UI. Slack does not sanction automated user clients, so the dedicated account can be suspended. Use an account you can afford to lose, not a personal or primary workspace owner.

| Environment Variable | Default | Description |
|---|---|---|
| `OMON_VOICE_ENABLED` | on when STT and TTS keys resolve | `false` disables Discord voice join/playback |
| `OMON_STT_API_KEY` | `OPENAI_API_KEY` | Key for `/audio/transcriptions` |
| `OMON_STT_BASE_URL` | `OPENAI_API_BASE` | OpenAI-compatible STT base URL |
| `OMON_STT_MODEL` | `whisper-1` | STT model for live voice and Discord voice-note attachments |
| `OMON_TTS_API_KEY` | `OPENAI_API_KEY` | Key for `/audio/speech` |
| `OMON_TTS_BASE_URL` | `OPENAI_API_BASE` | OpenAI-compatible TTS base URL |
| `OMON_TTS_MODEL` | `tts-1` | TTS model (`response_format=pcm`) |
| `OMON_TTS_VOICE` | `alloy` | TTS voice name |
| `OMON_VOICE_SILENCE_MS` | `700` | Trailing silence that closes an utterance |
| `OMON_VOICE_MIN_SPEECH_MS` | `400` | Shorter bursts are ignored |
| `OMON_VOICE_MAX_SPEECH_MS` | `30000` | Force-flush a long utterance |
| `SLACK_VOICE_REPLY` | `false` | Upload a spoken WAV for Slack audio-file replies when TTS is configured |
| `OMON_SLACK_HUDDLE_ENABLED` | `false` | Drive a signed-in Slack web client into an existing huddle |
| `OMON_SLACK_HUDDLE_PROFILE` | unset | Chrome user-data directory for the dedicated Slack account |
| `OMON_SLACK_HUDDLE_CHROME` | first Chrome/Chromium on `PATH` | Browser binary |
| `OMON_SLACK_HUDDLE_NODE` | `node` | Node.js 22+ used to run the sidecar |
| `OMON_SLACK_HUDDLE_SIDECAR` | `sidecar/slack-huddle/index.mjs` when that file exists | Sidecar script |
| `OMON_SLACK_HUDDLE_CDP_PORT` | unset | Attach to a Chrome you already started with remote debugging, instead of launching one |
| `OMON_SLACK_HUDDLE_MIC_LABEL` | `Omo Gateway Meeting Audio` (Linux), `BlackHole 2ch` (macOS) | Microphone the sidecar selects before unmuting |
| `OMON_SLACK_HUDDLE_PLAYBACK` | `pw-cat` into `omo_meeting` (Linux), `sox` into BlackHole (macOS) | Command that plays raw s16le on stdin. `$OMO_HUDDLE_RATE` is the sample rate |
| `OMON_SLACK_HUDDLE_JOIN_TIMEOUT_MS` | `30000` | How long the sidecar waits for the huddle UI |

---

## ⚙️ Configuration Reference

| Environment Variable | Default | Description |
|---|---|---|
| `DISCORD_BOT_TOKEN` | Required | Primary Discord Bot Token |
| `DISCORD_BOT_TOKENS` | Optional | Comma-separated tokens for Multi-Bot sharding |
| `DISCORD_ALLOWED_USERS` | Optional | Allowed Discord user IDs (empty allows all) |
| `DISCORD_ALLOWED_ROLES` | Optional | Comma-separated Discord role IDs allowed to interact with the bot |
| `DISCORD_ALLOW_ALL_USERS` | `false` | When true, bypasses user/role allowlists and grants access to all users |
| `DISCORD_ALLOWED_CHANNELS` | Optional | Comma-separated channel IDs allowed to process messages (DMs exempt) |
| `DISCORD_IGNORED_CHANNELS` | Optional | Comma-separated channel IDs ignored completely |
| `DISCORD_FREE_RESPONSE_CHANNELS` | Optional | Channels where the bot responds without @mention |
| `DISCORD_AUTO_THREAD` | `false` | When true, @mentions in guild text channels auto-create a public thread and route responses there |
| `DISCORD_THREAD_SESSIONS_PER_USER` | `true` | When false, all users in a thread share the same conversation session |
| `DISCORD_THREAD_REQUIRE_MENTION` | `false` | When true, requires an @-mention to respond even in active bot threads |
| `DISCORD_ALLOW_BOTS` | `none` | Bot handling policy: `none` (default), `mentions` (respond only when @-mentioned), `all` |
| `DISCORD_CHANNEL_CONTEXT` | `false` | When true, @mentions in guild channels backfill recent channel history as conversation context |
| `DISCORD_CHANNEL_CONTEXT_LIMIT` | `10` | Maximum preceding messages to backfill when channel context is enabled (clamped to <= 25) |
| `DISCORD_CHANNEL_TOPIC_CONTEXT` | `false` | When true, fetches channel topic and forum parent description as prompt context |
| `DISCORD_CHANNEL_PROMPTS` | Optional | JSON object mapping channel IDs to custom `system_prompt` and `skills` |
| `DISCORD_PROCESSING_REACTIONS` | `true` | When true, adds 👀 reaction when processing starts, swapping to ✅ on success or ❌ on failure |
| `DISCORD_CHUNK_PAGINATION` | `true` | When true, adds (i/N) pagination indicator headers to messages split across multiple chunks |
| `DISCORD_RUNTIME_FOOTER` | `false` | When true, appends a compact runtime metadata footer (`model · context% · cwd`) to the final assistant message |
| `DISCORD_PROFILE_ROUTES` | Optional | JSON array of hierarchical profile routing rules mapping `(guild, channel, thread)` to custom models, prompts, and toolsets |
| `DEFAULT_MODEL` | `gpt-4o` | Default LLM model identifier |
| `OPENAI_API_BASE` | `https://api.openai.com/v1` | OpenAI-compatible endpoint URL |
| `OPENAI_API_KEY` | Optional | OpenAI API key |
| `ANTHROPIC_BASE_URL` | Optional | Anthropic Messages endpoint URL |
| `ANTHROPIC_API_KEY` | Optional | Anthropic API key |
| `DATABASE_URL` | `sqlite://omon_gateway.db` | SQLite database path (WAL mode) |
| `OMON_WORKSPACE_ROOT` | `$HOME/.omon/workspace` | Dedicated sandboxed working directory used by terminal and file tools |
| `OMON_TOOL_ROOTS` | `$HOME` | Optional colon-separated absolute paths authorized for tool access (defaults to HOME; workspace always allowed; approval and hardline guards apply) |
| `HERMES_HOME` | `$HOME/.hermes` | Hermes root used by migration, cron synchronization, and Hermes skill discovery |
| `OMON_HERMES_PROFILES` | Auto-discover | Optional comma-separated Hermes cron profiles; when unset, synchronizes `default` plus discovered profile directories |
| `OMON_CRON_SCRIPT_TIMEOUT_SECS` | `1800` | Timeout in seconds for cron pre-run / script executions (default 1800 / 30m; overridable per job via `timeout_seconds` or `timeout`) |
| `APPROVAL_MODE` | `smart` | Enforced terminal approval policy: `smart` gates dangerous commands, `always` gates every command, and `never`/`yolo` bypass approval |
| `APPROVAL_TIMEOUT_SECS` | `900` | Seconds to wait for a Discord command approval before the request expires (default 900) |
| `APPROVALS_DENY` | Optional | Comma-separated wildcard globs (`npm publish *,kubectl delete *`) unconditionally blocked before policy, YOLO, or allowlists |
| `DISCORD_APPROVAL_MENTIONS` | `false` | When true, @-mentions allowed users (<@uid>) on approval prompts for push notifications |

---

## 📜 License

Licensed under the [Apache License 2.0](LICENSE).
