# Mobile Monitoring & Control of Coding Agents (2025–2026)

Seen: Trello, Jira, Linear, Notion, Asana, ClickUp, monday, Plane, WeKan, Kanboard, Planka, Vikunja, Focalboard, Taiga, Huly, Vibe Kanban, Task Master, Backlog.md

### Replit Agent (Replit Mobile)
- URL: https://replit.com/mobile
- Kind: agent-native
- What: Replit's official native mobile IDE application provides first-class support for launching, monitoring, and steering autonomous Replit Agent workflows. Developers can track step-by-step agent checkpoint progress, review generated code diffs, preview live web applications, and deploy directly from smartphones.
- Agent integration: Built-in proprietary Replit Agent runtime; WebSocket event streams; cloud dev environments (Nix containers, PostgreSQL, hosted services); mobile chat and prompt interface.
- Mobile: Native iOS app (Apple App Store) and native Android app (Google Play Store) with mobile-optimized checkpoint controls, live webview previews, and interactive agent steering (https://replit.com/mobile).
- Pricing: Free tier with limited Agent checkpoints/credits; Replit Core at $15/month billed annually ($20/month monthly); Replit Teams at $30/user/month (pricing verified October 2026).
- Self-host: no (cloud-only proprietary platform).
- License: Proprietary
- Activity: Highly active production platform; weekly releases on iOS and Android; millions of active developers worldwide.
- Pros: True native mobile app experience with zero configuration required; interactive checkpoint review and live web application preview running inside the app; instant push notifications upon agent task milestone or pause; rollback and code diff viewing on mobile screens.
- Cons: Fully proprietary and locked into the Replit cloud ecosystem; agent compute credits are rapidly consumed on free and basic tiers; no support for self-hosted containers or offline execution.
- Sources: https://replit.com/mobile, https://replit.com/pricing
- Confidence: high

### GitHub Mobile (Copilot Workspace / Agent PRs)
- URL: https://github.com/mobile
- Kind: agent-native
- What: GitHub's official native iOS and Android application includes integrated GitHub Copilot and Copilot Workspace features for mobile developers. It enables monitoring autonomous coding agent pull requests, tracking specification checklists, reviewing AI-generated code diffs, approving workflow runs, and steering issue tasks while away from a computer.
- Agent integration: Official GitHub Copilot, Copilot Workspace task planner, GitHub REST/GraphQL APIs, GitHub Actions workflow webhooks, pull request automation bots.
- Mobile: Native iOS app (Apple App Store) and native Android app (Google Play Store) featuring push notifications, interactive task lists, and syntax-highlighted diff viewers (https://github.com/mobile).
- Pricing: GitHub Mobile app is free; GitHub Copilot Individual at $10/month, Business at $19/user/month, Enterprise at $39/user/month (verified October 2026).
- Self-host: no (GitHub.com cloud platform; GitHub Enterprise Server supports mobile app if configured with corporate policy).
- License: Proprietary
- Activity: Highly active; updated every 1-2 weeks on Apple App Store and Google Play; widely deployed across millions of developers.
- Pros: Native mobile notification architecture for instant alerts when an agent finishes an issue or submits a PR; high-quality mobile code diff viewer with inline line-by-line review comments; enterprise SSO and hardware key security support; home screen widgets for active tasks and notifications.
- Cons: Does not allow running local CLI coding agents directly from the mobile app; Copilot Workspace mobile experience focuses primarily on task review and specification approval rather than full terminal interaction; proprietary cloud platform.
- Sources: https://github.com/mobile, https://github.com/features/copilot
- Confidence: high

### PocketShell
- URL: https://pocketshell.app
- Kind: agent-native
- What: A mobile remote terminal and AI agent companion that connects iOS and Android devices directly to local coding agent sessions (Claude Code, OpenAI Codex CLI) via end-to-end encrypted WebRTC peer-to-peer data channels. It delivers a full PTY terminal, live activity monitoring, structured agent stdio streaming, and mobile push alerts.
- Agent integration: Dedicated WebRTC `agent-{id}` channel streaming Claude Code and OpenAI Codex CLI stdio via framed JSON; host agent CLI daemon; local PTY bridge.
- Mobile: Native iOS app and native Android app (closed-source companion apps on Apple App Store and Google Play Store paired via QR code) (https://pocketshell.app).
- Pricing: Free tier available for single-device pairing; Pro tier available for host-to-host direct file transfers and unlimited devices ($5–$10/month on website, October 2026).
- Self-host: yes for open-source host daemon (Apache-2.0 Rust binary compiled via cargo or installed locally); cloud signaling broker is hosted proprietary.
- License: Open-source host agent under Apache-2.0 (`yashagldit/PocketShell`), proprietary mobile apps and signaling server.
- Activity: 10 GitHub stars, pushed July 2026; active development in v0.x beta.
- Pros: True peer-to-peer WebRTC data channel with DTLS encryption and ED25519-signed SDP transcripts; dedicated agent stream channel so traffic never touches central servers; low-latency real PTY with native touch gestures; push notification integration for agent pauses and completions.
- Cons: Mobile apps and signaling infrastructure are closed source; requires compiling or running a Rust host daemon on the local developer machine; relatively new project with a small community.
- Sources: https://github.com/yashagldit/PocketShell, https://pocketshell.app
- Confidence: high

### Herdr Connect
- URL: https://github.com/Tomyail/herdr-connect
- Kind: agent-native
- What: A dedicated native iOS companion app and local daemon for monitoring and controlling Herdr AI coding agents over private LAN or Tailscale networks with zero cloud relay. It lets developers view real-time agent lists, inspect active terminal outputs, interrupt or steer turns, and receive native iOS haptic and notification cues upon task completion.
- Agent integration: Direct JSON-RPC/WebSocket API to local Herdr agent daemon; monitors all agents spawned by Herdr (Claude Code, Codex, aider, custom shell agents).
- Mobile: Native iOS app distributed via public Apple TestFlight beta (`https://testflight.apple.com/join/ZkRzJ6rm`), built with Expo / React Native (Android planned but unreleased).
- Pricing: Free and open-source (100% free, no subscriptions or cloud fees).
- Self-host: yes (local daemon installed via curl shell script `~/.local/bin/herdr-connect`, runs on TCP port 9808 locally).
- License: Apache-2.0
- Activity: 19 GitHub stars, pushed October 15, 2026; active TestFlight beta.
- Pros: Native iOS UI with tactile haptics and instant push-style completion alerts over LAN; zero cloud dependencies, accounts, or telemetry; QR-code pairing with local TLS certificate generation; allows direct prompt follow-ups and turn interruption from phone.
- Cons: iOS only (no Android app yet); requires both devices to be on the same local Wi-Fi or connected via Tailscale/VPN; depends on Herdr being installed on host.
- Sources: https://github.com/Tomyail/herdr-connect, https://testflight.apple.com/join/ZkRzJ6rm
- Confidence: high

### Claude Remote Runner
- URL: https://remote-claude.shaniai.tech
- Kind: agent-native
- What: An open-source CLI wrapper and Progressive Web App (PWA) enabling secure, end-to-end encrypted remote control of Claude Code sessions from mobile devices. It streams terminal execution in real-time via WebRTC and sends push notifications to the phone whenever Claude pauses for permissions or human feedback.
- Agent integration: Claude Code CLI wrapper with automatic hook installation; captures agent pause states, prompt requests, and tool calls.
- Mobile: Progressive Web App (PWA) accessible via QR code scan from native mobile cameras on iOS and Android browsers (https://remote-claude.shaniai.tech).
- Pricing: Free and open-source (npm package `@sohampawar1866/remote-claude`, optional self-hosted Appwrite signaling server).
- Self-host: yes (CLI wrapper runs locally on Node.js; PWA frontend and Appwrite signaling backend can be self-hosted via Docker).
- License: MIT
- Activity: 11 GitHub stars, npm package released, pushed April 30, 2026.
- Pros: Instant QR-code pairing between terminal and mobile browser; real-time low-latency streaming over WebRTC; proactive mobile push notifications on pause and approval events; zero-config public signaling option included.
- Cons: Relies on fallback silence thresholds and pattern debouncing for pause detection; requires WebRTC network conditions that permit direct peer connections or STUN/TURN fallback; smaller community.
- Sources: https://github.com/sohampawar1866/claude-remote-runner, https://www.npmjs.com/package/@sohampawar1866/remote-claude
- Confidence: high

### Herdr Mobile Relay
- URL: https://github.com/0cv/herdr-mobile-relay
- Kind: agent-native
- What: A mobile web app and PWA for Android and iOS that aggregates and controls Herdr AI coding agents across multiple Linux and macOS host computers into one installable mobile interface. It supports push notifications, QR code pairing, multi-computer relays, and remote task approval.
- Agent integration: Herdr plugin architecture (`herdr plugin install 0cv/herdr-mobile-relay`); interacts with Herdr server protocol, agent processes, and workspace files.
- Mobile: Installable Progressive Web App (PWA) with manifest, standalone display mode, and Web Push notifications for iOS Safari and Android Chrome (https://github.com/0cv/herdr-mobile-relay).
- Pricing: Free and open-source.
- Self-host: yes (runs as a Herdr plugin on Linux/macOS with Community WebRTC Gateway or Cloudflare Tunnel).
- License: AGPL-3.0
- Activity: 297 GitHub stars, version 0.22.10 released, pushed October 16, 2026.
- Pros: Multi-computer aggregation allows monitoring agents running across multiple developer machines from a single phone PWA; Community WebRTC Gateway provides zero-config tunneling without third-party accounts; web push notifications notify when agents halt or need approval; QR code setup.
- Cons: Does not support native Windows hosts (WSL2 untested); requires Herdr 0.7.5+ runtime; AGPL-3.0 copyleft license.
- Sources: https://github.com/0cv/herdr-mobile-relay, https://herdr.dev
- Confidence: high

### Roamgate
- URL: https://roamgate.dev/
- Kind: agent-native
- What: A mobile-first browser and PWA client for Herdr that brings agent workspaces, coding sessions, live terminals, file trees, and code diff reviews to smartphones. It allows developers to start tasks on desktop and continue inspecting, steering, and reviewing diffs on a phone while the host keeps agents running.
- Agent integration: Connects to local/remote Herdr server API; monitors Claude Code, Codex, and other agents running in Herdr workspaces; inspects conversations and diff annotations.
- Mobile: Installable PWA and responsive web client for iOS Safari and Android Chrome with mobile touch-optimized layouts (https://roamgate.dev/).
- Pricing: Free and open-source.
- Self-host: yes (web client runs locally or self-hosted via static web server/Docker, connects to running Herdr instance).
- License: MIT
- Activity: 298 GitHub stars, pushed October 16, 2026; actively maintained.
- Pros: Clean, responsive UI with dedicated mobile views for file trees, git diffs, and task boards; PWA support allows installation to phone home screen with full-screen experience; bidirectional terminal and conversation inspection; allows seamless handoff from desktop to phone.
- Cons: Requires an active Herdr daemon running on the host machine; does not provide standalone push notifications without external webhook setup; mobile terminal input can be fiddly on virtual keyboards.
- Sources: https://github.com/powerfooI/roamgate, https://roamgate.dev/
- Confidence: high

### ClawBridge
- URL: https://clawbridge.app/
- Kind: agent-native
- What: An open-source mobile dashboard and "mission control" designed specifically for OpenClaw coding agents. It provides a mobile-first UI to watch agent thoughts and tool actions in real time, monitor token expenditures, trigger cron jobs, and manage background agent processes via a zero-config Cloudflare tunnel.
- Agent integration: OpenClaw agent runtime and gateway; WebSocket real-time event streaming; process management API.
- Mobile: Mobile-first web dashboard and installable PWA with QR code terminal pairing; optimized for phone viewports (https://clawbridge.app/).
- Pricing: Free and open-source.
- Self-host: yes (Node.js daemon `node index.js`, Dockerfile included, systemd service units provided).
- License: MIT
- Activity: 231 GitHub stars, pushed March 2026, version 1.2.3.
- Pros: Mobile-first responsive layout tailored specifically for smartphone screens; zero-config remote access via integrated temporary Cloudflare Tunnel and QR code pairing; real-time parallel activity feed showing tool calls and reasoning; built-in Cost Control Center with automated token-saving diagnostics.
- Cons: Tightly coupled to the OpenClaw agent ecosystem; push notifications rely on active browser session/WebSocket rather than native APNs/FCM; development pace has slowed compared to general Claude Code tools.
- Sources: https://github.com/dreamwing/clawbridge, https://clawbridge.app/
- Confidence: high

### Kandev
- URL: https://kandev.ai
- Kind: agent-native
- What: An open-source AI Kanban board and development environment built to orchestrate multiple AI coding agents in parallel, review code changes, and open PRs. It features comprehensive mobile remote access documentation, responsive touch bottom-sheets, and an installable PWA for managing tasks from a smartphone over Tailscale or Cloudflare Tunnel.
- Agent integration: Multi-provider agent runner (Claude Code, OpenAI, Gemini, local models); git worktree orchestration; webhook and WebSocket live updates.
- Mobile: Installable PWA with web-app manifest and standalone display mode for iOS Safari and Android Chrome; custom touch bottom-sheets and thumb-accessible task action bars (`docs/public/mobile-remote-access.md`).
- Pricing: Free and open-source (self-hosted AGPL-3.0); no paid enterprise gate on core features.
- Self-host: yes (binary, Docker, systemd service; default port 38429; no telemetry).
- License: AGPL-3.0
- Activity: 954 GitHub stars, pushed October 16, 2026; very active commit velocity.
- Pros: Dedicated first-class mobile UI design with swipeable bottom sheets and touch-friendly confirmation dialogs; comprehensive official documentation for secure remote mobile connectivity (Tailscale Serve, Cloudflare Access); full kanban board view coupled with agent worktrees; telemetry-free.
- Cons: AGPL-3.0 license may restrict proprietary commercial redistribution; requires network setup (Tailscale, VPN, or Cloudflare Tunnel) for remote mobile access; heavy initial build if compiling from source.
- Sources: https://github.com/kdlbs/kandev, https://kandev.ai
- Confidence: high

### Gander
- URL: https://github.com/weellio/gander
- Kind: agent-native
- What: An ambient NOC dashboard and control center for AI coding agents featuring a pixel-art office floor, a dedicated `/phone` mobile web console, an installable PWA, and instant Telegram alert integration. It attaches to Claude Code (and other CLIs like Codex, Gemini, aider via `gander-wrap`) to display agent activities, request permissions, and answer stuck sessions from a phone.
- Agent integration: Claude Code hooks (auto-attaches to CLAUDE.md and session lifecycle); `gander-wrap` and open event API for Codex, Gemini, Grok, aider; Telegram bot bridge.
- Mobile: Dedicated one-handed mobile console (`/phone`), installable PWA with standalone window for iOS/Android home screens, plus Telegram bot alerts with inline response buttons (`docs/REMOTE.md`).
- Pricing: Free and open-source (MIT); optional donations.
- Self-host: yes (local Node.js bridge server binding port 3131, secured with access tokens and Tailscale/cloudflared tunnel support).
- License: MIT
- Activity: 10 GitHub stars, pushed October 14, 2026; active recent release.
- Pros: Unique dual mobile approach: ultra-lightweight `/phone` page that halts polling when backgrounded to save mobile battery, plus a full visual PWA; bidirectional Telegram bot alerting allows approving permissions and replying `/stop` directly from chat; auto-attaches via Claude Code hooks without manual wiring; token burn and runaway cost monitor.
- Cons: Small star count (early release); full pixel-art floor view is dense on small phone screens (hence the `/phone` view); requires token setup when opening remote access outside loopback.
- Sources: https://github.com/weellio/gander
- Confidence: high

### Agent Teams AI
- URL: https://agentteams.live
- Kind: agent-native
- What: A multi-agent orchestration and team Kanban platform where users command fleets of coding agents (Claude Code, Codex, OpenCode, Cursor, Grok, Kiro, etc.) as autonomous employees. It provides a visual Kanban board, agent collaboration swimlanes, tool execution logs, and a web dashboard accessible from mobile browsers.
- Agent integration: Supports 300+ models and 200+ LLM providers; connects directly to CLI agents (Claude Code, Codex, OpenCode, Cursor, GitHub Copilot) and API providers; task dispatch and review pipelines.
- Mobile: Responsive web dashboard accessible via mobile browser; desktop client available for macOS/Windows/Linux (https://agentteams.live).
- Pricing: Free desktop app and open-source version (AGPL-3.0); connects to free models without authentication or user's own API keys/subscriptions.
- Self-host: yes (desktop app or self-hosted web server from source/Docker).
- License: AGPL-3.0
- Activity: 2,265 GitHub stars, pushed October 16, 2026; rapid release pace.
- Pros: Visual Kanban board specifically tailored for multi-agent teams with dependency tracking and agent-to-agent reviews; connects to virtually every major agent CLI and LLM provider; free tier models available out of the box with zero setup; detailed execution logs with tool call breakdowns.
- Cons: Desktop app is the primary target; mobile access is browser-based without native iOS/Android notifications; UI is complex with many panels that require horizontal panning on narrow phone screens.
- Sources: https://github.com/777genius/agent-teams-ai, https://agentteams.live
- Confidence: high

### cc-connect
- URL: https://github.com/chenhg5/cc-connect
- Kind: agent-native
- What: An open-source gateway bridging local AI coding agents (Claude Code, Cursor CLI, Gemini CLI, OpenAI Codex) to major messaging platforms including Telegram, Discord, Slack, Feishu/Lark, DingTalk, LINE, and WeChat Work. It turns smartphones into remote control consoles for desktop coding agents without requiring a public IP.
- Agent integration: Claude Code, OpenAI Codex, Google Gemini CLI, iFlow CLI, Qoder CLI; WebSocket & subprocess PTY management; MCP tools integration; webhook triggers.
- Mobile: Operates natively through Telegram, Discord, Slack, LINE, and Feishu mobile apps on iOS and Android; also provides a local Web UI (npm/Homebrew/binary).
- Pricing: Free and open-source (MIT); sponsored model API discounts available.
- Self-host: yes (single Go binary download, Homebrew `brew install`, or npm global install; config via TOML/Web UI).
- License: MIT
- Activity: 15,959 GitHub stars, pushed October 14, 2026 (v1.5.1-beta.1); massive community adoption.
- Pros: Works directly inside the mobile chat apps developers already have installed (Telegram, Discord, Slack) with zero mobile battery drain; supports dangerous tool permission approvals via interactive chat buttons; multi-agent orchestration and session switching (`/switch`, `/sessions`); handles bidirectional file and image attachments.
- Cons: Requires running the local daemon on the host machine; conversation interface is text/card based rather than a traditional visual Kanban board; requires creating and configuring bot tokens on respective messaging platforms.
- Sources: https://github.com/chenhg5/cc-connect, https://github.com/chenhg5/cc-connect/releases
- Confidence: high

### cc-haha
- URL: https://cchaha.ai
- Kind: agent-native
- What: A local-first desktop workspace and agent command center for Claude Code and multi-agent teams that includes a built-in H5 mobile web entrance and IM bot integrations (Telegram, WeChat, Feishu, DingTalk, WhatsApp, Slack). Developers can scan a QR code to monitor running coding sessions from a phone browser or approve permissions via messaging apps.
- Agent integration: Claude Code, ChatGPT/Codex, Grok, DeepSeek, Kimi, GLM, local Ollama/LM Studio models; MCP server graphical management (STDIO, HTTP, SSE); Git worktrees and diff inspection.
- Mobile: H5 mobile web access via QR code scan (keeps running when screen locks) plus native mobile messaging bots on Telegram, WeChat, WhatsApp, Feishu, and Slack (https://cchaha.ai).
- Pricing: Free for personal and internal enterprise use; commercial license required for paid distribution or commercial services (Community Source License 1.0).
- Self-host: yes (desktop client for macOS/Windows/Linux, with local web server for mobile H5 access).
- License: cc-haha Community Source License 1.0 (Source-available)
- Activity: 15,025 GitHub stars, pushed October 15, 2026; very active development.
- Pros: Dual mobile accessibility: quick H5 mobile web browser view via QR code and deep multi-platform chat bot bridge; rich visual desktop companion with Git worktrees, syntax-highlighted diffs, and MCP management; Computer Use and SubAgent visual workflows; mobile session doesn't disconnect on lock screen.
- Cons: Non-OSI source-available license (commercial resale restricted); desktop application must remain open to serve the H5 mobile connection; interface is primarily Chinese with ongoing English translation.
- Sources: https://github.com/NanmiCoder/cc-haha, https://cchaha.ai
- Confidence: high

### Claude Code Telegram Bot (overwirehq/claude-code-telegram)
- URL: https://github.com/overwirehq/claude-code-telegram
- Kind: agent-native
- What: A dedicated, lightweight Python-based Telegram bot providing remote mobile access to Anthropic's Claude Code CLI. Developers can chat naturally with Claude about codebases, maintain persistent project sessions, trigger coding tasks, and receive push notifications from webhooks and CI/CD pipelines while on mobile.
- Agent integration: Claude Code CLI subprocess execution; session state persistence; directory sandboxing; webhook event listener.
- Mobile: Mobile access through official Telegram apps on iOS and Android (https://github.com/overwirehq/claude-code-telegram).
- Pricing: Free and open-source (MIT).
- Self-host: yes (installed via `uv tool install` or `pip install`, runs as a local background system service).
- License: MIT
- Activity: 2,815 GitHub stars, pushed October 1, 2026 (v1.3.0); active maintenance.
- Pros: Very lightweight Python setup with `uv`; zero mobile client configuration required beyond Telegram; persistent project context across conversations; directory sandboxing prevents accidental directory escape; native Telegram push notifications on task completion.
- Cons: Limited to Telegram (no Discord or standalone web UI); lacks a graphical Kanban or diff viewer; interactive command prompts can be cumbersome for large code reviews in mobile chat.
- Sources: https://github.com/overwirehq/claude-code-telegram
- Confidence: high

### Ductor
- URL: https://ductor.dev
- Kind: agent-native
- What: A modular multi-transport gateway that controls official coding agent CLIs (Claude Code, OpenAI Codex CLI, Google Gemini CLI, Antigravity CLI, Grok Build) via Telegram, Matrix, and Slack. It executes official CLIs as subprocesses without SDK patching or API proxying, offering Docker sandboxing, cron jobs, and mobile status updates.
- Agent integration: Claude Code, Codex CLI, Gemini CLI, Antigravity CLI, Grok Build; executes official binaries directly; plain JSON/Markdown state stored under `~/.ductor/`.
- Mobile: Mobile access via Telegram, Matrix, and Slack apps on iOS and Android (https://ductor.dev).
- Pricing: Free and open-source (MIT; PyPI package `ductor`).
- Self-host: yes (Python CLI installed via `pip install ductor`, runs locally or in Docker).
- License: MIT
- Activity: 460 GitHub stars, pushed July 23, 2026; PyPI releases active.
- Pros: True multi-CLI support that uses official binaries directly so users leverage existing CLI subscriptions (Claude Max, Gemini Ultra) without extra API markup; support for Matrix and Slack alongside Telegram; optional Docker container sandboxing for safety; transparent filesystem state in JSON/Markdown.
- Cons: Output in chat apps can be noisy during long agent thinking/tool execution turns; lacks a visual task board interface; setup requires configuring bot credentials for chosen messaging networks.
- Sources: https://github.com/PleasePrompto/ductor, https://ductor.dev
- Confidence: high

### ccpoke
- URL: https://kaida-palooza.github.io/ccpoke/
- Kind: agent-native
- What: A lightweight two-way mobile notification and chat bridge connecting AI coding agents (Claude Code, Codex CLI, Cursor CLI) to Telegram. It leverages agent stop hooks to deliver instant mobile notifications when an agent finishes or pauses, and uses tmux to let users reply, approve permissions, and manage multiple agent sessions from their phone.
- Agent integration: Claude Code, Codex CLI, Cursor CLI; uses Stop Hooks and tmux session multiplexing; plugin architecture for new agents.
- Mobile: Mobile interaction via Telegram app on iOS and Android (https://kaida-palooza.github.io/ccpoke/).
- Pricing: Free and open-source (MIT; npm package `ccpoke`).
- Self-host: yes (Node.js tool installed via `npm i -g ccpoke` or executed via `npx -y ccpoke`).
- License: MIT
- Activity: 103 GitHub stars, pushed March 2026.
- Pros: Zero-install execution via `npx ccpoke` with guided CLI onboarding wizard; event-driven stop hooks eliminate polling and battery drain; tmux integration enables real two-way session interaction and permission grants from phone; multi-session switching.
- Cons: Requires tmux on the host system (automatically installed on some OSes, but complicates Windows setup); Telegram-only mobile interface; no visual web dashboard or Kanban board.
- Sources: https://github.com/kaida-palooza/ccpoke
- Confidence: high

### Agent4Discord
- URL: https://github.com/raravel/Agent4Discord
- Kind: agent-native
- What: A self-hosted Discord bot specifically crafted for managing and interacting with Claude Code sessions remotely. It maps each coding session to a dedicated Discord channel, renders tool calls in collapsible threads, streams thinking and output via live-updating embeds, and presents interactive buttons for permission decisions.
- Agent integration: Claude Code CLI; process streaming; interactive Discord components (buttons, select menus, modal forms).
- Mobile: Mobile control via the official Discord mobile app on iOS and Android (https://github.com/raravel/Agent4Discord).
- Pricing: Free and open-source (MIT).
- Self-host: yes (TypeScript/Node.js bot running locally on developer PC with custom Discord bot token).
- License: MIT
- Activity: Pushed October 1, 2026; recent active release with bilingual English/Korean documentation.
- Pros: Native Discord mobile integration with rich UI: interactive Allow/Always Allow/Deny buttons for tool approvals; thread-per-tool-call keeps channels clean and readable on small screens; filesystem directory browser with dropdown menus; token and cost tracking in dedicated `#a4d-usage` channel.
- Cons: Dedicated exclusively to Claude Code (does not support Codex or Gemini CLI yet); requires setting up a Discord bot and developer portal application; Discord message character limits (2,000 chars) require message chunking.
- Sources: https://github.com/raravel/Agent4Discord
- Confidence: high

### OpenHands
- URL: https://openhands.dev
- Kind: agent-native
- What: An open-source autonomous AI software development platform capable of writing code, running bash commands, browsing the web, and resolving GitHub issues. It provides a mobile-responsive web dashboard and an official Slack bot integration (`OpenHands for Slack`) that lets engineers dispatch and monitor autonomous coding tasks from their phones.
- Agent integration: Docker sandboxed runtime, REST API, WebSocket event stream, GitHub App, Slack bot app, MCP tools support.
- Mobile: Responsive web application for mobile browsers, plus official Slack bot integration for mobile task initiation, progress updates, and PR approvals (https://openhands.dev).
- Pricing: Free and open-source for self-hosting (MIT); OpenHands Cloud offers free tier with community usage and paid plans based on compute/agent usage (pricing as of October 2026).
- Self-host: yes (Docker one-liner `docker run -it -p 3000:3000 ghcr.io/all-hands-ai/openhands:latest`, Kubernetes Helm charts).
- License: MIT
- Activity: 91,014 GitHub stars, pushed October 16, 2026; top-tier OSS community with hundreds of contributors.
- Pros: Comprehensive autonomous capabilities (editing, terminal execution, browser browsing, test running); Slack bot allows triggering tasks directly from phone with conversational status updates; Docker sandbox isolates code execution; large active ecosystem with extensive LLM provider support.
- Cons: Resource-heavy self-hosted Docker footprint; mobile web UI is dense with multi-tab workspace layouts; Slack bot features are streamlined compared to the full desktop web dashboard.
- Sources: https://github.com/All-Hands-AI/OpenHands, https://openhands.dev, https://www.all-hands.dev/pricing
- Confidence: high

### Devin (Devin for Slack / Mobile Web)
- URL: https://devin.ai
- Kind: agent-native
- What: Cognition's flagship autonomous AI software engineer that plans, executes, debugs, and submits code changes independently. While primarily operated via a desktop web workspace, mobile users can monitor and steer Devin through its mobile-optimized web viewer and official Devin for Slack bot integration.
- Agent integration: Deep cloud-hosted sandboxed environments; GitHub/GitLab integrations; Slack bot integration; REST API and webhooks.
- Mobile: Mobile-responsive web dashboard (session viewing, log inspection) and mobile Slack app (`@Devin`) for initiating tasks, receiving milestone threads, and approving execution steps (https://devin.ai).
- Pricing: Proprietary commercial service; developer and enterprise subscription plans (team plans start around $500/month; enterprise custom pricing).
- Self-host: no (closed-source cloud platform).
- License: Proprietary
- Activity: Leading commercial agent; continuous product updates and integrations in 2025-2026.
- Pros: Industry-leading benchmark performance on SWE-bench and real-world repository tasks; Slack integration provides native mobile notifications, progress thread updates, and mobile approvals; fully sandboxed remote cloud environment requires zero local hardware; automated PR drafting with unit tests.
- Cons: High price point out of reach for individual hobbyists; fully closed-source proprietary platform; no standalone native iOS/Android application; relies on cloud hosting with no on-premise self-host option.
- Sources: https://devin.ai, https://cognition.ai
- Confidence: high

### Routa
- URL: https://phodal.github.io/routa/
- Kind: agent-native
- What: An open-source, workspace-first multi-agent coordination platform for software development featuring shared specifications, Kanban orchestration, and MCP/ACP/A2A (Agent-to-Agent) communication across web and desktop. Its web interface provides mobile-responsive Kanban boards for tracking tasks assigned to various coding agents.
- Agent integration: Model Context Protocol (MCP), Agent Client Protocol (ACP), Agent-to-Agent (A2A) protocol; REST and WebSocket APIs; multi-model execution.
- Mobile: Responsive web dashboard accessible via mobile browsers on iOS and Android (https://phodal.github.io/routa/).
- Pricing: Free and open-source (MIT).
- Self-host: yes (Docker, local Node.js web server).
- License: MIT
- Activity: 1,828 GitHub stars, pushed October 14, 2026; active commit velocity.
- Pros: True agent-native Kanban board displaying task status (Todo, In Progress, Review, Done) as agents transition; standardized protocol support (MCP, ACP, A2A) ensures interoperability with modern agent standards; multi-agent collaboration with shared project specs; 100% free open-source.
- Cons: Lacks a dedicated native mobile app or push notification service; mobile experience requires using a mobile web browser with manual screen scrolling; smaller user community compared to mainstream tools.
- Sources: https://github.com/phodal/routa, https://phodal.github.io/routa/
- Confidence: high

## EXPAND
- LEAD: Omnara (YC S25) — WHY: Open-source managed agent platform positioning as an alternative to Claude Managed Agents, actively building out mobile PWA management endpoints — ANGLE: gh api repos/omnara-ai/omnara and docs.omnara.com mobile roadmap
- LEAD: Blink Shell & Termius Native AI Integration — WHY: Mobile iOS terminal apps adding native AI agent command runners and session monitors directly into their mobile PTY engines — ANGLE: blink shell ai agent mobile ios
- LEAD: Native Push Notification Daemon Standards for Agent Stop Hooks — WHY: Emerging open standards (such as Web Push, ntfy, and Bark) being integrated directly into agent config files (e.g., hooks.json) without third-party bot intermediaries — ANGLE: claude code hooks ntfy push notification bark
